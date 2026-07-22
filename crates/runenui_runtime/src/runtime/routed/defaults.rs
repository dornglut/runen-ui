use runenui_core::{__runtime::MountedEffect, HostProtocol, SemanticCommand};

use super::{
    super::{CollectedRoutedOutput, Runtime},
    transaction::RoutedTransaction,
};
use crate::{TraceRecordKind, TraceRoutedIntegrityFailure};

impl<State, Action, Protocol: HostProtocol> Runtime<State, Action, Protocol> {
    pub(super) fn apply_semantic_default(
        &mut self,
        transaction: &mut RoutedTransaction<Action>,
        command: SemanticCommand,
    ) -> Result<(), TraceRoutedIntegrityFailure> {
        let kind = if transaction.default_prevented {
            TraceRecordKind::SemanticDefaultSuppressed { command }
        } else {
            TraceRecordKind::SemanticDefaultApplied { command }
        };
        transaction.parent = self.trace.record_event(
            kind,
            transaction.sequence,
            transaction.parent,
            Some(transaction.target_trace.clone()),
            transaction.instant,
            &transaction.target,
            Some(&transaction.target),
            transaction.origin,
        );
        if transaction.default_prevented || command != SemanticCommand::Activate {
            return Ok(());
        }
        transaction.failure_current_target = Some(transaction.target.clone());
        #[cfg(feature = "internal-test-seams")]
        if self.routed_semantic_default_failure_for_test {
            return Err(TraceRoutedIntegrityFailure::SemanticDefaultFailure);
        }
        self.invoke_activation_default(transaction)
    }

    fn invoke_activation_default(
        &mut self,
        transaction: &mut RoutedTransaction<Action>,
    ) -> Result<(), TraceRoutedIntegrityFailure> {
        let activation = self
            .tree
            .activation_probe(&transaction.target)
            .map_err(|_| TraceRoutedIntegrityFailure::SemanticDefaultFailure)?;
        if !activation.enabled() || !activation.is_actionable() {
            return Ok(());
        }
        let target = transaction.target.clone();
        let subscription_credit = transaction.subscription_credit(&target);
        let activation = self
            .tree
            .activate(&target, transaction.output_allowance(&target))
            .map_err(|_| TraceRoutedIntegrityFailure::SemanticDefaultFailure)?;
        transaction.remaining_outputs = activation.remaining_outputs;
        if activation.overflowed {
            return Err(TraceRoutedIntegrityFailure::OutputAllowanceExceeded);
        }
        self.record_event_mutation(
            transaction,
            &target,
            activation.state_changed,
            activation.invalidation,
            activation.subscription_invalidation,
            subscription_credit,
        );
        for effect in activation.outputs {
            match effect {
                MountedEffect::Action(action) => {
                    transaction.parent = self.trace.record_event(
                        TraceRecordKind::RoutedActionCollected,
                        transaction.sequence,
                        transaction.parent,
                        Some(transaction.target_trace.clone()),
                        transaction.instant,
                        &transaction.target,
                        Some(&transaction.target),
                        transaction.origin,
                    );
                    transaction
                        .default_outputs
                        .push(CollectedRoutedOutput::Action {
                            action,
                            causal_parent: transaction.parent,
                            current_target: transaction.target.clone(),
                        });
                }
                effect => transaction
                    .mounted_work
                    .push((transaction.target.clone(), effect)),
            }
        }
        Ok(())
    }
}
