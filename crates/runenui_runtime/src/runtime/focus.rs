use runenui_core::{
    CommandOrigin, CompositionCancel, CompositionCancelReason, CompositionEvent, FocusEvent,
    FocusEventKind, FocusReason, HostProtocol, MonotonicInstant, SemanticCommand, UiEvent,
    WidgetInvalidation,
};

use super::{CollectedRoutedOutput, RoutedTransaction, Runtime, RuntimeStatus};
use crate::{
    MountedNodeId, ReconciliationGeneration, TraceContext, TraceDeliveryOutcome, TraceEventContext,
    TraceEventFamily, TraceFocusBoundaryOutcome, TraceModalityTransition, TraceRecordKind,
    TraceRouteSnapshot, TraceRoutedIntegrityFailure, TraceSequence, TraceSpaceCleanupReason,
    TraceSurfaceContext, TraceTarget, TraceTargetTransition, WorkSequence,
    focus::{
        FocusBoundaryOutcome, FocusNavigation, FocusSelection, is_focus_eligible, nearest_scope,
        select_focus,
    },
    mounted::{PlannedInvalidation, PlannedLifetimeReason, RouteBuildError, TargetStatus},
    trace::{MandatoryTracePlan, TraceRecordDraft, TraceReservation},
};

#[derive(Clone, Copy)]
pub(crate) struct InputLifetimeCleanupCause {
    pub sequence: Option<WorkSequence>,
    pub causal_parent: Option<TraceSequence>,
    pub instant: MonotonicInstant,
    pub origin: CommandOrigin,
}

impl InputLifetimeCleanupCause {
    pub(crate) const fn new(
        sequence: Option<WorkSequence>,
        causal_parent: Option<TraceSequence>,
        instant: MonotonicInstant,
        origin: CommandOrigin,
    ) -> Self {
        Self {
            sequence,
            causal_parent,
            instant,
            origin,
        }
    }
}

pub(in crate::runtime) struct ReconciledFocusCleanup {
    pub old_route: Vec<TraceTarget>,
    pub reason: FocusReason,
    pub sequence: WorkSequence,
    pub causal_parent: Option<TraceSequence>,
    pub instant: MonotonicInstant,
    pub before: ReconciliationGeneration,
    pub after: ReconciliationGeneration,
    pub trace_target: Option<TraceTarget>,
    pub surface: Option<TraceSurfaceContext>,
}

struct FocusNotificationPlan {
    kind: FocusEventKind,
    reason: FocusReason,
    target: MountedNodeId,
    route: Vec<MountedNodeId>,
    related: Option<MountedNodeId>,
    previous: Option<MountedNodeId>,
    current: Option<MountedNodeId>,
}

impl<State, Action, Protocol: HostProtocol> Runtime<State, Action, Protocol> {
    /// Applies input-lifetime revocation selected by the sole reconciliation
    /// plan while every old route is still live. The plan has already expanded
    /// subtree invalidation, so same-slot reuse can never be mistaken for the
    /// old owner.
    pub(in crate::runtime) fn cleanup_planned_input_lifetimes(
        &mut self,
        invalidated: &[PlannedInvalidation],
        cause: InputLifetimeCleanupCause,
    ) {
        let composition_reason = self.composition.owner().and_then(|owner| {
            invalidated
                .iter()
                .find(|planned| planned.id() == owner)
                .map(|planned| match planned.reason() {
                    PlannedLifetimeReason::Removal => CompositionCancelReason::Removal,
                    PlannedLifetimeReason::Replacement => CompositionCancelReason::Replacement,
                })
        });
        if let Some(reason) = composition_reason
            && self.cancel_composition_while_live(reason, cause).is_err()
        {
            let retirement_parent = self.suppress_composition_cleanup(reason, cause);
            if matches!(self.status, RuntimeStatus::Running) {
                self.enter_terminal_with_parent(
                    crate::RuntimeTerminalReason::Poisoned,
                    0,
                    retirement_parent,
                );
            }
            return;
        }
        if matches!(self.status, RuntimeStatus::Terminal(_)) {
            return;
        }
        let space_reason = self.space_ownership.as_ref().and_then(|ownership| {
            invalidated
                .iter()
                .find(|planned| planned.id() == &ownership.target)
                .map(|planned| match planned.reason() {
                    PlannedLifetimeReason::Removal => TraceSpaceCleanupReason::Removal,
                    PlannedLifetimeReason::Replacement => TraceSpaceCleanupReason::Replacement,
                })
        });
        if let Some(reason) = space_reason {
            self.revoke_space_ownership_with_cause(reason, cause);
        }
    }

