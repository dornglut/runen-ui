use runenui_core::{__runtime::MountedEffect, HostProtocol, SemanticActionData, SemanticCommand};

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
        if matches!(
            command,
            SemanticCommand::Copy | SemanticCommand::Cut | SemanticCommand::Paste
        ) {
            transaction.parent = self.trace.record_event(
                TraceRecordKind::EditingDefaultUnavailable { command },
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
        if super::is_editing_command(command) {
            return self.apply_editing_default(transaction, command);
        }
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

    #[allow(clippy::too_many_lines)]
    fn apply_editing_default(
        &mut self,
        transaction: &mut RoutedTransaction<Action>,
        command: SemanticCommand,
    ) -> Result<(), TraceRoutedIntegrityFailure> {
        let target = transaction.target.clone();
        if !self.editing.has_owner(&target) || self.focus.focused_node() != Some(&target) {
            return Ok(());
        }
        if command == SemanticCommand::SetSelection {
            let Some(SemanticActionData::Selection(selection)) = transaction
                .semantic_target
                .as_ref()
                .and_then(runenui_core::SemanticActionTarget::data)
            else {
                return Ok(());
            };
            let Ok((snapshot, source)) = self.editing.caret_source(&target) else {
                return Ok(());
            };
            let Ok(map) = self
                .surface_publication
                .text_caret_map(&target, snapshot, &source)
            else {
                return Ok(());
            };
            if self
                .editing
                .set_selection(&target, *selection, &map)
                .is_err()
            {
                return Ok(());
            }
            transaction.invalidation |= runenui_core::WidgetInvalidation::SEMANTICS;
            self.tree.mark_runtime_semantic_product_dirty();
            return Ok(());
        }
        if matches!(
            command,
            SemanticCommand::DeleteBackward
                | SemanticCommand::DeleteForward
                | SemanticCommand::Undo
                | SemanticCommand::Redo
                | SemanticCommand::ReplaceSelection
        ) {
            transaction.consume_mandatory_default_command()?;
        }
        let namespace = self.tree.runtime_namespace();
        if command == SemanticCommand::ReplaceSelection {
            let Some(SemanticActionData::ReplacementText(text)) = transaction
                .semantic_target
                .as_ref()
                .and_then(runenui_core::SemanticActionTarget::data)
            else {
                return Ok(());
            };
            let prepared = match self
                .editing
                .prepare_replace_selection(&namespace, &target, text)
            {
                Ok(prepared) => prepared,
                Err(
                    crate::editing::EditPrepareError::MissingOwner
                    | crate::editing::EditPrepareError::Unavailable
                    | crate::editing::EditPrepareError::InvalidCoordinates,
                ) => return Ok(()),
                Err(
                    crate::editing::EditPrepareError::PendingCapacity
                    | crate::editing::EditPrepareError::RequestExhausted,
                ) => return Err(TraceRoutedIntegrityFailure::CommitInvariantFailure),
            };
            transaction.invalidation |= runenui_core::WidgetInvalidation::PAINT
                | runenui_core::WidgetInvalidation::SEMANTICS;
            self.tree.mark_runtime_semantic_product_dirty();
            transaction
                .default_outputs
                .push(CollectedRoutedOutput::EditAction {
                    action: prepared.action,
                    origin: prepared.origin,
                    causal_parent: transaction.parent,
                    current_target: target,
                });
            return Ok(());
        }
        let caret_map = if matches!(
            command,
            SemanticCommand::MoveBackward
                | SemanticCommand::MoveForward
                | SemanticCommand::ExtendBackward
                | SemanticCommand::ExtendForward
                | SemanticCommand::SelectAll
                | SemanticCommand::DeleteBackward
                | SemanticCommand::DeleteForward
        ) {
            let Ok((snapshot, source)) = self.editing.caret_source(&target) else {
                return Ok(());
            };
            let Ok(map) = self
                .surface_publication
                .text_caret_map(&target, snapshot, &source)
            else {
                return Ok(());
            };
            Some(map)
        } else {
            None
        };
        let prepared =
            match self
                .editing
                .prepare_command(&namespace, &target, command, caret_map.as_ref())
            {
                Ok(prepared) => prepared,
                Err(
                    crate::editing::EditPrepareError::MissingOwner
                    | crate::editing::EditPrepareError::Unavailable
                    | crate::editing::EditPrepareError::InvalidCoordinates,
                ) => return Ok(()),
                Err(
                    crate::editing::EditPrepareError::PendingCapacity
                    | crate::editing::EditPrepareError::RequestExhausted,
                ) => return Err(TraceRoutedIntegrityFailure::CommitInvariantFailure),
            };
        transaction.invalidation |=
            runenui_core::WidgetInvalidation::PAINT | runenui_core::WidgetInvalidation::SEMANTICS;
        self.tree.mark_runtime_semantic_product_dirty();
        if let Some(prepared) = prepared {
            transaction
                .default_outputs
                .push(CollectedRoutedOutput::EditAction {
                    action: prepared.action,
                    origin: prepared.origin,
                    causal_parent: transaction.parent,
                    current_target: target,
                });
        }
        Ok(())
    }

    fn semantic_default_target_rejection(
        &self,
        transaction: &RoutedTransaction<Action>,
        _command: SemanticCommand,
    ) -> Option<TraceSemanticActionRejection> {
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
