use runenui_core::{
    __runtime::WidgetBridgeError, WidgetActivation, WidgetActivationContext, WidgetInvalidation,
};

use super::{
    CachedCapability, MountedNodeId, apply_invalidation, node::state_is_corrupted,
    tree::MountedTree,
};

pub(crate) struct MountedActivationOutput<Action> {
    pub(crate) invalidation: WidgetInvalidation,
    pub(crate) subscription_invalidation: bool,
    pub(crate) outputs: Vec<runenui_core::__runtime::MountedEffect<Action>>,
    pub(crate) primary_action: bool,
    pub(crate) state_changed: bool,
}

impl<Action> MountedTree<Action> {
    pub(crate) fn activate(
        &mut self,
        id: &MountedNodeId,
    ) -> Result<MountedActivationOutput<Action>, WidgetBridgeError> {
        let node = self
            .node_mut(id)
            .ok_or(WidgetBridgeError::StatePayloadMismatch)?;
        if state_is_corrupted(node) {
            return Err(WidgetBridgeError::StatePayloadMismatch);
        }
        let mut context = WidgetActivationContext::__runtime_new();
        let activation = node.widget.activate(&mut node.state, &mut context)?;
        let invalidation = context.__runtime_take_invalidation();
        let subscription_invalidation = context.__runtime_take_subscription_invalidation();
        apply_invalidation(node, invalidation);
        let mut outputs = context.__runtime_take_outputs();
        let state_changed = activation.state_changed();
        let action = activation.into_action();
        let primary_action = action.is_some();
        if let Some(action) = action {
            outputs.insert(0, runenui_core::__runtime::MountedEffect::Action(action));
        }
        Ok(MountedActivationOutput {
            invalidation,
            subscription_invalidation,
            outputs,
            primary_action,
            state_changed,
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

    pub(crate) fn ensure_layout_capabilities(&mut self, id: &MountedNodeId) {
        let Some(node) = self.node_mut(id) else {
            return;
        };
        if state_is_corrupted(node) {
            node.integrity_failed = true;
            node.caches.measurement = CachedCapability::StatePayloadMismatch;
            node.caches.child_layout = CachedCapability::StatePayloadMismatch;
            return;
        }
        if matches!(node.caches.measurement, CachedCapability::Unresolved) {
            let result = node.widget.measure(&node.state);
            cache_result(
                result,
                &mut node.caches.measurement,
                &mut node.integrity_failed,
            );
        }
        if matches!(node.caches.child_layout, CachedCapability::Unresolved) {
            let result = node.widget.child_layout(&node.state);
            cache_result(
                result,
                &mut node.caches.child_layout,
                &mut node.integrity_failed,
            );
        }
    }

    pub(crate) fn ensure_paint_capability(&mut self, id: &MountedNodeId) {
        let Some(node) = self.node_mut(id) else {
            return;
        };
        if state_is_corrupted(node) {
            node.integrity_failed = true;
            node.caches.paint = CachedCapability::StatePayloadMismatch;
        } else if matches!(node.caches.paint, CachedCapability::Unresolved) {
            let result = node.widget.paint(&node.state);
            cache_result(result, &mut node.caches.paint, &mut node.integrity_failed);
        }
    }

    pub(crate) fn ensure_semantics_capability(&mut self, id: &MountedNodeId) {
        let Some(node) = self.node_mut(id) else {
            return;
        };
        if state_is_corrupted(node) {
            node.integrity_failed = true;
            node.caches.semantics = CachedCapability::StatePayloadMismatch;
        } else if matches!(node.caches.semantics, CachedCapability::Unresolved) {
            let result = node.widget.semantics(&node.state);
            cache_result(
                result,
                &mut node.caches.semantics,
                &mut node.integrity_failed,
            );
        }
    }

    pub(crate) fn ensure_diagnostics_capability(&mut self, id: &MountedNodeId) {
        let Some(node) = self.node_mut(id) else {
            return;
        };
        if state_is_corrupted(node) {
            node.integrity_failed = true;
            node.caches.diagnostics = CachedCapability::StatePayloadMismatch;
        } else if matches!(node.caches.diagnostics, CachedCapability::Unresolved) {
            if let Ok(value) = node.widget.diagnostics(&node.state) {
                node.caches.diagnostics = CachedCapability::Ready(value);
            } else {
                node.integrity_failed = true;
                node.caches.diagnostics = CachedCapability::StatePayloadMismatch;
            }
        }
    }
}

fn cache_result<T>(
    result: Result<T, WidgetBridgeError>,
    cache: &mut CachedCapability<T>,
    integrity_failed: &mut bool,
) {
    if let Ok(value) = result {
        *cache = CachedCapability::Ready(value);
    } else {
        *integrity_failed = true;
        *cache = CachedCapability::StatePayloadMismatch;
    }
}