    /// A compatible update leaves a lifetime live, but may still revoke its
    /// interaction or text-input capability. Recheck each affected owner once
    /// before focus cleanup can route or suppress `FocusOut`.
    pub(in crate::runtime) fn cleanup_lost_input_capabilities(
        &mut self,
        cause: InputLifetimeCleanupCause,
    ) {
        let composition_owner = self.composition.owner().cloned();
        let space_owner = self
            .space_ownership
            .as_ref()
            .map(|ownership| ownership.target.clone());

        let composition_capabilities = composition_owner.as_ref().map(|owner| {
            if self.tree.target_status(owner) == TargetStatus::Live {
                self.tree.refresh_input_capabilities(owner).ok()
            } else {
                None
            }
        });
        let space_capabilities = match (space_owner.as_ref(), composition_owner.as_ref()) {
            (Some(space), Some(composition)) if space == composition => composition_capabilities,
            (Some(space), _) => Some(if self.tree.target_status(space) == TargetStatus::Live {
                self.tree.refresh_input_capabilities(space).ok()
            } else {
                None
            }),
            (None, _) => None,
        };

        let composition_lost = composition_owner.is_some()
            && composition_capabilities
                .flatten()
                .is_none_or(|(activation, text_input)| {
                    !activation.enabled()
                        || !text_input.accepts_committed_text()
                        || !text_input.accepts_composition()
                });
        if composition_lost
            && self
                .cancel_composition_while_live(CompositionCancelReason::Disablement, cause)
                .is_err()
        {
            let retirement_parent =
                self.suppress_composition_cleanup(CompositionCancelReason::Disablement, cause);
            if matches!(self.status, RuntimeStatus::Running) {
                self.enter_terminal_with_parent(
                    crate::RuntimeTerminalReason::Poisoned,
                    0,
                    retirement_parent,
                );
            }
            return;
        }
        if matches!(self.status, RuntimeStatus::Terminal(_)) {
            return;
        }
        let space_lost = space_owner.is_some()
            && space_capabilities
                .flatten()
                .is_none_or(|(activation, _)| !activation.enabled() || !activation.is_actionable());
        if space_lost {
            self.revoke_space_ownership_with_cause(TraceSpaceCleanupReason::Disablement, cause);
        }
    }

    /// Uses the existing routed-event authority to notify a still-live
    /// composition owner. The composition lifetime keeps its canonical start
    /// sequence while the causing lifecycle operation is retained as parent.
    pub(crate) fn cancel_composition_while_live(
        &mut self,
        reason: CompositionCancelReason,
        cause: InputLifetimeCleanupCause,
    ) -> Result<Option<TraceSequence>, ()> {
        let (Some(owner), Some(start_sequence)) = (
            self.composition.owner().cloned(),
            self.composition.start_sequence(),
        ) else {
            return Ok(cause.causal_parent);
        };
        let facts = super::RoutedIngressFacts::new(
            start_sequence,
            owner.clone(),
            cause.origin,
            cause.instant,
            TraceEventContext::new(TraceEventFamily::Composition, false),
            cause.causal_parent,
            TraceReservation::continuation(),
        );
        let Some(mut transaction) = self
            .begin_routed_transaction_with_trace(facts, MandatoryTracePlan::composition_cleanup())
        else {
            return Err(());
        };
        if let Err(failure) =
            self.cancel_composition_in_transaction(&mut transaction, &owner, reason)
        {
            let current = transaction.failure_current_target.clone();
            self.poison_transaction(&transaction, failure, current.as_ref());
            return Err(());
        }
        let cleanup_parent = transaction.parent;
        let failure_facts = transaction.failure_facts();
        if self.commit_routed_transaction(transaction).is_err() {
            self.poison_routed_event(
                &failure_facts,
                TraceRoutedIntegrityFailure::CommitInvariantFailure,
                Some(&owner),
            );
            return Err(());
        }
        Ok(cleanup_parent)
    }

