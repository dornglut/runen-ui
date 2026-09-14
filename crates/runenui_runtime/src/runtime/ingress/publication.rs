use core::mem;

use runenui_core::MonotonicInstant;

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
    TraceSurfaceSnapshotKind,
    trace::{TraceRecordDraft, TraceReservation},
};

struct PublicationAdmission {
    surface: SurfacePublicationAdmission,
    stationary_rehit: bool,
}

fn candidate_trace_plan(
    redraw_pending: bool,
    stationary_rehit: bool,
    request_followup: bool,
) -> Option<MandatoryTracePlan> {
    let plan = MandatoryTracePlan::surface_publication(redraw_pending, stationary_rehit);
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

    pub(crate) fn publish_surface(
        &mut self,
        context: &crate::SurfaceBuildContext<'_>,
    ) -> Result<crate::SurfacePublication, PublishSurfaceError> {
        let admission = self.admit_surface_publication()?;
        let instant = self.now();
        let focused_owner = self.focus.focused_node().cloned();
        let interaction = self
            .pointer_registry
            .surface_interaction_projection(focused_owner.as_ref());
        let candidate = SurfacePublicationCandidateInputs::new(
            &interaction,
            focused_owner.as_ref(),
            admission.surface,
            instant,
        );
        let staged = match self.surface_publication.plan_publication(
            &mut self.tree,
            &mut self.text_system,
            context,
            candidate,
        ) {
            Ok(staged) => staged,
            Err(SurfacePublicationPlanError::SemanticIntegrity) => {
                let reason = RuntimeTerminalReason::Poisoned;
                self.enter_terminal(reason, 0);
                return Err(PublishSurfaceError::Terminal(reason));
            }
            Err(SurfacePublicationPlanError::TextLayout(error)) => {
                return Err(PublishSurfaceError::TextLayout(error));
            }
            Err(SurfacePublicationPlanError::PresentationGeometry) => {
                return Err(PublishSurfaceError::PresentationGeometry);
            }
            Err(SurfacePublicationPlanError::Motion) => return Err(PublishSurfaceError::Motion),
            Err(SurfacePublicationPlanError::CounterExhausted(counter)) => {
                let reason = RuntimeTerminalReason::SurfacePublicationCounterExhausted(counter);
                self.enter_terminal(reason, 0);
                return Err(PublishSurfaceError::Terminal(reason));
            }
        };

        let motion_activity = staged.motion_activity();
        let request_followup =
            motion_activity.continuous_redraw() || motion_activity.followup_publication();
        let Some(candidate_trace_plan) = candidate_trace_plan(
            self.surface_publication.is_dirty(),
            admission.stationary_rehit,
            request_followup,
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

        let commit = staged.commit_store();
        let publication = self
            .surface_publication
            .commit_publication(&mut self.tree, commit);
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
        if admission.stationary_rehit {
            self.commit_stationary_pointer_rehit(&publication, instant, published);
        }
        if let Some(redraw) = redraw {
            self.acknowledge_redraw_at(&redraw, published, instant)
                .unwrap_or_else(|_| unreachable!("runtime-issued redraw request remains local"));
        }
        self.replenish_surface_publication_reservation();
        if let Some(admission) = followup_redraw {
            self.commit_admitted_redraw_request(admission, published, instant);
        }
        Ok(publication)
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
        let committed = self.queue.push_pointer_rehit_preflighted(
            input_context.clone(),
            instant,
            causal_parent,
            TraceReservation::continuation(),
        );
        match committed {
            Ok(_) => self.external_queue_commit_accepted(),
            Err(QueueCommitError::Full | QueueCommitError::SequenceExhausted) => {
                unreachable!("stationary pointer re-hit queue admission was preflighted")
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
        );
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
