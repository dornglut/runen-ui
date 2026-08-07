use runenui_core::{FocusReason, HostProtocol, MonotonicInstant, WidgetInvalidation};

use super::{
    super::{
        CollectedRoutedOutput, Runtime, application::PlannedWorkTrace, mounted_effect_into_effect,
    },
    transaction::RoutedTransaction,
};
use crate::{
    TraceRecordKind,
    queue::ApplicationActionOrigin,
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
        if self.routed_commit_failure_for_test {
            return Err(());
        }
        pre_output_commit(self, &mut transaction)?;
        if !transaction.pointer_capture_requests.is_empty() {
            return Err(());
        }
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
        let plan = self.plan_routed_outputs(&mut transaction)?;
        self.commit_routed_plan(transaction, plan)
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

    fn commit_routed_plan(
        &mut self,
        transaction: RoutedTransaction<Action>,
        plan: PlannedApplicationTransaction<Action, Protocol>,
    ) -> Result<(), ()> {
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
        self.finish_routed_invalidation(transaction.invalidation, committed, transaction.instant);
        Ok(())
    }

    fn finish_routed_invalidation(
        &mut self,
        invalidation: WidgetInvalidation,
        causal_parent: Option<crate::TraceSequence>,
        instant: MonotonicInstant,
    ) {
        if invalidation.contains(WidgetInvalidation::INTERACTION) {
            self.tree.finish_focus_validation();
        }
        if crate::mounted::publication_is_dirty(invalidation) {
            self.request_redraw(causal_parent, instant);
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
                        ApplicationActionOrigin::RoutedCommand,
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