    /// Retires a composition that could not deliver its required cleanup after
    /// the runtime crossed an explicit terminal integrity boundary. The trace
    /// contains the preceding routed admission or integrity failure, but never
    /// falsely claims that a cancellation callback was delivered.
    pub(crate) fn suppress_composition_cleanup(
        &mut self,
        _reason: CompositionCancelReason,
        cause: InputLifetimeCleanupCause,
    ) -> Option<TraceSequence> {
        let (Some(owner), Some(start_sequence)) = (
            self.composition.owner().cloned(),
            self.composition.start_sequence(),
        ) else {
            return cause.causal_parent;
        };
        self.composition = crate::input::CompositionState::None;
        self.trace.record_event(
            TraceRecordKind::CompositionRetired,
            start_sequence,
            cause.causal_parent,
            Some(self.tree.trace_target(&owner)),
            cause.instant,
            &owner,
            Some(&owner),
            cause.origin,
        )
    }

    pub(crate) fn revoke_space_ownership_with_cause(
        &mut self,
        reason: TraceSpaceCleanupReason,
        cause: InputLifetimeCleanupCause,
    ) -> Option<TraceSequence> {
        if !self.trace.is_enabled() {
            self.revoke_space_ownership(reason);
            return cause.causal_parent;
        }
        let Some(ownership) = self.space_ownership.take() else {
            return cause.causal_parent;
        };
        let sequence = cause.sequence.unwrap_or(ownership.down_sequence);
        self.trace.record_event(
            TraceRecordKind::KeyboardSpaceOwnershipCleared { reason },
            sequence,
            cause.causal_parent,
            Some(self.tree.trace_target(&ownership.target)),
            cause.instant,
            &ownership.target,
            Some(&ownership.target),
            cause.origin,
        )
    }

    /// Routes composition cancellation through the current transaction before
    /// its old focus owner can receive `FocusOut` or become stale.
    fn cancel_composition_in_transaction(
        &mut self,
        transaction: &mut RoutedTransaction<Action>,
        owner: &MountedNodeId,
        reason: CompositionCancelReason,
    ) -> Result<(), TraceRoutedIntegrityFailure> {
        let Some(generation) = self.composition.generation().cloned() else {
            return Ok(());
        };
        if self.composition.owner() != Some(owner) {
            return Ok(());
        }
        let route = self.checked_focus_route(owner)?;
        let event = UiEvent::Composition(CompositionEvent::Cancel(
            CompositionCancel::__runtime_new(generation, reason),
        ));
        self.invoke_focus_callbacks(transaction, &event, route, None)?;
        self.composition = crate::input::CompositionState::None;
        transaction.parent = self.trace.record_event(
            TraceRecordKind::CompositionCancelled { reason },
            transaction.sequence,
            transaction.parent,
            Some(self.tree.trace_target(owner)),
            transaction.instant,
            &transaction.target,
            Some(owner),
            transaction.origin,
        );
        transaction.parent = self.trace.record_event(
            TraceRecordKind::CompositionRetired,
            transaction.sequence,
            transaction.parent,
            Some(self.tree.trace_target(owner)),
            transaction.instant,
            &transaction.target,
            Some(owner),
            transaction.origin,
        );
        Ok(())
    }

    pub(in crate::runtime) fn commit_pending_modality(
        &mut self,
        transaction: &mut RoutedTransaction<Action>,
    ) {
        let modality = transaction.pending_modality;
        let previous = self.focus.modality();
        if self.focus.set_modality(modality).is_some() && self.trace.is_enabled() {
            let context =
                TraceContext::modality_change(TraceModalityTransition::new(previous, modality));
            transaction.parent = self.trace.record_draft(
                TraceRecordDraft::focus_fact(
                    TraceRecordKind::ModalityChanged,
                    transaction.instant,
                    context,
                )
                .with_work_sequence(Some(transaction.sequence))
                .with_causal_parent(transaction.parent)
                .with_target(Some(transaction.target_trace.clone()))
                .with_routed_endpoints(
                    transaction.target.clone(),
                    None,
                    transaction.origin,
                ),
            );
        }
    }

