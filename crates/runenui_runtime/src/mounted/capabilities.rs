use runenui_core::{
    __runtime::WidgetBridgeError, SemanticContribution, SemanticContributionContext,
    SemanticContributionError, SemanticKey, WidgetActivation, WidgetActivationContext,
    WidgetInvalidation, WidgetTextInput,
};

use super::{
    CachedCapability, CachedSemanticContribution, MountedNodeId, apply_invalidation,
    node::state_is_corrupted, semantic::SemanticReconcileError, tree::MountedTree,
};

#[allow(
    clippy::struct_excessive_bools,
    reason = "activation reports independent state, subscription, primary-action, and overflow facts"
)]
pub(crate) struct MountedActivationOutput<Action> {
    pub(crate) invalidation: WidgetInvalidation,
    pub(crate) subscription_invalidation: bool,
    pub(crate) outputs: Vec<runenui_core::__runtime::MountedEffect<Action>>,
    pub(crate) state_changed: bool,
    pub(crate) overflowed: bool,
    pub(crate) remaining_outputs: usize,
}

enum SemanticEvaluation {
    Ready {
        contribution: SemanticContribution,
        ordered_keys: Vec<SemanticKey>,
    },
    Invalid(SemanticContributionError),
    StatePayloadMismatch,
}

impl<Action> MountedTree<Action> {
    /// Refreshes the input-facing capability cache after a compatible update.
    /// Reconciliation cannot assume that a widget remembered to invalidate
    /// interaction state when its enablement or text-input declaration changed.
    pub(crate) fn refresh_input_capabilities(
        &mut self,
        id: &MountedNodeId,
    ) -> Result<(WidgetActivation, WidgetTextInput), WidgetBridgeError> {
        let Some(node) = self.node_mut(id) else {
            return Ok((WidgetActivation::NONE, WidgetTextInput::NONE));
        };
        if state_is_corrupted(node) {
            node.integrity_failed = true;
            node.caches.activation = CachedCapability::StatePayloadMismatch;
            node.caches.text_input = CachedCapability::StatePayloadMismatch;
            return Err(WidgetBridgeError::StatePayloadMismatch);
        }
        let activation = node.widget.activation(&node.state);
        let text_input = node.widget.text_input(&node.state);
        if let (Ok(activation), Ok(text_input)) = (activation, text_input) {
            node.caches.activation = CachedCapability::Ready(activation);
            node.caches.text_input = CachedCapability::Ready(text_input);
            Ok((activation, text_input))
        } else {
            node.integrity_failed = true;
            node.caches.activation = CachedCapability::StatePayloadMismatch;
            node.caches.text_input = CachedCapability::StatePayloadMismatch;
            Err(WidgetBridgeError::StatePayloadMismatch)
        }
    }

    pub(crate) fn text_input_probe(
        &mut self,
        id: &MountedNodeId,
    ) -> Result<WidgetTextInput, WidgetBridgeError> {
        let Some(node) = self.node_mut(id) else {
            return Ok(WidgetTextInput::NONE);
        };
        if state_is_corrupted(node) {
            node.integrity_failed = true;
            node.caches.text_input = CachedCapability::StatePayloadMismatch;
            return Err(WidgetBridgeError::StatePayloadMismatch);
        }
        match node.caches.text_input {
            CachedCapability::Ready(value) => Ok(value),
            CachedCapability::StatePayloadMismatch => Err(WidgetBridgeError::StatePayloadMismatch),
            CachedCapability::Unresolved => match node.widget.text_input(&node.state) {
                Ok(value) => {
                    node.caches.text_input = CachedCapability::Ready(value);
                    Ok(value)
                }
                Err(error) => {
                    node.integrity_failed = true;
                    node.caches.text_input = CachedCapability::StatePayloadMismatch;
                    Err(error)
                }
            },
        }
    }

