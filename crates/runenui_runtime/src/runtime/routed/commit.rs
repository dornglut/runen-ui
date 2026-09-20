use runenui_core::{EventSource, FocusReason, HostProtocol, MonotonicInstant, WidgetInvalidation};

use super::{
    super::{
        CollectedRoutedOutput, Runtime, application::PlannedWorkTrace, mounted_effect_into_effect,
    },
    transaction::RoutedTransaction,
};
use crate::{
    TraceActionCategory, TraceRecordKind,
    transaction::{
        ApplicationTransactionInput, OwnedTransactionLedger, PlannedApplicationTransaction,
        TransactionLedger,
    },
    work::WorkOwner,
};

impl<State, Action, Protocol: HostProtocol> Runtime<State, Action, Protocol> {
    pub(crate) fn commit_routed_transaction(
        &mut self,
        transaction: RoutedTransaction<Action>,
    ) -> Result<(), ()> {
        self.commit_routed_transaction_with(transaction, |_, transaction| {
            if transaction.pointer_capture_requests.is_empty() {
                Ok(())
            } else {
                Err(())
            }
        })
    }

    pub(in crate::runtime) fn commit_routed_transaction_with(
        &mut self,
        mut transaction: RoutedTransaction<Action>,
        pre_output_commit: impl FnOnce(&mut Self, &mut RoutedTransaction<Action>) -> Result<(), ()>,
    ) -> Result<(), ()> {
        #[cfg(feature = "internal-test-seams")]
        if self.test_seams.routed_commit_failure {
            return Err(());
        }
        let pointer_interaction_before = (transaction.origin.source() == EventSource::Pointer)
            .then(|| self.pointer_registry.surface_interaction_projection(None));
        pre_output_commit(self, &mut transaction)?;
        if !transaction.pointer_capture_requests.is_empty() {
            return Err(());
        }
        let pointer_style_changed = pointer_interaction_before.is_some_and(|before| {
            before.content_differs(&self.pointer_registry.surface_interaction_projection(None))
        });
        let focused = self.focus.focused_node().cloned();
        if transaction
            .invalidation
            .contains(WidgetInvalidation::INTERACTION)
            && focused
                .as_ref()
                .is_some_and(|focused| !self.validate_focus(focused))
        {
            self.commit_focus_transition(&mut transaction, None, FocusReason::Disablement)
                .map_err(|_| ())?;
        }
        self.cancel_stale_framework_services();
        self.stage_committed_framework_services(&mut transaction);
        let plan = self.plan_routed_outputs(&mut transaction)?;
        self.commit_routed_plan(transaction, plan, pointer_style_changed)
    }

    fn plan_routed_outputs(
        &self,
        transaction: &mut RoutedTransaction<Action>,
    ) -> Result<PlannedApplicationTransaction<Action, Protocol>, ()> {
        let subscription_dirty: Vec<_> = core::mem::take(&mut transaction.subscription_dirty)
            .into_iter()
            .filter(|owner| !self.mounted_subscription_reconcile_pending.contains(owner))
            .collect();
        let mut mounted = Vec::with_capacity(transaction.mounted_work.len());
        for (owner, effect) in core::mem::take(&mut transaction.mounted_work) {
            let ledger = TransactionLedger::from_outputs(
                vec![mounted_effect_into_effect(effect)],
                self.limits.transaction_outputs(),
            )
            .map_err(|_| ())?;
            mounted.push(OwnedTransactionLedger {
                owner: WorkOwner::Mounted(owner),
                ledger,
            });
        }
        PlannedApplicationTransaction::plan(
            ApplicationTransactionInput {
                lifecycle_invalidated: Vec::new(),
                mounted_subscription_dirty: subscription_dirty,
                application: TransactionLedger::from_outputs(
                    Vec::new(),
                    self.limits.transaction_outputs(),
                )
                .map_err(|_| ())?,
                application_subscription_invalidated: Vec::new(),
                application_subscription_starts: Vec::new(),
                mounted,
            },
            &self.work,
            &self.queue,
        )
        .map_err(|_| ())
    }