    pub(in crate::runtime) fn apply_focus_default(
        &mut self,
        transaction: &mut RoutedTransaction<Action>,
        command: SemanticCommand,
    ) -> Result<(), TraceRoutedIntegrityFailure> {
        if command == SemanticCommand::RequestFocus {
            let target = transaction.target.clone();
            if is_focus_eligible(&mut self.tree, &target) {
                return self.commit_focus_transition(
                    transaction,
                    Some(target),
                    FocusReason::ProgrammaticRequest,
                );
            }
            return Ok(());
        }
        let (navigation, reason) = match command {
            SemanticCommand::FocusNext => (FocusNavigation::Next, FocusReason::LinearNavigation),
            SemanticCommand::FocusPrevious => {
                (FocusNavigation::Previous, FocusReason::LinearNavigation)
            }
            SemanticCommand::FocusLeft => (
                FocusNavigation::Direction(runenui_core::FocusDirection::Left),
                FocusReason::DirectionalNavigation,
            ),
            SemanticCommand::FocusRight => (
                FocusNavigation::Direction(runenui_core::FocusDirection::Right),
                FocusReason::DirectionalNavigation,
            ),
            SemanticCommand::FocusUp => (
                FocusNavigation::Direction(runenui_core::FocusDirection::Up),
                FocusReason::DirectionalNavigation,
            ),
            SemanticCommand::FocusDown => (
                FocusNavigation::Direction(runenui_core::FocusDirection::Down),
                FocusReason::DirectionalNavigation,
            ),
            SemanticCommand::RestoreFocus => {
                (FocusNavigation::Restore, FocusReason::RememberedRestoration)
            }
            _ => return Ok(()),
        };
        let geometry = self.surface_publication.current_focus_geometry();
        let Some(selection) = select_focus(
            &mut self.tree,
            &self.focus,
            &transaction.target,
            navigation,
            &geometry,
        ) else {
            return Ok(());
        };
        self.record_focus_selection(transaction, command, &selection);
        if let Some(direction) = selection.scroll {
            if transaction.remaining_outputs == 0 {
                return Err(TraceRoutedIntegrityFailure::OutputAllowanceExceeded);
            }
            transaction.remaining_outputs -= 1;
            transaction
                .default_outputs
                .push(CollectedRoutedOutput::Command {
                    target: selection.active_scope,
                    command: SemanticCommand::LogicalFocusScroll(direction),
                    origin: CommandOrigin::__runtime_semantic_default(transaction.origin.source()),
                    causal_parent: transaction.parent,
                });
            return Ok(());
        }
        if selection.target.is_some() {
            self.commit_focus_transition(transaction, selection.target, reason)?;
        }
        Ok(())
    }

    fn record_focus_selection(
        &mut self,
        transaction: &mut RoutedTransaction<Action>,
        command: SemanticCommand,
        selection: &FocusSelection,
    ) {
        transaction.parent = self.trace.record_event(
            TraceRecordKind::FocusCommandEvaluated {
                command,
                linear_policy: selection.policy.linear(),
                directional_policy: selection.policy.directional(),
            },
            transaction.sequence,
            transaction.parent,
            Some(self.tree.trace_target(&selection.active_scope)),
            transaction.instant,
            &transaction.target,
            Some(&selection.active_scope),
            transaction.origin,
        );
        let outcome = match selection.outcome {
            FocusBoundaryOutcome::Candidate => TraceFocusBoundaryOutcome::Candidate,
            FocusBoundaryOutcome::Delegate => TraceFocusBoundaryOutcome::Delegated,
            FocusBoundaryOutcome::Trap => TraceFocusBoundaryOutcome::Trapped,
            FocusBoundaryOutcome::Stop => TraceFocusBoundaryOutcome::Stopped,
            FocusBoundaryOutcome::Wrap => TraceFocusBoundaryOutcome::Wrapped,
            FocusBoundaryOutcome::LogicalScroll => TraceFocusBoundaryOutcome::LogicalScroll,
            FocusBoundaryOutcome::Empty => TraceFocusBoundaryOutcome::Empty,
        };
        let trace_target = selection.target.as_ref().unwrap_or(&selection.active_scope);
        transaction.parent = self.trace.record_event(
            TraceRecordKind::FocusCandidateSelected { outcome },
            transaction.sequence,
            transaction.parent,
            Some(self.tree.trace_target(trace_target)),
            transaction.instant,
            &transaction.target,
            Some(trace_target),
            transaction.origin,
        );
        if command == SemanticCommand::RestoreFocus {
            transaction.parent = self.trace.record_event(
                if selection.remembered_rejected {
                    TraceRecordKind::FocusRestorationRejected
                } else {
                    TraceRecordKind::FocusRestorationAccepted
                },
                transaction.sequence,
                transaction.parent,
                Some(self.tree.trace_target(trace_target)),
                transaction.instant,
                &transaction.target,
                Some(trace_target),
                transaction.origin,
            );
        }
    }