    pub(crate) fn activate(
        &mut self,
        id: &MountedNodeId,
        output_allowance: usize,
    ) -> Result<MountedActivationOutput<Action>, WidgetBridgeError> {
        let node = self
            .node_mut(id)
            .ok_or(WidgetBridgeError::StatePayloadMismatch)?;
        if state_is_corrupted(node) {
            return Err(WidgetBridgeError::StatePayloadMismatch);
        }
        let mut context = WidgetActivationContext::__runtime_new_bounded(output_allowance);
        let activation = node.widget.activate(&mut node.state, &mut context)?;
        let invalidation = context.__runtime_take_invalidation();
        let subscription_invalidation = context.__runtime_take_subscription_invalidation();
        apply_invalidation(node, invalidation);
        let mut outputs = context.__runtime_take_outputs();
        let state_changed = activation.state_changed();
        let action = activation.into_action();
        if let Some(action) = action
            && context.__runtime_reserve_output()
        {
            outputs.insert(0, runenui_core::__runtime::MountedEffect::Action(action));
        }
        let remaining_outputs = context
            .__runtime_remaining_outputs()
            .unwrap_or_else(|| unreachable!("activation context is bounded"));
        Ok(MountedActivationOutput {
            invalidation,
            subscription_invalidation,
            outputs,
            state_changed,
            overflowed: context.__runtime_overflowed(),
            remaining_outputs,
        })
    }

    pub(crate) fn activation(
        &mut self,
        id: &MountedNodeId,
    ) -> Result<WidgetActivation, WidgetBridgeError> {
        let Some(node) = self.node_mut(id) else {
            return Ok(WidgetActivation::NONE);
        };
        if state_is_corrupted(node) {
            node.integrity_failed = true;
            node.caches.activation = CachedCapability::StatePayloadMismatch;
            return Err(WidgetBridgeError::StatePayloadMismatch);
        }
        match node.caches.activation {
            CachedCapability::Ready(value) => Ok(value),
            CachedCapability::StatePayloadMismatch => Err(WidgetBridgeError::StatePayloadMismatch),
            CachedCapability::Unresolved => match node.widget.activation(&node.state) {
                Ok(value) => {
                    node.caches.activation = CachedCapability::Ready(value);
                    Ok(value)
                }
                Err(error) => {
                    node.integrity_failed = true;
                    node.caches.activation = CachedCapability::StatePayloadMismatch;
                    Err(error)
                }
            },
        }
    }

    pub(crate) fn activation_probe(
        &mut self,
        id: &MountedNodeId,
    ) -> Result<WidgetActivation, WidgetBridgeError> {
        let Some(node) = self.node_mut(id) else {
            return Ok(WidgetActivation::NONE);
        };
        if state_is_corrupted(node) {
            node.integrity_failed = true;
            node.caches.activation = CachedCapability::StatePayloadMismatch;
            return Err(WidgetBridgeError::StatePayloadMismatch);
        }
        match node.caches.activation {
            CachedCapability::Ready(value) => Ok(value),
            CachedCapability::StatePayloadMismatch => Err(WidgetBridgeError::StatePayloadMismatch),
            CachedCapability::Unresolved => match node.widget.activation(&node.state) {
                Ok(value) => Ok(value),
                Err(error) => {
                    node.integrity_failed = true;
                    node.caches.activation = CachedCapability::StatePayloadMismatch;
                    Err(error)
                }
            },
        }
    }

    pub(crate) fn ensure_semantics_capability(&mut self, id: &MountedNodeId) {
        self.ensure_semantics_capability_with_public_slot_limit(id, u64::from(u32::MAX) + 1);
    }

    #[cfg(test)]
    pub(crate) fn ensure_semantics_capability_with_public_slot_limit_for_test(
        &mut self,
        id: &MountedNodeId,
        public_slot_limit: u64,
    ) {
        self.ensure_semantics_capability_with_public_slot_limit(id, public_slot_limit);
    }

    fn ensure_semantics_capability_with_public_slot_limit(
        &mut self,
        id: &MountedNodeId,
        public_slot_limit: u64,
    ) {
        let direct_mounted_children = self.node(id).map_or(0, |node| node.children.len());
        let context = SemanticContributionContext::__runtime_new(direct_mounted_children);
        let evaluation = {
            let Some(node) = self.node_mut(id) else {
                return;
            };
            if !matches!(
                node.caches.semantics,
                CachedSemanticContribution::Unresolved
            ) {
                return;
            }
            if state_is_corrupted(node) {
                SemanticEvaluation::StatePayloadMismatch
            } else {
                node.widget.semantics(&node.state, context).map_or(
                    SemanticEvaluation::StatePayloadMismatch,
                    |contribution| match contribution.validate(context) {
                        Ok(validation) => SemanticEvaluation::Ready {
                            ordered_keys: validation.ordered_keys().to_vec(),
                            contribution,
                        },
                        Err(error) => SemanticEvaluation::Invalid(error),
                    },
                )
            }
        };

        match evaluation {
            SemanticEvaluation::Ready {
                contribution,
                ordered_keys,
            } => {
                self.commit_semantic_evaluation(id, contribution, &ordered_keys, public_slot_limit);
            }
            SemanticEvaluation::Invalid(error) => {
                self.withdraw_semantic_owner(id, CachedSemanticContribution::Invalid(error), false);
            }
            SemanticEvaluation::StatePayloadMismatch => {
                self.withdraw_semantic_owner(
                    id,
                    CachedSemanticContribution::StatePayloadMismatch,
                    true,
                );
            }
        }
    }

