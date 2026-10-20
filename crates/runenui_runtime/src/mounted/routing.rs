//! Immutable mounted-route construction and checked event invocation.

use runenui_core::{
    __runtime::{EventContextOutput, WidgetBridgeError},
    CommandOrigin, EventPhase, LogicalPoint, LogicalSize, MonotonicInstant, PointerId,
    ScrollControlSnapshot, ShortcutBinding, UiEvent, WidgetEventOutput, WorkSequence,
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
    #[cfg(feature = "controlled-transaction-prototype")]
    fn take_controlled_invocation(
        &mut self,
        current: &MountedNodeId,
        phase: EventPhase,
        event: &UiEvent,
        allowance: usize,
    ) -> Option<EventInvocation<Action>> {
        use runenui_core::{CompositionEvent, ControlledNoticeKind, FocusEventKind};
        let kind = if event
            .as_focus()
            .is_some_and(|f| f.kind() == FocusEventKind::Out)
        {
            ControlledNoticeKind::FocusOut
        } else if matches!(event, UiEvent::Composition(CompositionEvent::Cancel(_))) {
            ControlledNoticeKind::CompositionCancel
        } else {
            return None;
        };
        let index = self
            .controlled_pending
            .iter()
            .position(|item| &item.owner == current && item.phase == phase && item.kind == kind)?;
        let action = self.controlled_pending.remove(index).action;
        Some(EventInvocation {
            widget: WidgetEventOutput::none(),
            output: EventContextOutput::__controlled_staged_action(action, allowance),
        })
    }

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

    pub(crate) fn shortcut_bindings(&self, owner: &MountedNodeId) -> Option<&[ShortcutBinding]> {
        self.node(owner)
            .map(|node| node.shortcut_bindings.as_slice())
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
        scroll_control: Option<ScrollControlSnapshot>,
        default_prevented: bool,
        propagation_stopped: bool,
        output_allowance: usize,
    ) -> Result<EventInvocation<Action>, WidgetBridgeError> {
        #[cfg(feature = "controlled-transaction-prototype")]
        if let Some(output) =
            self.take_controlled_invocation(current, phase, event, output_allowance)
        {
            return Ok(output);
        }
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
            scroll_control,
            // Composition lifecycle is runtime-owned. In particular, a
            // cancellation that protects an exact live owner cannot be
            // prevented or retargeted by widget code.
            !matches!(event, UiEvent::Composition(_)),
            default_prevented,
            propagation_stopped,
            output_allowance,
        )?;
        apply_invalidation(node, output.invalidation);
        Ok(EventInvocation { widget, output })
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn invoke_focus_event(
        &mut self,
        current: &MountedNodeId,
        original: &MountedNodeId,
        related: Option<&MountedNodeId>,
        event: &UiEvent,
        phase: EventPhase,
        origin: CommandOrigin,
        sequence: WorkSequence,
        instant: MonotonicInstant,
        scroll_control: Option<ScrollControlSnapshot>,
        propagation_stopped: bool,
        output_allowance: usize,
    ) -> Result<EventInvocation<Action>, WidgetBridgeError> {
        #[cfg(feature = "controlled-transaction-prototype")]
        if let Some(output) =
            self.take_controlled_invocation(current, phase, event, output_allowance)
        {
            return Ok(output);
        }
        let node = self
            .node_mut(current)
            .ok_or(WidgetBridgeError::StatePayloadMismatch)?;
        let (widget, output) = node.widget.event(
            &mut node.state,
            event,
            phase,
            original,
            current,
            related,
            origin,
            sequence,
            instant,
            scroll_control,
            false,
            false,
            propagation_stopped,
            output_allowance,
        )?;
        apply_invalidation(node, output.invalidation);
        Ok(EventInvocation { widget, output })
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn invoke_pointer_event(
        &mut self,
        current: &MountedNodeId,
        original: &MountedNodeId,
        related: Option<&MountedNodeId>,
        event: &UiEvent,
        phase: EventPhase,
        origin: CommandOrigin,
        sequence: WorkSequence,
        instant: MonotonicInstant,
        pointer_id: PointerId,
        physical_target: Option<&MountedNodeId>,
        physical_path: &[MountedNodeId],
        pointer_local_position: Option<LogicalPoint>,
        pointer_local_size: Option<LogicalSize>,
        scroll_control: Option<ScrollControlSnapshot>,
        default_cancelable: bool,
        default_prevented: bool,
        propagation_stopped: bool,
        output_allowance: usize,
    ) -> Result<EventInvocation<Action>, WidgetBridgeError> {
        let node = self
            .node_mut(current)
            .ok_or(WidgetBridgeError::StatePayloadMismatch)?;
        let (widget, output) = node.widget.pointer_event(
            &mut node.state,
            event,
            phase,
            original,
            current,
            related,
            origin,
            sequence,
            instant,
            pointer_id,
            physical_target,
            physical_path,
            pointer_local_position,
            pointer_local_size,
            scroll_control,
            default_cancelable,
            default_prevented,
            propagation_stopped,
            output_allowance,
        )?;
        apply_invalidation(node, output.invalidation);
        Ok(EventInvocation { widget, output })
    }
}
