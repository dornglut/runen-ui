use runenui_core::{__runtime::MountedEffect, HostProtocol, SemanticCommand};

use super::{
    super::{CollectedRoutedOutput, Runtime, ingress::trace_semantic_action_rejection},
    transaction::RoutedTransaction,
};
use crate::{TraceRecordKind, TraceRoutedIntegrityFailure, TraceSemanticActionRejection};

impl<State, Action, Protocol: HostProtocol> Runtime<State, Action, Protocol> {
    pub(super) fn apply_semantic_default(
        &mut self,
        transaction: &mut RoutedTransaction<Action>,
        command: SemanticCommand,
    ) -> Result<(), TraceRoutedIntegrityFailure> {
        self.commit_pending_modality(transaction);
        if transaction.default_prevented {
            transaction.parent = self.trace.record_event(
                TraceRecordKind::SemanticDefaultSuppressed { command },
                transaction.sequence,
                transaction.parent,
                Some(transaction.target_trace.clone()),
                transaction.instant,
                &transaction.target,
                Some(&transaction.target),
                transaction.origin,
            );
            return Ok(());
        }
        if let Some(outcome) = self.semantic_default_target_rejection(transaction, command) {
            transaction.parent = self.trace.record_event(
                TraceRecordKind::SemanticDefaultTargetInvalidated { command, outcome },
                transaction.sequence,
                transaction.parent,
                Some(transaction.target_trace.clone()),
                transaction.instant,
                &transaction.target,
                Some(&transaction.target),
                transaction.origin,
            );
            return Ok(());
        }
        transaction.parent = self.trace.record_event(
            TraceRecordKind::SemanticDefaultApplied { command },
            transaction.sequence,
            transaction.parent,
            Some(transaction.target_trace.clone()),
            transaction.instant,
            &transaction.target,
            Some(&transaction.target),
            transaction.origin,
        );
        if command != SemanticCommand::Activate {
            return self.apply_focus_default(transaction, command);
        }
        transaction.failure_current_target = Some(transaction.target.clone());
        #[cfg(feature = "internal-test-seams")]
        if self.routed_semantic_default_failure_for_test {
            return Err(TraceRoutedIntegrityFailure::SemanticDefaultFailure);
        }
        self.invoke_activation_default(transaction)
    }

    fn semantic_default_target_rejection(
        &self,
        transaction: &RoutedTransaction<Action>,
        command: SemanticCommand,
    ) -> Option<TraceSemanticActionRejection> {
        if !matches!(
            command,
            SemanticCommand::Activate | SemanticCommand::RequestFocus
        ) {
            return None;
        }
        let semantic_target = transaction.semantic_target.as_ref()?;
        match self.revalidate_semantic_action_target(semantic_target) {
            Ok(owner) if owner == transaction.target => None,
            Ok(_) => Some(TraceSemanticActionRejection::OwnerChanged),
            Err(kind) => Some(trace_semantic_action_rejection(kind)),
        }
    }

    fn invoke_activation_default(
        &mut self,
        transaction: &mut RoutedTransaction<Action>,
    ) -> Result<(), TraceRoutedIntegrityFailure> {
        let activation = self
            .tree
            .activation_probe(&transaction.target)
            .map_err(|_| TraceRoutedIntegrityFailure::SemanticDefaultFailure)?;
        let requires_actionable = transaction
            .semantic_target
            .as_ref()
            .is_none_or(|target| target.semantic_key().is_primary());
        if !activation.enabled() || (requires_actionable && !activation.is_actionable()) {
            return Ok(());
        }
        let target = transaction.target.clone();
        let subscription_credit = transaction.subscription_credit(&target);
        let activation = self
            .tree
            .activate(
                &target,
                transaction.output_allowance(&target),
                transaction.semantic_target.clone(),
            )
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