    fn commit_semantic_evaluation(
        &mut self,
        id: &MountedNodeId,
        contribution: SemanticContribution,
        ordered_keys: &[SemanticKey],
        public_slot_limit: u64,
    ) {
        let current = self
            .node(id)
            .map(|node| node.semantic_bindings.clone())
            .unwrap_or_default();
        let runtime = self.runtime.clone();
        let mut transaction = self.semantic_store.transaction();
        let owner_plan = match transaction.stage_owner(
            &runtime,
            id,
            &current,
            ordered_keys,
            public_slot_limit,
        ) {
            Ok(owner_plan) => owner_plan,
            Err(SemanticReconcileError::IdentityExhausted) => {
                self.withdraw_semantic_owner(
                    id,
                    CachedSemanticContribution::IdentityExhausted,
                    false,
                );
                return;
            }
            Err(SemanticReconcileError::Integrity(_)) => {
                self.withdraw_semantic_owner(
                    id,
                    CachedSemanticContribution::IndexIntegrityFailure,
                    true,
                );
                return;
            }
        };
        let plan = match transaction.finalize_fail_closed(&runtime) {
            Ok(plan) => plan,
            Err(SemanticReconcileError::IdentityExhausted) => {
                self.withdraw_semantic_owner(
                    id,
                    CachedSemanticContribution::IdentityExhausted,
                    false,
                );
                return;
            }
            Err(SemanticReconcileError::Integrity(_)) => {
                self.withdraw_semantic_owner(
                    id,
                    CachedSemanticContribution::IndexIntegrityFailure,
                    true,
                );
                return;
            }
        };
        let identity_exhausted = plan.identity_exhausted(owner_plan);
        let bindings = plan.bindings(owner_plan).to_vec();
        plan.commit();

        let node = self
            .node_mut(id)
            .unwrap_or_else(|| unreachable!("semantic owner remains live"));
        node.semantic_bindings = bindings;
        node.caches.semantics = if identity_exhausted {
            CachedSemanticContribution::IdentityExhausted
        } else {
            CachedSemanticContribution::Ready(contribution)
        };
    }

    fn withdraw_semantic_owner(
        &mut self,
        id: &MountedNodeId,
        cache: CachedSemanticContribution,
        mark_integrity_failed: bool,
    ) {
        let runtime = self.runtime.clone();
        let planned_purge_succeeded = self.try_commit_semantic_owner_purge(id, &runtime);

        let purge_failed = if planned_purge_succeeded {
            false
        } else {
            let bindings = self
                .node(id)
                .map(|node| node.semantic_bindings.clone())
                .unwrap_or_default();
            self.semantic_store
                .revoke_owner(&runtime, id, &bindings)
                .is_err()
        };

        let node = self
            .node_mut(id)
            .unwrap_or_else(|| unreachable!("semantic owner remains live"));
        node.semantic_bindings.clear();
        if mark_integrity_failed || purge_failed {
            node.integrity_failed = true;
        }
        node.caches.semantics = if purge_failed {
            CachedSemanticContribution::IndexIntegrityFailure
        } else {
            cache
        };
    }

    fn try_commit_semantic_owner_purge(
        &mut self,
        id: &MountedNodeId,
        runtime: &runenui_core::__runtime::RuntimeNamespace,
    ) -> bool {
        let mut transaction = self.semantic_store.transaction();
        let Ok(owner_plan) = transaction.stage_owner_purge(id, u64::from(u32::MAX) + 1) else {
            return false;
        };
        let Ok(plan) = transaction.finalize_fail_closed(runtime) else {
            return false;
        };
        debug_assert!(plan.bindings(owner_plan).is_empty());
        plan.commit();
        true
    }
}
