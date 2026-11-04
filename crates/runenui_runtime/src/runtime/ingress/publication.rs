use core::mem;
use std::collections::HashMap;

use runenui_core::{
    CommandOrigin, MonotonicInstant, ProducerBudget, SemanticCommand, SemanticRelationshipKind,
    WorkFamily, WorkResidencyBudget, WorkStartBudget,
};

use super::{
    HostProtocol, MandatoryTracePlan, QueueCommitError, Runtime, RuntimeStatus,
    RuntimeTerminalReason, TraceRecordKind, TraceSequence,
};
use crate::runtime::surface_publication::{
    RedrawRevisionAdmission, SurfacePublicationAdmission, SurfacePublicationCandidateInputs,
    SurfacePublicationPlanError,
};
use crate::{
    PublishSurfaceError, SurfacePublicationCounter, TracePublicationContext, TraceSurfaceContext,
    TraceSurfaceSnapshotKind, TraceTarget,
    trace::{StagedMotionTraceFact, TraceRecordDraft, TraceReservation},
};

struct PublicationAdmission {
    surface: SurfacePublicationAdmission,
    stationary_rehit: bool,
}

struct SurfacePublicationCompletion<'a> {
    publication: &'a crate::SurfacePublication,
    previous_semantics: Option<&'a crate::SemanticPublication>,
    published: Option<TraceSequence>,
    stationary_rehit: bool,
    producer_root: Option<crate::queue::RootQuotaId>,
    instant: MonotonicInstant,
    redraw: Option<crate::RedrawRequest>,
    followup_redraw: Option<RedrawRevisionAdmission>,
}

struct SurfacePublicationCandidateState {
    previous_semantics: Option<crate::SemanticPublication>,
    focused_owner: Option<crate::MountedNodeId>,
    interaction: crate::surface::SurfaceInteractionProjection,
    editing: HashMap<crate::MountedNodeId, crate::editing::EditingSemanticProjection>,
    sensitivities: HashMap<crate::MountedNodeId, runenui_core::TextSensitivity>,
    preedits: HashMap<crate::MountedNodeId, std::sync::Arc<runenui_text::TextPreeditProjection>>,
    instant: MonotonicInstant,
}

impl SurfacePublicationCandidateState {
    const fn inputs(
        &self,
        admission: SurfacePublicationAdmission,
    ) -> SurfacePublicationCandidateInputs<'_> {
        SurfacePublicationCandidateInputs::new(
            &self.interaction,
            self.focused_owner.as_ref(),
            &self.editing,
            &self.sensitivities,
            &self.preedits,
            admission,
            self.instant,
        )
    }
}

fn publication_reveal_candidates<'a>(
    current: &'a crate::SemanticPublication,
    previous: Option<&'a crate::SemanticPublication>,
) -> impl Iterator<Item = &'a crate::SemanticNode> + 'a {
    current.snapshot().nodes().iter().filter(move |node| {
        if node.state().expanded() != Some(true) || node.state().disabled() || node.state().inert()
        {
            return false;
        }
        let active = node
            .relationships()
            .iter()
            .find(|relation| relation.kind() == SemanticRelationshipKind::ActiveDescendant)
            .map(crate::SemanticRelationship::target);
        active.is_some_and(|active| {
            !previous.is_some_and(|previous| {
                let old = previous.snapshot();
                old.node(node.id())
                    .is_some_and(|node| node.state().expanded() == Some(true))
                    && old.node(active).is_some()
            })
        })
    })
}

