//! Immutable mounted-route construction and checked event invocation.

use runenui_core::{
    __runtime::{EventContextOutput, WidgetBridgeError},
    CommandOrigin, EventPhase, MonotonicInstant, UiEvent, WidgetEventOutput, WorkSequence,
};

use super::{
    MountedNodeId, MountedTree, TargetStatus, apply_invalidation, node::state_is_corrupted,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum RouteBuildError {
    Target(TargetStatus),
    BrokenTopology,
    BridgeMismatch,
}

pub(crate) struct EventInvocation<Action> {
    pub(crate) widget: WidgetEventOutput,
    pub(crate) output: EventContextOutput<Action>,
}

impl<Action> MountedTree<Action> {
    pub(crate) fn event_route(
        &self,
        target: &MountedNodeId,
    ) -> Result<Vec<MountedNodeId>, RouteBuildError> {
        let status = self.target_status(target);
        if status != TargetStatus::Live {
            return Err(RouteBuildError::Target(status));
        }
        let mut route = Vec::new();
        let mut current = target.clone();
        loop {
            let node = self.node(&current).ok_or(RouteBuildError::BrokenTopology)?;
            route.push(current);
            let Some(parent) = node.parent.clone() else {
                break;
            };
            if route.len() > self.live_count() {
                return Err(RouteBuildError::BrokenTopology);
            }
            current = parent;
        }
        route.reverse();
        if route.first() != self.root.as_ref() || route.last() != Some(target) {
            return Err(RouteBuildError::BrokenTopology);
        }
        Ok(route)
    }

    pub(crate) fn preflight_event_bridges(
        &self,
        route: &[MountedNodeId],
    ) -> Result<(), RouteBuildError> {
        for id in route {
            let node = self.node(id).ok_or(RouteBuildError::BrokenTopology)?;
            if state_is_corrupted(node) || !node.widget.event_bridge_matches(&node.state) {
                return Err(RouteBuildError::BridgeMismatch);
            }
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn invoke_event(
        &mut self,
        current: &MountedNodeId,
        original: &MountedNodeId,
        event: &UiEvent,
        phase: EventPhase,
        origin: CommandOrigin,
        sequence: WorkSequence,
        instant: MonotonicInstant,
        default_prevented: bool,
        propagation_stopped: bool,
        output_allowance: usize,
    ) -> Result<EventInvocation<Action>, WidgetBridgeError> {
        let node = self
            .node_mut(current)
            .ok_or(WidgetBridgeError::StatePayloadMismatch)?;
        let (widget, output) = node.widget.event(
            &mut node.state,
            event,
            phase,
            original,
            current,
            None,
            origin,
            sequence,
            instant,
            true,
            default_prevented,
            propagation_stopped,
            output_allowance,
        )?;
        apply_invalidation(node, output.invalidation);
        Ok(EventInvocation { widget, output })
    }
}
