use super::{
    ApplicationTransactionInput, HashMap, HostProtocol, IntoEffects, MandatoryTracePlan,
    OwnedTransactionLedger, PlannedApplicationTransaction, Runtime, RuntimeStatus,
    RuntimeTerminalReason, SubscriptionDiff, SubscriptionSet, TraceRecordKind, TransactionLedger,
    TransactionPlanError, UiApp, WorkOwner, mounted_effect_into_effect,
    required_application_transaction_trace_records,
};

impl<State, Action, Protocol: HostProtocol> Runtime<State, Action, Protocol> {
    pub(in crate::runtime) fn take_initial_mounted_ledgers(
        &mut self,
        initial_output_count: usize,
    ) -> Option<Vec<OwnedTransactionLedger<Action, Protocol>>> {
        let initial_mounted_outputs = core::mem::take(&mut self.initial_mounted_outputs);
        let mut total_outputs = initial_output_count;
        let mut mounted = Vec::with_capacity(initial_mounted_outputs.len());
        for (owner, outputs) in initial_mounted_outputs {
            total_outputs = total_outputs.checked_add(outputs.len())?;
            if total_outputs > self.limits.transaction_outputs() {
                return None;
            }
            let effects = outputs
                .into_iter()
                .map(mounted_effect_into_effect)
                .collect();
            let ledger =
                TransactionLedger::from_outputs(effects, self.limits.transaction_outputs())
                    .unwrap_or_else(|_| {
                        unreachable!("complete initial transaction allowance was checked")
                    });
            mounted.push(OwnedTransactionLedger {
                owner: WorkOwner::Mounted(owner),
                ledger,
            });
        }
        Some(mounted)
    }

    pub(crate) fn initialize_application_work<App>(&mut self)
    where
        App: UiApp<State = State, Action = Action, HostProtocol = Protocol>,
    {
        if !matches!(self.status, RuntimeStatus::Running) {
            return;
        }
        let effects = App::initial_effects(self.state());
        let Ok(ledger) =
            TransactionLedger::collect(effects.into_effects(), self.limits.transaction_outputs())
        else {
            self.enter_terminal(RuntimeTerminalReason::Poisoned, 0);
            return;
        };
        let initial_output_count = ledger.len();
        let mut subscriptions = SubscriptionSet::new();
        App::subscriptions(self.state(), &mut subscriptions);
        let SubscriptionDiff {
            invalidated,
            starts,
            duplicate_keys,
        } = self.derive_subscription_diff(
            &WorkOwner::Application,
            subscriptions.__runtime_into_declarations(),
        );
        let cancelled = invalidated.len();
        let mounted_subscription_dirty =
            core::mem::take(&mut self.initial_mounted_subscription_owners);
        let Some(mounted) = self.take_initial_mounted_ledgers(initial_output_count) else {
            self.enter_terminal(RuntimeTerminalReason::Poisoned, 0);
            return;
        };
        let input = ApplicationTransactionInput {
            lifecycle_invalidated: Vec::new(),
            mounted_subscription_dirty,
            application: ledger,
            application_subscription_invalidated: invalidated,
            application_subscription_starts: starts,
            mounted,
        };
        let plan = match PlannedApplicationTransaction::plan(input, &self.work, &self.queue) {
            Ok(plan) => plan,
            Err(error) => {
                let reason = match error {
                    TransactionPlanError::WorkSequenceExhausted => {
                        RuntimeTerminalReason::WorkSequenceExhausted
                    }
                    TransactionPlanError::WorkGenerationExhausted => {
                        RuntimeTerminalReason::WorkGenerationExhausted
                    }
                    TransactionPlanError::QueueFull | TransactionPlanError::RegistryFull => {
                        RuntimeTerminalReason::Poisoned
                    }
                };
                self.enter_terminal(reason, 0);
                return;
            }
        };
        let Some(required_trace_records) =
            required_application_transaction_trace_records(&plan).checked_add(1)
        else {
            self.enter_terminal(RuntimeTerminalReason::TraceSequenceExhausted, 0);
            return;
        };
        if !self
            .trace
            .can_admit(MandatoryTracePlan::planned_scheduler_transaction(
                required_trace_records,
            ))
        {
            self.enter_terminal(RuntimeTerminalReason::TraceSequenceExhausted, 0);
            return;
        }
        let transaction_parent = self.trace.record(
            TraceRecordKind::InitialApplicationTransactionStarted,
            None,
            None,
            None,
            None,
            None,
        );
        if self
            .commit_planned_application_transaction(
                plan,
                &duplicate_keys,
                cancelled,
                transaction_parent,
                HashMap::new(),
            )
            .is_err()
        {
            self.enter_terminal(RuntimeTerminalReason::Poisoned, 0);
            return;
        }
        self.record_optional(
            TraceRecordKind::InitialEffectsCommitted {
                count: initial_output_count,
            },
            None,
            None,
            None,
        );
    }
}