    pub(in crate::runtime) fn commit_focus_transition(
        &mut self,
        transaction: &mut RoutedTransaction<Action>,
        new_target: Option<MountedNodeId>,
        reason: FocusReason,
    ) -> Result<(), TraceRoutedIntegrityFailure> {
        let old_target = self.focus.focused_node().cloned();
        if old_target == new_target {
            return Ok(());
        }
        if let Some(old) = old_target.as_ref() {
            self.cancel_composition_in_transaction(
                transaction,
                old,
                runenui_core::CompositionCancelReason::FocusTransfer,
            )?;
        }
        self.revoke_space_for_focus_transfer(transaction, old_target.as_ref());
        let old_route = match old_target.as_ref() {
            Some(old) if self.tree.target_status(old) == TargetStatus::Live => {
                self.checked_focus_route(old)?
            }
            _ => Vec::new(),
        };
        let new_route = match new_target.as_ref() {
            Some(new) => self.checked_focus_route(new)?,
            None => Vec::new(),
        };

        let common = old_route
            .iter()
            .zip(&new_route)
            .take_while(|(old, new)| old == new)
            .count();
        let left = old_route.len().saturating_sub(common);
        let entered = new_route.len().saturating_sub(common);

        self.focus
            .commit(new_target.clone(), new_route.clone(), reason);
        if let Some(target) = new_target.as_ref() {
            for scope in new_route.iter().filter(|scope| {
                self.tree.node(scope).is_some_and(|node| {
                    node.parent.is_none()
                        || node
                            .focus_scope
                            .is_some_and(runenui_core::FocusScope::remembers_last)
                })
            }) {
                self.focus.remember(scope.clone(), target.clone());
            }
        }
        transaction.invalidation |= WidgetInvalidation::INTERACTION;
        self.record_focus_commit(
            transaction,
            old_target.as_ref(),
            new_target.as_ref(),
            reason,
            left,
            entered,
        );

        if let Some(old) = old_target.as_ref() {
            let plan = FocusNotificationPlan {
                kind: FocusEventKind::Out,
                reason,
                target: old.clone(),
                route: old_route,
                related: new_target.clone(),
                previous: old_target.clone(),
                current: new_target.clone(),
            };
            if plan.route.is_empty() {
                self.record_suppressed_focus_notification(transaction, &plan);
            } else {
                self.invoke_focus_notification(transaction, plan)?;
            }
        }
        if let Some(new) = new_target {
            self.invoke_focus_notification(
                transaction,
                FocusNotificationPlan {
                    kind: FocusEventKind::In,
                    reason,
                    target: new,
                    route: new_route,
                    related: old_target.clone(),
                    previous: old_target,
                    current: self.focus.focused_node().cloned(),
                },
            )?;
        }
        Ok(())
    }

    fn revoke_space_for_focus_transfer(
        &mut self,
        transaction: &mut RoutedTransaction<Action>,
        old_target: Option<&MountedNodeId>,
    ) {
        if self
            .space_ownership
            .as_ref()
            .is_some_and(|owner| old_target == Some(&owner.target))
        {
            transaction.parent = self.revoke_space_ownership_with_cause(
                TraceSpaceCleanupReason::FocusTransfer,
                InputLifetimeCleanupCause::new(
                    Some(transaction.sequence),
                    transaction.parent,
                    transaction.instant,
                    transaction.origin,
                ),
            );
        }
    }

    fn record_focus_commit(
        &mut self,
        transaction: &mut RoutedTransaction<Action>,
        old_target: Option<&MountedNodeId>,
        new_target: Option<&MountedNodeId>,
        reason: FocusReason,
        left: usize,
        entered: usize,
    ) {
        let current_target = new_target.or(old_target);
        if self.trace.is_enabled() {
            let transition = TraceTargetTransition::new(
                old_target.map(|target| self.tree.trace_target(target)),
                new_target.map(|target| self.tree.trace_target(target)),
            );
            let context = TraceContext::focus_transition(
                self.surface_publication.current_trace_surface_context(),
                transition,
            );
            transaction.parent = self.trace.record_draft(
                TraceRecordDraft::focus_fact(
                    TraceRecordKind::FocusTransitionCommitted { reason },
                    transaction.instant,
                    context,
                )
                .with_work_sequence(Some(transaction.sequence))
                .with_causal_parent(transaction.parent)
                .with_target(current_target.map(|target| self.tree.trace_target(target)))
                .with_routed_endpoints(
                    transaction.target.clone(),
                    current_target.cloned(),
                    transaction.origin,
                ),
            );
        }
        transaction.parent = self.trace.record_event(
            TraceRecordKind::FocusWithinInvalidated { left, entered },
            transaction.sequence,
            transaction.parent,
            current_target.map(|target| self.tree.trace_target(target)),
            transaction.instant,
            &transaction.target,
            current_target,
            transaction.origin,
        );
    }