/// Composes simultaneous descendant frontiers, rather than giving each derived
/// request a new root that could compete after publication has committed.
fn publication_producer_budget(
    interaction: Option<ProducerBudget>,
    derived_turns: usize,
    rehit: bool,
    input_method: bool,
) -> Option<ProducerBudget> {
    let child = if derived_turns == 0 {
        ProducerBudget::new(0, 0)
    } else {
        let child = interaction?;
        if child.queue_credits() == 0 {
            return None;
        }
        child
    };
    let service = usize::from(input_method);
    // One runtime IME request has a start and independent response/cancel
    // credits. Re-hit itself has one sequenced envelope in addition to the
    // per-pointer routed descendants included in derived_turns.
    let credits = child
        .queue_credits()
        .checked_mul(derived_turns)?
        .checked_add(service.checked_mul(3)?)?
        .checked_add(usize::from(rehit))?;
    let residency = child.work_residency();
    let scaled = |family| residency.for_family(family).checked_mul(derived_turns);
    Some(
        ProducerBudget::new(credits, child.outputs().max(service))
            .with_application_outputs(child.application_outputs())
            .with_reconciliation(
                child.mounted_nodes(),
                child.retained_updates(),
                child.dirty_subscription_owners(),
            )
            .with_subscriptions(
                child.application_subscriptions(),
                child.mounted_subscriptions_per_owner(),
            )
            .with_work_starts(WorkStartBudget::new(
                child.work_starts().for_family(WorkFamily::LocalTask),
                child.work_starts().for_family(WorkFamily::SendTask),
                child.work_starts().for_family(WorkFamily::Timer),
                child.work_starts().for_family(WorkFamily::HostRequest),
                child
                    .work_starts()
                    .for_family(WorkFamily::FrameworkService)
                    .max(service),
            ))
            .with_work_residency(WorkResidencyBudget::new(
                scaled(WorkFamily::LocalTask)?,
                scaled(WorkFamily::SendTask)?,
                scaled(WorkFamily::Timer)?,
                residency.subscriptions().checked_mul(derived_turns)?,
                scaled(WorkFamily::HostRequest)?,
                scaled(WorkFamily::FrameworkService)?.checked_add(service)?,
            )),
    )
}

fn candidate_trace_plan(
    redraw_pending: bool,
    stationary_rehit: bool,
    request_followup: bool,
    motion_trace_count: usize,
) -> Option<MandatoryTracePlan> {
    let motion = MandatoryTracePlan::one_fact().checked_mul(motion_trace_count)?;
    let plan = MandatoryTracePlan::surface_publication(redraw_pending, stationary_rehit)
        .checked_add(motion)?;
    if request_followup {
        plan.checked_add(MandatoryTracePlan::one_fact())
    } else {
        Some(plan)
    }
}

impl<State, Action, Protocol: HostProtocol> Runtime<State, Action, Protocol> {
    pub(crate) fn request_redraw(
        &mut self,
        causal_parent: Option<TraceSequence>,
        instant: MonotonicInstant,
    ) {
        if !self.trace.can_admit(MandatoryTracePlan::one_fact()) {
            self.enter_terminal(RuntimeTerminalReason::TraceSequenceExhausted, 0);
            return;
        }
        let Some(next) = self.surface_publication.request_redraw() else {
            self.enter_terminal(
                RuntimeTerminalReason::SurfacePublicationCounterExhausted(
                    SurfacePublicationCounter::RedrawRevision,
                ),
                0,
            );
            return;
        };
        let requested = if self.trace.is_enabled() {
            self.trace.record_draft(
                TraceRecordDraft::redraw_fact(
                    TraceRecordKind::RedrawRequested { revision: next },
                    instant,
                )
                .with_causal_parent(causal_parent),
            )
        } else {
            None
        };
        if self.trace.is_enabled() && requested.is_none() {
            self.enter_terminal(RuntimeTerminalReason::TraceSequenceExhausted, 0);
            return;
        }
        self.surface_trace.note_request(next, requested);
    }

    fn commit_admitted_redraw_request(
        &mut self,
        admission: RedrawRevisionAdmission,
        causal_parent: Option<TraceSequence>,
        instant: MonotonicInstant,
    ) {
        let revision = self.surface_publication.commit_redraw_request(admission);
        let requested = if self.trace.is_enabled() {
            Some(
                self.trace
                    .record_draft(
                        TraceRecordDraft::redraw_fact(
                            TraceRecordKind::RedrawRequested { revision },
                            instant,
                        )
                        .with_causal_parent(causal_parent),
                    )
                    .unwrap_or_else(|| {
                        unreachable!("surface publication trace plan admitted follow-up redraw")
                    }),
            )
        } else {
            None
        };
        self.surface_trace.note_request(revision, requested);
    }

    pub(crate) fn take_redraw_request(&mut self) -> Option<crate::RedrawRequest> {
        let instant = self.now();
        self.take_redraw_request_at(instant)
    }

    fn take_redraw_request_at(
        &mut self,
        instant: MonotonicInstant,
    ) -> Option<crate::RedrawRequest> {
        let mut request = self.surface_publication.take_redraw_request()?;
        let request_parent = self.surface_trace.request_parent(request.revision());
        request.bind_request_trace(request_parent);
        let taken = if self.trace.is_enabled() {
            self.trace.record_draft(
                TraceRecordDraft::redraw_fact(
                    TraceRecordKind::RedrawTaken {
                        revision: request.revision(),
                    },
                    instant,
                )
                .with_causal_parent(request_parent),
            )
        } else {
            None
        };
        request.bind_taken_trace(taken);
        Some(request)
    }

