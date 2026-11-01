use super::transaction::ApplicationTransactionCommitContext;
use super::{
    ApplicationTraceTransaction, ApplicationTransactionInput, HashMap, HashSet, HostProtocol,
    MandatoryTracePlan, OwnedTransactionLedger, PlannedApplicationTransaction, Runtime,
    RuntimeStatus, RuntimeTerminalReason, SubscriptionDiff, TraceRecordKind, TransactionLedger,
    TransactionPlanError, WorkOwner, mounted_effect_into_effect,
    required_application_transaction_trace_records,
};

impl<State, Action, Protocol: HostProtocol> Runtime<State, Action, Protocol> {
    pub(in crate::runtime) fn take_initial_mounted_ledgers(
        &mut self,
        initial_output_count: usize,
        output_allowance: usize,
    ) -> Result<Vec<OwnedTransactionLedger<Action, Protocol>>, RuntimeTerminalReason> {
        let initial_mounted_outputs = core::mem::take(&mut self.initial_mounted_outputs);
        let mut total_outputs = initial_output_count;
        let mut mounted = Vec::with_capacity(initial_mounted_outputs.len());
        for (owner, outputs) in initial_mounted_outputs {
            total_outputs = total_outputs
                .checked_add(outputs.len())
                .ok_or(RuntimeTerminalReason::Poisoned)?;
            if total_outputs > output_allowance.min(self.limits.transaction_outputs()) {
                return Err(RuntimeTerminalReason::ProducerContractViolation(
                    runenui_core::ProducerContractViolation {
                        kind: runenui_core::ProducerViolationKind::DeclarationExceeded,
                        phase: runenui_core::ProducerPhase::InitialEffects,
                        component: runenui_core::BudgetComponent::ApplicationOutputs,
                        declared: output_allowance,
                        attempted: Some(total_outputs),
                    },
                ));
            }
            let effects = outputs
                .into_iter()
                .map(mounted_effect_into_effect)
                .collect();
            let ledger = TransactionLedger::from_outputs(
                effects,
                output_allowance.min(self.limits.transaction_outputs()),
            )
            .unwrap_or_else(|_| unreachable!("complete initial transaction allowance was checked"));
            mounted.push(OwnedTransactionLedger {
                owner: WorkOwner::Mounted(owner),
                ledger,
            });
        }
        Ok(mounted)
    }

    fn commit_initial_application_work(
        &mut self,
        input: super::ApplicationTransactionInput<Action, Protocol>,
        duplicate_keys: HashSet<runenui_core::WorkKey>,
        cancelled: usize,
        root_quota: crate::queue::RootQuotaId,
    ) {
        let initial_output_count = input.application.len();
        let producer_budget = self.initial_producer_budget;
        let plan = match PlannedApplicationTransaction::plan(
            input,
            &self.work,
            &self.queue,
            self.state(),
            self.action_producer_budget,
            Some(root_quota),
            Some(producer_budget),
        ) {
            Ok(plan) => plan,
            Err(error) => {
                let reason = match error {
                    TransactionPlanError::ProducerQuotaExceeded => RuntimeTerminalReason::Poisoned,
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
        let trace_transaction = ApplicationTraceTransaction::new(self.now());
        let transaction_parent = self.trace.record_draft(
            trace_transaction.fact(TraceRecordKind::InitialApplicationTransactionStarted),
        );
        let context = ApplicationTransactionCommitContext::new(
            duplicate_keys,
            cancelled,
            transaction_parent,
            HashMap::new(),
            trace_transaction,
            Some(root_quota),
        );
        if self
            .commit_planned_application_transaction(plan, context)
            .is_err()
        {
            self.enter_terminal(RuntimeTerminalReason::Poisoned, 0);
            return;
        }
        self.trace.record_draft(
            trace_transaction.fact(TraceRecordKind::InitialEffectsCommitted {
                count: initial_output_count,
            }),
        );
        if self.queue.finish_root_action(root_quota).is_err() {
            self.enter_terminal(RuntimeTerminalReason::Poisoned, 0);
        }
    }

    pub(crate) fn initialize_application_work(&mut self) {
        if !matches!(self.status, RuntimeStatus::Running) {
            return;
        }
        let Some(reservation) = self.initial_root_reservation.take() else {
            self.enter_terminal(RuntimeTerminalReason::Poisoned, 0);
            return;
        };
        let Ok(root_quota) = self.queue.begin_root_action(&reservation) else {
            self.enter_terminal(RuntimeTerminalReason::Poisoned, 0);
            return;
        };
        let Some(initial_work) = self.initial_application_work.take() else {
            self.enter_terminal(RuntimeTerminalReason::Poisoned, 0);
            return;
        };
        let ledger = initial_work.application;
        let initial_output_count = ledger.len();
        let producer_budget = self.initial_producer_budget;
        let SubscriptionDiff {
            invalidated,
            starts,
            duplicate_keys,
        } = self.derive_subscription_diff(&WorkOwner::Application, initial_work.subscriptions);
        if let Err(reason) = self.validate_subscription_residency(
            Some(root_quota),
            &invalidated,
            starts.len(),
            runenui_core::ProducerPhase::ApplicationSubscriptions,
        ) {
            self.enter_terminal(reason, 0);
            return;
        }
        let cancelled = invalidated.len();
        let mounted_subscription_dirty =
            core::mem::take(&mut self.initial_mounted_subscription_owners);
        let mounted = match self.take_initial_mounted_ledgers(
            initial_output_count,
            producer_budget.application_outputs(),
        ) {
            Ok(mounted) => mounted,
            Err(reason) => {
                self.enter_terminal(reason, 0);
                return;
            }
        };
        if let Err(reason) = crate::transaction::validate_application_family_starts(
            producer_budget.work_starts(),
            core::iter::once(&ledger).chain(mounted.iter().map(|batch| &batch.ledger)),
            &starts,
        ) {
            let reason = match reason {
                RuntimeTerminalReason::ProducerContractViolation(mut cause) => {
                    cause.phase = runenui_core::ProducerPhase::InitialEffects;
                    RuntimeTerminalReason::ProducerContractViolation(cause)
                }
                reason => reason,
            };
            self.enter_terminal(reason, 0);
            return;
        }
        let input = ApplicationTransactionInput {
            lifecycle_invalidated: Vec::new(),
            mounted_subscription_dirty,
            application: ledger,
            application_subscription_invalidated: invalidated,
            application_subscription_starts: starts,
            mounted,
        };
        self.commit_initial_application_work(input, duplicate_keys, cancelled, root_quota);
    }
}