    fn checked_focus_route(
        &self,
        target: &MountedNodeId,
    ) -> Result<Vec<MountedNodeId>, TraceRoutedIntegrityFailure> {
        let route = self.tree.event_route(target).map_err(|error| match error {
            RouteBuildError::Target(_) => TraceRoutedIntegrityFailure::CommitInvariantFailure,
            RouteBuildError::BrokenTopology => TraceRoutedIntegrityFailure::BrokenTopology,
            RouteBuildError::BridgeMismatch => TraceRoutedIntegrityFailure::EventBridgeMismatch,
        })?;
        self.tree
            .preflight_event_bridges(&route)
            .map_err(|_| TraceRoutedIntegrityFailure::EventBridgeMismatch)?;
        Ok(route)
    }

    fn focus_notification_context(
        &self,
        plan: &FocusNotificationPlan,
        delivery: TraceDeliveryOutcome,
    ) -> Option<TraceContext> {
        self.trace.is_enabled().then(|| {
            let route = TraceRouteSnapshot::new(
                plan.route
                    .iter()
                    .map(|target| self.tree.trace_target(target))
                    .collect(),
                plan.related
                    .as_ref()
                    .map(|target| self.tree.trace_target(target)),
            );
            let transition = TraceTargetTransition::new(
                plan.previous
                    .as_ref()
                    .map(|target| self.tree.trace_target(target)),
                plan.current
                    .as_ref()
                    .map(|target| self.tree.trace_target(target)),
            );
            TraceContext::focus_notification(
                self.surface_publication.current_trace_surface_context(),
                route,
                transition,
                delivery,
            )
        })
    }

    fn record_suppressed_focus_notification(
        &mut self,
        transaction: &mut RoutedTransaction<Action>,
        plan: &FocusNotificationPlan,
    ) {
        let Some(context) = self.focus_notification_context(plan, TraceDeliveryOutcome::Suppressed)
        else {
            return;
        };
        transaction.parent = self.trace.record_draft(
            TraceRecordDraft::focus_fact(
                TraceRecordKind::FocusNotificationResolved { kind: plan.kind },
                transaction.instant,
                context,
            )
            .with_work_sequence(Some(transaction.sequence))
            .with_causal_parent(transaction.parent)
            .with_target(Some(self.tree.trace_target(&plan.target)))
            .with_routed_endpoints(
                transaction.target.clone(),
                Some(plan.target.clone()),
                transaction.origin,
            ),
        );
    }

    fn invoke_focus_notification(
        &mut self,
        transaction: &mut RoutedTransaction<Action>,
        plan: FocusNotificationPlan,
    ) -> Result<(), TraceRoutedIntegrityFailure> {
        let context = self.focus_notification_context(&plan, TraceDeliveryOutcome::Delivered);
        let event = UiEvent::Focus(FocusEvent::__runtime_new(
            plan.kind,
            plan.reason,
            plan.target.clone(),
        ));
        self.invoke_focus_callbacks(transaction, &event, plan.route, plan.related.as_ref())?;
        if let Some(context) = context {
            transaction.parent = self.trace.record_draft(
                TraceRecordDraft::focus_fact(
                    TraceRecordKind::FocusNotificationResolved { kind: plan.kind },
                    transaction.instant,
                    context,
                )
                .with_work_sequence(Some(transaction.sequence))
                .with_causal_parent(transaction.parent)
                .with_target(Some(self.tree.trace_target(&plan.target)))
                .with_routed_endpoints(
                    transaction.target.clone(),
                    Some(plan.target),
                    transaction.origin,
                ),
            );
        }
        Ok(())
    }

    pub(in crate::runtime) fn prune_focus_memory(&mut self) {
        let tree = &self.tree;
        self.focus.retain_remembered(|scope, target| {
            tree.target_status(scope) == TargetStatus::Live
                && tree.target_status(target) == TargetStatus::Live
                && nearest_scope(tree, target).as_ref() == Some(scope)
        });
    }