    pub(crate) fn acknowledge_redraw(
        &mut self,
        request: &crate::RedrawRequest,
    ) -> Result<(), crate::RedrawAcknowledgeError> {
        let instant = self.now();
        self.acknowledge_redraw_at(request, request.control_parent(), instant)
    }

    fn acknowledge_redraw_at(
        &mut self,
        request: &crate::RedrawRequest,
        causal_parent: Option<TraceSequence>,
        instant: MonotonicInstant,
    ) -> Result<(), crate::RedrawAcknowledgeError> {
        self.surface_publication.acknowledge_redraw(request)?;
        if self.trace.is_enabled() {
            self.trace.record_draft(
                TraceRecordDraft::redraw_fact(
                    TraceRecordKind::RedrawAcknowledged {
                        revision: request.revision(),
                    },
                    instant,
                )
                .with_causal_parent(causal_parent),
            );
        }
        self.surface_trace
            .clear_if_acknowledged(request.revision(), self.surface_publication.is_dirty());
        Ok(())
    }

    #[allow(
        clippy::too_many_lines,
        reason = "surface publication keeps staged admission and the final infallible commit in one linear transaction so mutation order remains directly auditable"
    )]
    pub(crate) fn publish_surface(
        &mut self,
        context: &crate::SurfaceBuildContext<'_>,
    ) -> Result<crate::SurfacePublication, PublishSurfaceError> {
        // First preserve established surface/queue/trace rejection precedence:
        // admission only checks reservations and does not publish input state.
        let admission = self.admit_surface_publication()?;
        if !self.can_admit_direct_input_ownership_boundary() {
            let reason = RuntimeTerminalReason::Poisoned;
            self.enter_terminal(reason, 0);
            return Err(PublishSurfaceError::Terminal(reason));
        }
        // Preserve accepted #537 observation admission before any retained
        // mutation or producer reservation. Pending direct boundaries stay gaps.
        let direct_input_observation = self
            .reserve_direct_publication_observation()
            .map_err(PublishSurfaceError::InputObservation)?;
        let presentation_commands = self
            .publication_presentation_command_bound()
            .ok_or(PublishSurfaceError::InvalidProducerProfile)?;
        let reveal_bound = self
            .tree
            .publication_reveal_bound()
            .ok_or(PublishSurfaceError::InvalidProducerProfile)?;
        let rehit_pointers = self
            .pointer_registry
            .publication_pointer_count(self.queue.queued_pointer_ids());
        let input_method = self.publication_input_method_may_be_staged();
        let derived_turns = reveal_bound
            .checked_add(presentation_commands)
            .and_then(|count| count.checked_add(rehit_pointers))
            .ok_or(PublishSurfaceError::InvalidProducerProfile)?;
        let interaction_budget = if derived_turns == 0 {
            None
        } else {
            self.interaction_producer_budget
                .map(|profile| profile(self.state()))
        };
        let producer = publication_producer_budget(
            interaction_budget,
            derived_turns,
            admission.stationary_rehit,
            input_method,
        )
        .ok_or(PublishSurfaceError::InvalidProducerProfile)?;
        let producer_reservation = if producer.queue_credits() == 0 {
            None
        } else {
            if !self.application_output_budget_fits(producer) {
                return Err(PublishSurfaceError::InvalidProducerProfile);
            }
            // Every knowable resource refusal precedes publication capability
            // callbacks. Candidate shape must fit the declaration afterwards.
            self.queue
                .preflight_reservation(producer.queue_credits())
                .map_err(|error| self.publication_reservation_error(error))?;
            self.work
                .preflight_producer_residency(&self.queue, producer)
                .map_err(PublishSurfaceError::WorkCapacity)?;
            if input_method && self.work.preview_generations(1).is_err() {
                let reason = RuntimeTerminalReason::WorkGenerationExhausted;
                self.enter_terminal(reason, 0);
                return Err(PublishSurfaceError::Terminal(reason));
            }
            Some(
                self.reserve_producer(producer)
                    .map_err(|error| self.publication_reservation_error(error))?,
            )
        };
        let producer_root = producer_reservation
            .as_ref()
            .map(crate::queue::QueueReservation::quota);
        let result = self.publish_surface_admitted(context, admission, reveal_bound, producer_root);
        if matches!(self.status, RuntimeStatus::Running)
            && let Some(reservation) = producer_reservation.as_ref()
            && self.queue.finish_reservation(reservation).is_err()
        {
            let reason = RuntimeTerminalReason::Poisoned;
            self.enter_terminal(reason, 0);
            return Err(PublishSurfaceError::Terminal(reason));
        }
        let mut publication = result?;
        self.retire_input_continuation_claims();
        if let RuntimeStatus::Terminal(reason) = self.status {
            return Err(PublishSurfaceError::Terminal(reason));
        }
        if let Some((before_revision, reservation)) = direct_input_observation {
            let (after, transition) = self.input_ownership_reserved(reservation);
            let exact_single_step = before_revision
                .get()
                .checked_add(1)
                .is_some_and(|expected| after.revision().get() == expected);
            let exact_publication_context = after.surfaces().iter().any(|surface| {
                surface.surface_id() == publication.input_context().surface_id()
                    && surface.latest_retained_context() == Some(publication.input_context())
            });
            if exact_single_step
                && after.status() == RuntimeStatus::Running
                && exact_publication_context
            {
                publication =
                    publication.with_input_ownership_transition(crate::InputOwnershipTransition {
                        before_revision,
                        after: transition,
                    });
            }
        }
        Ok(publication)
    }

    fn publish_surface_admitted(
        &mut self,
        context: &crate::SurfaceBuildContext<'_>,
        admission: PublicationAdmission,
        reveal_bound: usize,
        producer_root: Option<crate::queue::RootQuotaId>,
    ) -> Result<crate::SurfacePublication, PublishSurfaceError> {
        // Retain a cheap immutable handle to the previous committed semantic
        // publication. Exact lookup is O(1) per candidate, without cloning
        // every semantic node or building an unbounded mirror of its IDs.
        let candidate_state = self.surface_publication_candidate_state();
        let stationary_rehit = admission.stationary_rehit;
        let candidate = candidate_state.inputs(admission.surface);
        let staged = self.surface_publication.plan_publication(
            &mut self.tree,
            &mut self.text_system,
            context,
            candidate,
        );
        let staged = match staged {
            Ok(staged) => staged,
            Err(error) => {
                return Err(self.surface_publication_plan_error(error, candidate_state.instant)?);
            }
        };
        let previous_semantics = candidate_state.previous_semantics;
        let instant = candidate_state.instant;
        let motion_trace_facts = staged.motion_trace_facts();
        let motion_activity = staged.motion_activity();
        let request_followup =
            motion_activity.continuous_redraw() || motion_activity.followup_publication();
        let Some(candidate_trace_plan) = candidate_trace_plan(
            self.surface_publication.is_dirty(),
            stationary_rehit,
            request_followup,
            motion_trace_facts.len(),
        ) else {
            drop(staged);
            let reason = RuntimeTerminalReason::TraceSequenceExhausted;
            self.enter_terminal(reason, 0);
            return Err(PublishSurfaceError::Terminal(reason));
        };
        if !self.trace.can_replace_reservation(
            self.surface_trace.publication_reservation,
            candidate_trace_plan,
        ) {
            drop(staged);
            let reason = RuntimeTerminalReason::TraceSequenceExhausted;
            self.enter_terminal(reason, 0);
            return Err(PublishSurfaceError::Terminal(reason));
        }
        let followup_redraw = request_followup
            .then(|| self.surface_publication.admit_redraw_request())
            .transpose();
        let followup_redraw = match followup_redraw {
            Ok(admission) => admission,
            Err(counter) => {
                drop(staged);
                let reason = RuntimeTerminalReason::SurfacePublicationCounterExhausted(counter);
                self.enter_terminal(reason, 0);
                return Err(PublishSurfaceError::Terminal(reason));
            }
        };

        if publication_reveal_candidates(staged.semantic_publication(), previous_semantics.as_ref())
            .count()
            > reveal_bound
        {
            drop(staged);
            let reason = RuntimeTerminalReason::Poisoned;
            self.enter_terminal(reason, 0);
            return Err(PublishSurfaceError::Terminal(reason));
        }
        let commit = staged.commit_store();
        let publication = self
            .surface_publication
            .commit_publication(&mut self.tree, commit);
        self.note_direct_input_ownership_boundary();
        self.cancel_stale_framework_services();
        self.record_motion_trace_facts(motion_trace_facts, instant);
        let redraw = self.take_redraw_request_at(instant);
        let publication_reservation = mem::replace(
            &mut self.surface_trace.publication_reservation,
            TraceReservation::continuation(),
        );
        let published = self.record_surface_publication(
            publication_reservation,
            redraw
                .as_ref()
                .and_then(crate::RedrawRequest::request_parent),
            instant,
            &publication,
        );
        self.finish_admitted_surface_publication(SurfacePublicationCompletion {
            publication: &publication,
            previous_semantics: previous_semantics.as_ref(),
            published,
            stationary_rehit,
            producer_root,
            instant,
            redraw,
            followup_redraw,
        })?;
        Ok(publication)
    }

    fn surface_publication_candidate_state(&self) -> SurfacePublicationCandidateState {
        let instant = self.now();
        let focused_owner = self.focus.focused_node().cloned();
        let interaction = self
            .pointer_registry
            .surface_interaction_projection(focused_owner.as_ref())
            .with_focus_visible(self.focus.focus_visible());
        SurfacePublicationCandidateState {
            previous_semantics: self
                .surface_publication
                .current_semantic_publication()
                .cloned(),
            focused_owner,
            interaction,
            editing: self.editing.semantic_projections(),
            sensitivities: self.editing.sensitivity_projections(),
            preedits: self.editing.preedit_projections(),
            instant,
        }
    }

    fn surface_publication_plan_error(
        &mut self,
        error: SurfacePublicationPlanError,
        instant: MonotonicInstant,
    ) -> Result<PublishSurfaceError, PublishSurfaceError> {
        let error = match error {
            SurfacePublicationPlanError::SemanticIntegrity => {
                let reason = RuntimeTerminalReason::Poisoned;
                self.enter_terminal(reason, 0);
                PublishSurfaceError::Terminal(reason)
            }
            SurfacePublicationPlanError::TextLayout(error) => {
                PublishSurfaceError::TextLayout(error)
            }
            SurfacePublicationPlanError::PresentationGeometry => {
                PublishSurfaceError::PresentationGeometry
            }
            SurfacePublicationPlanError::Motion(failure) => {
                self.record_motion_planning_rejection(&failure, instant)?;
                PublishSurfaceError::Motion
            }
            SurfacePublicationPlanError::CounterExhausted(counter) => {
                let reason = RuntimeTerminalReason::SurfacePublicationCounterExhausted(counter);
                self.enter_terminal(reason, 0);
                PublishSurfaceError::Terminal(reason)
            }
        };
        Ok(error)
    }

    fn finish_admitted_surface_publication(
        &mut self,
        completion: SurfacePublicationCompletion<'_>,
    ) -> Result<(), PublishSurfaceError> {
        let SurfacePublicationCompletion {
            publication,
            previous_semantics,
            published,
            stationary_rehit,
            producer_root,
            instant,
            redraw,
            followup_redraw,
        } = completion;
        self.synchronize_presentation_lifecycle_after_publication(published, producer_root);
        if let RuntimeStatus::Terminal(reason) = self.status {
            return Err(PublishSurfaceError::Terminal(reason));
        }
        // The geometry and semantic identities are now atomically retained.
        // Route one exact mounted-generation reveal through the existing M4/#344
        // command queue, never through a second scroll or focus controller.
        self.reveal_newly_published_active_descendants(
            publication,
            previous_semantics,
            published,
            producer_root,
        );
        if let RuntimeStatus::Terminal(reason) = self.status {
            return Err(PublishSurfaceError::Terminal(reason));
        }
        if stationary_rehit {
            self.commit_stationary_pointer_rehit(publication, instant, published, producer_root);
        }
        if let RuntimeStatus::Terminal(reason) = self.status {
            return Err(PublishSurfaceError::Terminal(reason));
        }
        self.synchronize_input_method_after_publication(published, instant, producer_root);
        if let RuntimeStatus::Terminal(reason) = self.status {
            return Err(PublishSurfaceError::Terminal(reason));
        }
        if let Some(redraw) = redraw {
            self.acknowledge_redraw_at(&redraw, published, instant)
                .unwrap_or_else(|_| unreachable!("runtime-issued redraw request remains local"));
        }
        self.replenish_surface_publication_reservation();
        if let Some(admission) = followup_redraw {
            self.commit_admitted_redraw_request(admission, published, instant);
        }
        Ok(())
    }

    fn publication_reservation_error(
        &mut self,
        error: crate::queue::QueueReservationError,
    ) -> PublishSurfaceError {
        use crate::queue::QueueReservationError;
        match error {
            QueueReservationError::Capacity => PublishSurfaceError::Full,
            QueueReservationError::WorkCapacity(family) => {
                PublishSurfaceError::WorkCapacity(family)
            }
            QueueReservationError::Closed => PublishSurfaceError::Closed,
            QueueReservationError::SequenceExhausted => {
                let reason = RuntimeTerminalReason::WorkSequenceExhausted;
                self.enter_terminal(reason, 0);
                PublishSurfaceError::Terminal(reason)
            }
            QueueReservationError::Foreign
            | QueueReservationError::Stale
            | QueueReservationError::Exhausted => {
                let reason = RuntimeTerminalReason::Poisoned;
                self.enter_terminal(reason, 0);
                PublishSurfaceError::Terminal(reason)
            }
        }
    }

    fn reveal_newly_published_active_descendants(
        &mut self,
        publication: &crate::SurfacePublication,
        previous: Option<&crate::SemanticPublication>,
        causal_parent: Option<TraceSequence>,
        root_quota: Option<crate::queue::RootQuotaId>,
    ) {
        let snapshot = publication.semantic_publication().snapshot();
        // Stream current semantic owners rather than materializing a second
        // unbounded list of candidates. Admission remains with the canonical
        // #344 command FIFO, even for very large semantic publications.
        for combo in publication_reveal_candidates(publication.semantic_publication(), previous) {
            let target = (|| {
                // The semantic compositor validates these generic relationships;
                // demand exactly one of each and current mounted ancestry. No
                // built-in control role or widget type is special to this path.
                let relation = |kind| {
                    let mut matches = combo
                        .relationships()
                        .iter()
                        .filter(|relationship| relationship.kind() == kind);
                    let first = matches.next()?;
                    matches.next().is_none().then_some(first.target())
                };
                let list_id = relation(SemanticRelationshipKind::Controls)?;
                let active_id = relation(SemanticRelationshipKind::ActiveDescendant)?;
                // Both relationship targets must exist in this committed snapshot.
                snapshot.node(list_id)?;
                let option = snapshot.node(active_id)?;
                if option.state().disabled() || option.state().inert() {
                    return None;
                }
                let combo_owner = self.tree.semantic_action_authority(combo.id()).ok()?;
                let list_owner = self.tree.semantic_action_authority(list_id).ok()?;
                let option_owner = self.tree.semantic_action_authority(active_id).ok()?;
                if !combo_owner.activation().enabled()
                    || !list_owner.activation().enabled()
                    || !option_owner.activation().enabled()
                    || !self
                        .tree
                        .is_descendant_or_self(list_owner.owner(), combo_owner.owner())
                    || !self
                        .tree
                        .is_descendant_or_self(option_owner.owner(), list_owner.owner())
                {
                    return None;
                }
                Some(option_owner.owner().clone())
            })();
            let Some(target) = target else {
                continue;
            };
            // Capacity rejection leaves the retained publication intact and
            // cannot redirect the reveal to another Option or move real focus.
            // Counter/sequence exhaustion uses the normal terminal protocol.
            if self
                .submit_command_with_parent(
                    target,
                    SemanticCommand::ScrollIntoView,
                    CommandOrigin::programmatic(),
                    causal_parent,
                    root_quota,
                )
                .is_err()
                && !matches!(self.status, RuntimeStatus::Running)
            {
                break;
            }
        }
    }

    fn record_motion_trace_facts(
        &mut self,
        facts: Vec<StagedMotionTraceFact>,
        instant: MonotonicInstant,
    ) {
        if !self.trace.is_enabled() {
            return;
        }
        for staged in facts {
            self.trace
                .record_draft(
                    TraceRecordDraft::lifecycle_fact(
                        TraceRecordKind::Motion {
                            target: staged.target,
                            fact: staged.fact,
                        },
                        instant,
                    )
                    .with_target(Some(TraceTarget::new(staged.owner, staged.authored_id))),
                )
                .unwrap_or_else(|| {
                    unreachable!("candidate-dependent motion trace admission was preflighted")
                });
        }
    }

    fn record_motion_planning_rejection(
        &mut self,
        failure: &crate::surface::MotionPlanningFailure,
        instant: MonotonicInstant,
    ) -> Result<(), PublishSurfaceError> {
        if !self.trace.is_enabled() {
            return Ok(());
        }
        if !self.trace.can_admit(MandatoryTracePlan::one_fact()) {
            let reason = RuntimeTerminalReason::TraceSequenceExhausted;
            self.enter_terminal(reason, 0);
            return Err(PublishSurfaceError::Terminal(reason));
        }
        self.trace
            .record_draft(
                TraceRecordDraft::lifecycle_fact(
                    TraceRecordKind::Motion {
                        target: failure.target,
                        fact: failure.fact.clone(),
                    },
                    instant,
                )
                .with_target(Some(TraceTarget::new(
                    failure.owner.clone(),
                    failure.authored_id.clone(),
                ))),
            )
            .unwrap_or_else(|| {
                unreachable!("motion planning rejection trace admission was preflighted")
            });
        Ok(())
    }

    fn admit_surface_publication(&mut self) -> Result<PublicationAdmission, PublishSurfaceError> {
        match self.status {
            RuntimeStatus::Running => {}
            RuntimeStatus::Terminal(reason) => return Err(PublishSurfaceError::Terminal(reason)),
            RuntimeStatus::Closed => return Err(PublishSurfaceError::Closed),
        }

        let surface = self
            .surface_publication
            .admit_publication()
            .map_err(|counter| {
                let reason = RuntimeTerminalReason::SurfacePublicationCounterExhausted(counter);
                self.enter_terminal(reason, 0);
                PublishSurfaceError::Terminal(reason)
            })?;

        let stationary_rehit =
            self.pointer_registry.has_streams() || self.queue.has_pointer_envelopes();
        if stationary_rehit {
            match self.queue.preflight_commit(1) {
                Ok(()) => {}
                Err(QueueCommitError::Full) => return Err(PublishSurfaceError::Full),
                Err(QueueCommitError::SequenceExhausted) => {
                    let reason = RuntimeTerminalReason::WorkSequenceExhausted;
                    self.enter_terminal(reason, 0);
                    return Err(PublishSurfaceError::Terminal(reason));
                }
            }
        }

        let trace_plan = MandatoryTracePlan::surface_publication(
            self.surface_publication.is_dirty(),
            stationary_rehit,
        );
        if !self
            .trace
            .can_replace_reservation(self.surface_trace.publication_reservation, trace_plan)
        {
            let reason = RuntimeTerminalReason::TraceSequenceExhausted;
            self.enter_terminal(reason, 0);
            return Err(PublishSurfaceError::Terminal(reason));
        }

        Ok(PublicationAdmission {
            surface,
            stationary_rehit,
        })
    }

    fn commit_stationary_pointer_rehit(
        &mut self,
        publication: &crate::SurfacePublication,
        instant: MonotonicInstant,
        published: Option<TraceSequence>,
        root_quota: Option<crate::queue::RootQuotaId>,
    ) {
        let input_context = publication.input_context();
        let work_sequence = self
            .queue
            .next_sequence()
            .unwrap_or_else(|| unreachable!("stationary re-hit work sequence was preflighted"));
        let causal_parent = if self.trace.is_enabled() {
            self.trace.record(
                TraceRecordKind::PointerStationaryRehitQueued {
                    hit_test_generation: input_context.hit_test_generation(),
                    coordinate_revision: input_context.coordinate_revision(),
                },
                Some(work_sequence),
                published,
                None,
                None,
                None,
            )
        } else {
            None
        };
        let Some(root_quota) = root_quota else {
            self.enter_terminal(RuntimeTerminalReason::Poisoned, 0);
            return;
        };
        let committed = self.queue.push_pointer_rehit_rooted(
            root_quota,
            input_context.clone(),
            instant,
            causal_parent,
            TraceReservation::continuation(),
        );
        match committed {
            Ok(_) => self.external_queue_commit_accepted(),
            Err(_) => {
                self.enter_terminal(RuntimeTerminalReason::Poisoned, 0);
            }
        }
    }

    fn record_surface_publication(
        &mut self,
        reservation: TraceReservation,
        causal_parent: Option<TraceSequence>,
        instant: MonotonicInstant,
        publication: &crate::SurfacePublication,
    ) -> Option<TraceSequence> {
        if !self.trace.is_enabled() {
            self.trace.release_reservation(reservation);
            return None;
        }
        let input_context = publication.input_context();
        let publication_context = TracePublicationContext::new(
            TraceSurfaceContext::accepted(input_context, TraceSurfaceSnapshotKind::Current),
            self.report.generation(),
            publication.frame().nodes().len(),
            self.surface_publication.phase_report().executed().to_vec(),
        );
        let published = self.trace.record_reserved_draft(
            reservation,
            TraceRecordDraft::publication_fact(
                TraceRecordKind::SurfacePublished,
                instant,
                publication_context,
            )
            .with_causal_parent(causal_parent),
        );
        if published.is_none() {
            self.enter_terminal(RuntimeTerminalReason::TraceSequenceExhausted, 0);
        }
        published
    }

    fn replenish_surface_publication_reservation(&mut self) {
        let Some(reservation) = self.trace.reserve_surface_publication() else {
            self.enter_terminal(RuntimeTerminalReason::TraceSequenceExhausted, 0);
            return;
        };
        self.surface_trace.publication_reservation = reservation;
    }

    pub(crate) fn note_surface_focus_validation(&mut self) {
        self.surface_publication.note_focus_validation();
    }

    pub(crate) const fn last_surface_phase_report(&self) -> &crate::SurfacePhaseReport {
        self.surface_publication.phase_report()
    }
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use runenui_core::{
        AnimationId, Element, ExplicitTimeline, MotionEasing, MotionKeyframe, MotionRepeat,
        MotionValue, NoHostProtocol, ReducedMotionStrategy, SceneOpacity, StyleEnvironment,
        TimelineSpec, UnitInterval, Widget,
    };

    use crate::{LayoutConstraints, RuntimeConfig, SurfaceBuildContext};

    use super::*;

    #[derive(Debug)]
    struct MotionProbe;

    impl Widget<()> for MotionProbe {
        fn producer_budget(
            &self,
            phase: runenui_core::WidgetProducerPhase,
            _state: &Self::State,
        ) -> runenui_core::WidgetProducerBudget {
            match phase {
                runenui_core::WidgetProducerPhase::Subscriptions => {
                    runenui_core::WidgetProducerBudget::new(0).with_subscriptions(0)
                }
                _ => runenui_core::WidgetProducerBudget::new(0),
            }
        }

        type State = ();

        fn create_state(&self) -> Self::State {}
    }

    fn active_opacity_timeline() -> ExplicitTimeline {
        let spec = TimelineSpec::new(
            vec![
                MotionKeyframe::new(
                    UnitInterval::ZERO,
                    MotionValue::Opacity(SceneOpacity::TRANSPARENT),
                ),
                MotionKeyframe::new(
                    UnitInterval::ONE,
                    MotionValue::Opacity(SceneOpacity::OPAQUE),
                ),
            ],
            vec![MotionEasing::Linear],
            Duration::from_millis(100),
            Duration::ZERO,
            MotionRepeat::ONCE,
            Some(ReducedMotionStrategy::PreserveEssential),
        )
        .unwrap_or_else(|_| unreachable!("test timeline is valid"));
        ExplicitTimeline::new(
            AnimationId::from_static("fade")
                .unwrap_or_else(|_| unreachable!("test animation id is valid")),
            spec,
        )
    }

    fn published_count(runtime: &Runtime<(), (), NoHostProtocol>) -> usize {
        runtime
            .trace
            .records()
            .filter(|record| matches!(record.kind(), TraceRecordKind::SurfacePublished))
            .count()
    }

    #[test]
    fn motion_followup_redraw_exhaustion_refuses_before_publication_commit() {
        let mut runtime = Runtime::<(), (), NoHostProtocol>::mount(
            (),
            |()| Element::new(MotionProbe).timeline(active_opacity_timeline()),
            RuntimeConfig::default(),
            runenui_core::ProducerBudget::new(1, 0).with_reconciliation(1, 0, 1),
            |(), _| {
                Ok(crate::runtime::PreparedInitialApplicationWork {
                    application: crate::transaction::TransactionLedger::from_outputs(Vec::new(), 0)
                        .unwrap_or_else(|_| unreachable!("empty initial ledger fits")),
                    subscriptions: Vec::new(),
                })
            },
        )
        .unwrap_or_else(|error| unreachable!("internal test mount rejected: {error}"));
        runtime
            .surface_publication
            .seed_redraw_revision_for_test(u64::MAX);
        assert!(!runtime.surface_publication.is_dirty());
        let phases_before = runtime.last_surface_phase_report().clone();
        let published_before = published_count(&runtime);
        let environment = StyleEnvironment::default();
        let context = SurfaceBuildContext::new(&environment, LayoutConstraints::unbounded());
        let reason = RuntimeTerminalReason::SurfacePublicationCounterExhausted(
            SurfacePublicationCounter::RedrawRevision,
        );

        assert_eq!(
            runtime.publish_surface(&context),
            Err(PublishSurfaceError::Terminal(reason))
        );
        assert_eq!(runtime.status, RuntimeStatus::Terminal(reason));
        assert_eq!(published_count(&runtime), published_before);
        assert_eq!(runtime.last_surface_phase_report(), &phases_before);
    }
}