    #[allow(clippy::too_many_lines)] // Stages all mutation before the single routed commit boundary.
    fn commit_routed_plan(
        &mut self,
        mut transaction: RoutedTransaction<Action>,
        plan: PlannedApplicationTransaction<Action, Protocol>,
        pointer_style_changed: bool,
    ) -> Result<(), ()> {
        let focus_before = transaction.focus_before.clone();
        let PlannedApplicationTransaction {
            invalidated,
            starts,
            application_outputs,
            application_subscription_starts,
            mounted_outputs,
            mounted_subscription_dirty,
            next_generation,
            semantic_events,
        } = plan;
        if !application_outputs.is_empty() || !application_subscription_starts.is_empty() {
            return Err(());
        }
        let work_trace = PlannedWorkTrace::new(transaction.parent, transaction.instant);
        let cancellation_lineage = self.commit_application_starts(
            &invalidated,
            starts,
            next_generation,
            semantic_events,
            work_trace,
        );
        for owner in mounted_subscription_dirty {
            self.queue
                .push_mounted_subscription_reconcile(owner.clone(), transaction.parent)
                .map_err(|_| ())?;
            self.mounted_subscription_reconcile_pending.push(owner);
        }
        self.append_collected_routed_outputs(
            transaction.notification_outputs,
            transaction.instant,
        )?;
        self.append_collected_routed_outputs(transaction.routed_outputs, transaction.instant)?;
        self.append_collected_routed_outputs(transaction.default_outputs, transaction.instant)?;
        self.append_cancellation_envelopes(&invalidated, &cancellation_lineage);
        self.append_planned_outputs(mounted_outputs, transaction.parent, transaction.instant)
            .map_err(|_| ())?;
        let mut pointer_selection_changed = false;
        if let Some(selection) = transaction.pointer_selection_update.take() {
            pointer_selection_changed = self
                .editing
                .set_selection(&selection.owner, selection.selection, &selection.caret_map)
                .map_err(|_| ())?;
            if pointer_selection_changed {
                transaction.invalidation |= WidgetInvalidation::SEMANTICS;
                self.tree.mark_runtime_semantic_product_dirty();
            }
        }
        if let Some(transition) = transaction.pointer_selection_transition {
            let pointer_id = transaction
                .pointer_id
                .unwrap_or_else(|| unreachable!("text-selection transitions are pointer-owned"));
            let (kind, owner) = (
                match transition {
                    super::transaction::PointerSelectionTransition::Started => {
                        TraceRecordKind::PointerTextSelectionStarted { pointer_id }
                    }
                    super::transaction::PointerSelectionTransition::Updated => {
                        TraceRecordKind::PointerTextSelectionUpdated { pointer_id }
                    }
                    super::transaction::PointerSelectionTransition::Ended => {
                        TraceRecordKind::PointerTextSelectionEnded { pointer_id }
                    }
                    super::transaction::PointerSelectionTransition::Cancelled => {
                        TraceRecordKind::PointerTextSelectionCancelled { pointer_id }
                    }
                },
                transaction.target.clone(),
            );
            transaction.parent = self.trace.record_event(
                kind,
                transaction.sequence,
                transaction.parent,
                Some(self.tree.trace_target(&owner)),
                transaction.instant,
                &transaction.target,
                Some(&owner),
                transaction.origin,
            );
        }
        let mut scroll_changed = false;
        for update in &transaction.scroll_updates {
            scroll_changed |= self.tree.commit_scroll_offset(&update.owner, update.offset);
        }
        for consumption in &transaction.scroll_consumptions {
            transaction.parent = self.trace.record_event(
                TraceRecordKind::LogicalScrollOwnerApplied {
                    evaluation_order: consumption.evaluation_order,
                    offered: consumption.offered,
                    consumed: consumption.consumed,
                    remainder: consumption.remainder,
                    offset: consumption.offset,
                    maximum: consumption.maximum,
                },
                transaction.sequence,
                transaction.parent,
                Some(self.tree.trace_target(&consumption.owner)),
                transaction.instant,
                &transaction.target,
                Some(&consumption.owner),
                transaction.origin,
            );
        }
        if let Some(remainder) = transaction.scroll_chain_remainder {
            transaction.parent = self.trace.record_event(
                TraceRecordKind::LogicalScrollChainCompleted { remainder },
                transaction.sequence,
                transaction.parent,
                Some(transaction.target_trace.clone()),
                transaction.instant,
                &transaction.target,
                Some(&transaction.target),
                transaction.origin,
            );
        }
        let committed = self.trace.record_event(
            TraceRecordKind::RoutedEventCommitted,
            transaction.sequence,
            transaction.parent,
            Some(transaction.target_trace),
            transaction.instant,
            &transaction.target,
            None,
            transaction.origin,
        );
        let focus_changed = self.focus.focused_node() != focus_before.as_ref();
        self.finish_routed_invalidation(
            transaction.invalidation,
            focus_changed,
            pointer_style_changed || scroll_changed || pointer_selection_changed,
            committed,
            transaction.instant,
        );
        Ok(())
    }

    fn finish_routed_invalidation(
        &mut self,
        invalidation: WidgetInvalidation,
        focus_changed: bool,
        pointer_presentation_changed: bool,
        causal_parent: Option<crate::TraceSequence>,
        instant: MonotonicInstant,
    ) {
        if invalidation.contains(WidgetInvalidation::INTERACTION) {
            self.tree.finish_focus_validation();
        }
        if focus_changed {
            self.tree.mark_runtime_semantic_product_dirty();
        }
        if focus_changed
            || pointer_presentation_changed
            || crate::mounted::publication_is_dirty(invalidation)
        {
            self.request_redraw(causal_parent, instant);
        }
        if focus_changed {
            self.cancel_stale_framework_services();
        }
    }

    fn append_collected_routed_outputs(
        &mut self,
        outputs: Vec<CollectedRoutedOutput<Action>>,
        instant: MonotonicInstant,
    ) -> Result<(), ()> {
        for output in outputs {
            match output {
                CollectedRoutedOutput::Action {
                    action,
                    causal_parent,
                    current_target,
                } => {
                    self.commit_preflighted_action(
                        action,
                        causal_parent,
                        Some(self.tree.trace_target(&current_target)),
                        TraceActionCategory::RoutedCommand,
                        instant,
                    )
                    .map_err(|_| ())?;
                }
                CollectedRoutedOutput::EditAction {
                    action,
                    origin,
                    causal_parent,
                    current_target,
                } => {
                    self.commit_preflighted_edit_action(
                        action,
                        origin,
                        causal_parent,
                        Some(self.tree.trace_target(&current_target)),
                        instant,
                    )
                    .map_err(|_| ())?;
                }
                CollectedRoutedOutput::Command {
                    target,
                    command,
                    origin,
                    causal_parent,
                } => {
                    self.commit_preflighted_routed_command(
                        &target,
                        command,
                        origin,
                        causal_parent,
                        instant,
                    )
                    .map_err(|_| ())?;
                }
            }
        }
        Ok(())
    }
}