    pub(in crate::runtime) fn commit_reconciled_focus_cleanup(
        &mut self,
        cleanup: ReconciledFocusCleanup,
    ) {
        let ReconciledFocusCleanup {
            old_route,
            reason,
            sequence,
            causal_parent,
            instant,
            before,
            after,
            trace_target,
            surface,
        } = cleanup;
        self.focus.commit(None, Vec::new(), reason);
        let Some(trace_target) = trace_target else {
            return;
        };
        let transition_context = TraceContext::focus_transition(
            surface.clone(),
            TraceTargetTransition::new(Some(trace_target.clone()), None),
        );
        let transition = self.trace.record_draft(
            TraceRecordDraft::focus_fact(
                TraceRecordKind::FocusTransitionCommitted { reason },
                instant,
                transition_context,
            )
            .with_work_sequence(Some(sequence))
            .with_causal_parent(causal_parent)
            .with_reconciliation(Some(before), Some(after))
            .with_target(Some(trace_target.clone())),
        );
        let within = self.trace.record_draft(
            TraceRecordDraft::lifecycle_fact(
                TraceRecordKind::FocusWithinInvalidated {
                    left: old_route.len(),
                    entered: 0,
                },
                instant,
            )
            .with_work_sequence(Some(sequence))
            .with_causal_parent(transition)
            .with_reconciliation(Some(before), Some(after))
            .with_target(Some(trace_target.clone())),
        );
        let notification_context = TraceContext::focus_notification(
            surface,
            TraceRouteSnapshot::new(old_route, None),
            TraceTargetTransition::new(Some(trace_target.clone()), None),
            TraceDeliveryOutcome::Suppressed,
        );
        self.trace.record_draft(
            TraceRecordDraft::focus_fact(
                TraceRecordKind::FocusNotificationResolved {
                    kind: FocusEventKind::Out,
                },
                instant,
                notification_context,
            )
            .with_work_sequence(Some(sequence))
            .with_causal_parent(within)
            .with_reconciliation(Some(before), Some(after))
            .with_target(Some(trace_target)),
        );
    }

    pub(in crate::runtime) fn clear_focus_for_shutdown(
        &mut self,
        causal_parent: Option<TraceSequence>,
        instant: MonotonicInstant,
    ) -> Option<TraceSequence> {
        let old_target = self.focus.focused_node().cloned();
        let old_route_len = self.focus.route_len();
        let trace_facts = self.trace.is_enabled().then(|| {
            let target = old_target
                .as_ref()
                .map(|target| self.tree.trace_target(target));
            let route = self
                .focus
                .route()
                .iter()
                .map(|target| self.tree.trace_target(target))
                .collect::<Vec<_>>();
            let surface = self.surface_publication.current_trace_surface_context();
            (target, route, surface)
        });
        self.focus.clear_all(FocusReason::Shutdown);
        let Some(_old_target) = old_target else {
            return causal_parent;
        };
        let Some((Some(trace_target), old_route, surface)) = trace_facts else {
            return causal_parent;
        };
        let transition_context = TraceContext::focus_transition(
            surface.clone(),
            TraceTargetTransition::new(Some(trace_target.clone()), None),
        );
        let transition = self.trace.record_draft(
            TraceRecordDraft::focus_fact(
                TraceRecordKind::FocusTransitionCommitted {
                    reason: FocusReason::Shutdown,
                },
                instant,
                transition_context,
            )
            .with_causal_parent(causal_parent)
            .with_target(Some(trace_target.clone())),
        );
        let within = self.trace.record_draft(
            TraceRecordDraft::lifecycle_fact(
                TraceRecordKind::FocusWithinInvalidated {
                    left: old_route_len,
                    entered: 0,
                },
                instant,
            )
            .with_causal_parent(transition)
            .with_target(Some(trace_target.clone())),
        );
        let notification_context = TraceContext::focus_notification(
            surface,
            TraceRouteSnapshot::new(old_route, None),
            TraceTargetTransition::new(Some(trace_target.clone()), None),
            TraceDeliveryOutcome::Suppressed,
        );
        self.trace.record_draft(
            TraceRecordDraft::focus_fact(
                TraceRecordKind::FocusNotificationResolved {
                    kind: FocusEventKind::Out,
                },
                instant,
                notification_context,
            )
            .with_causal_parent(within)
            .with_target(Some(trace_target)),
        )
    }
}
