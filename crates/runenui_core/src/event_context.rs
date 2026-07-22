//! Borrowed transaction-scoped routed event context.

use core::{fmt, future::Future};

use crate::{
    CommandOrigin, EventPhase, MonotonicInstant, MountedNodeId, PointerId, SemanticCommand,
    SendTaskStartFailure, TimerEffect, WidgetInvalidation, WorkFamily, WorkKey, WorkSequence,
    effects::MountedEffect, widget_context::WidgetWorkCollector,
};

/// One action or delegated command in exact callback emission order.
#[doc(hidden)]
pub enum RoutedEventOutput<Action> {
    Action(Action),
    Command {
        target: MountedNodeId,
        command: SemanticCommand,
        origin: CommandOrigin,
    },
}

/// One staged current-node pointer-capture mutation request.
#[doc(hidden)]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PointerCaptureRequest {
    Capture {
        pointer_id: PointerId,
        target: MountedNodeId,
    },
    Release {
        pointer_id: PointerId,
        target: MountedNodeId,
    },
}

/// Collected provisional output returned to the runtime bridge.
#[doc(hidden)]
#[allow(
    clippy::struct_excessive_bools,
    reason = "each flag is an independent transaction outcome consumed by the runtime"
)]
pub struct EventContextOutput<Action> {
    pub ordered: Vec<RoutedEventOutput<Action>>,
    pub invalidation: WidgetInvalidation,
    pub subscription_invalidation: bool,
    pub mounted_work: Vec<MountedEffect<Action>>,
    pub pointer_capture: Option<PointerCaptureRequest>,
    pub propagation_stopped: bool,
    pub default_prevented: bool,
    pub overflowed: bool,
    pub remaining_outputs: usize,
}

/// Borrowed facts and provisional output surface for one routed callback.
#[allow(
    clippy::struct_excessive_bools,
    reason = "cancelability, flow control, coalescing, and overflow are independent protocol facts"
)]
pub struct EventContext<'a, Action> {
    phase: EventPhase,
    original_target: &'a MountedNodeId,
    current_target: &'a MountedNodeId,
    related_target: Option<&'a MountedNodeId>,
    origin: CommandOrigin,
    sequence: WorkSequence,
    instant: MonotonicInstant,
    pointer_id: Option<PointerId>,
    physical_target: Option<&'a MountedNodeId>,
    physical_path: &'a [MountedNodeId],
    pointer_capture: Option<PointerCaptureRequest>,
    default_cancelable: bool,
    default_prevented: bool,
    propagation_stopped: bool,
    invalidation: WidgetInvalidation,
    subscription_invalidation: bool,
    ordered: Vec<RoutedEventOutput<Action>>,
    mounted_work: WidgetWorkCollector<Action>,
    remaining_outputs: usize,
    overflowed: bool,
}

impl<Action> fmt::Debug for EventContext<'_, Action> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("EventContext")
            .field("phase", &self.phase)
            .field("original_target", self.original_target)
            .field("current_target", self.current_target)
            .field("related_target", &self.related_target)
            .field("origin", &self.origin)
            .field("sequence", &self.sequence)
            .field("instant", &self.instant)
            .field("pointer_id", &self.pointer_id)
            .field("physical_target", &self.physical_target)
            .field("physical_path", &self.physical_path)
            .field("default_cancelable", &self.default_cancelable)
            .field("default_prevented", &self.default_prevented)
            .field("propagation_stopped", &self.propagation_stopped)
            .finish_non_exhaustive()
    }
}

impl<'a, Action> EventContext<'a, Action> {
    #[must_use]
    pub const fn phase(&self) -> EventPhase {
        self.phase
    }

    #[must_use]
    pub const fn original_target(&self) -> &MountedNodeId {
        self.original_target
    }

    #[must_use]
    pub const fn current_target(&self) -> &MountedNodeId {
        self.current_target
    }

    #[must_use]
    pub const fn related_target(&self) -> Option<&MountedNodeId> {
        self.related_target
    }

    #[must_use]
    pub const fn command_origin(&self) -> CommandOrigin {
        self.origin
    }

    #[must_use]
    pub const fn sequence(&self) -> WorkSequence {
        self.sequence
    }

    #[must_use]
    pub const fn instant(&self) -> MonotonicInstant {
        self.instant
    }

    /// Returns the current pointer-stream identity for pointer event families.
    #[must_use]
    pub const fn pointer_id(&self) -> Option<PointerId> {
        self.pointer_id
    }

    /// Borrows the physical hit target independently of the routed target.
    #[must_use]
    pub const fn physical_target(&self) -> Option<&MountedNodeId> {
        self.physical_target
    }

    /// Borrows the immutable physical root-to-hit path.
    #[must_use]
    pub const fn physical_path(&self) -> &[MountedNodeId] {
        self.physical_path
    }

    #[must_use]
    pub const fn default_is_cancelable(&self) -> bool {
        self.default_cancelable
    }

    #[must_use]
    pub const fn default_is_prevented(&self) -> bool {
        self.default_prevented
    }

    #[must_use]
    pub const fn propagation_is_stopped(&self) -> bool {
        self.propagation_stopped
    }

    pub fn emit(&mut self, action: Action) {
        if self.reserve_output() {
            self.ordered.push(RoutedEventOutput::Action(action));
        }
    }

    pub fn emit_command(&mut self, command: SemanticCommand) {
        if self.reserve_output() {
            self.ordered.push(RoutedEventOutput::Command {
                target: self.current_target.clone(),
                command,
                origin: CommandOrigin::delegated(self.origin.source()),
            });
        }
    }

    /// Stages capture of the current pointer by the current routed node.
    pub fn capture_pointer(&mut self) {
        let Some(pointer_id) = self.pointer_id else {
            return;
        };
        if self.reserve_output() {
            self.pointer_capture = Some(PointerCaptureRequest::Capture {
                pointer_id,
                target: self.current_target.clone(),
            });
        }
    }

    /// Stages release of the current pointer when the current node owns capture.
    pub fn release_pointer_capture(&mut self) {
        let Some(pointer_id) = self.pointer_id else {
            return;
        };
        if self.reserve_output() {
            self.pointer_capture = Some(PointerCaptureRequest::Release {
                pointer_id,
                target: self.current_target.clone(),
            });
        }
    }

    pub fn invalidate(&mut self, invalidation: WidgetInvalidation) {
        self.invalidation |= invalidation;
    }

    pub const fn invalidate_subscriptions(&mut self) {
        if !self.subscription_invalidation {
            if self.remaining_outputs == 0 {
                self.overflowed = true;
                return;
            }
            self.remaining_outputs -= 1;
            self.subscription_invalidation = true;
        }
    }

    pub fn local_task(&mut self, future: impl Future<Output = Option<Action>> + 'static) {
        if self.reserve_output() {
            self.mounted_work.local_task(future);
        }
    }

    pub fn keyed_local_task(
        &mut self,
        key: WorkKey,
        future: impl Future<Output = Option<Action>> + 'static,
    ) {
        if self.reserve_output() {
            self.mounted_work.keyed_local_task(key, future);
        }
    }

    pub fn send_task<Output>(
        &mut self,
        future: impl Future<Output = Output> + Send + 'static,
        map: impl FnOnce(Output) -> Action + 'static,
    ) where
        Output: Send + 'static,
    {
        if self.reserve_output() {
            self.mounted_work.send_task(future, map);
        }
    }

    pub fn keyed_send_task<Output>(
        &mut self,
        key: WorkKey,
        future: impl Future<Output = Output> + Send + 'static,
        map: impl FnOnce(Output) -> Action + 'static,
    ) where
        Output: Send + 'static,
    {
        if self.reserve_output() {
            self.mounted_work.keyed_send_task(key, future, map);
        }
    }

    pub fn send_task_with_failure<Output>(
        &mut self,
        future: impl Future<Output = Output> + Send + 'static,
        map: impl FnOnce(Output) -> Action + 'static,
        start_failure: impl FnOnce(SendTaskStartFailure) -> Action + 'static,
    ) where
        Output: Send + 'static,
    {
        if self.reserve_output() {
            self.mounted_work
                .send_task_with_failure(future, map, start_failure);
        }
    }

    pub fn timer(&mut self, timer: TimerEffect<Action>) {
        if self.reserve_output() {
            self.mounted_work.timer(timer);
        }
    }

    pub fn cancel(&mut self, family: WorkFamily, key: WorkKey) {
        if self.reserve_output() {
            self.mounted_work.cancel(family, key);
        }
    }

    pub const fn stop_propagation(&mut self) {
        self.propagation_stopped = true;
    }

    pub const fn prevent_default(&mut self) {
        if self.default_cancelable {
            self.default_prevented = true;
        }
    }

    const fn reserve_output(&mut self) -> bool {
        if self.remaining_outputs == 0 {
            self.overflowed = true;
            false
        } else {
            self.remaining_outputs -= 1;
            true
        }
    }

    pub(crate) const fn mapped_child<ChildAction>(&self) -> EventContext<'a, ChildAction> {
        EventContext::new_with_pointer_facts(
            self.phase,
            self.original_target,
            self.current_target,
            self.related_target,
            self.origin,
            self.sequence,
            self.instant,
            self.pointer_id,
            self.physical_target,
            self.physical_path,
            self.default_cancelable,
            self.default_prevented,
            self.propagation_stopped,
            self.remaining_outputs,
        )
    }

    pub(crate) fn absorb_mapped<ChildAction: 'static>(
        &mut self,
        mut child: EventContextOutput<ChildAction>,
        mapper: &std::rc::Rc<dyn Fn(ChildAction) -> Action>,
    ) where
        Action: 'static,
    {
        self.invalidation |= child.invalidation;
        self.subscription_invalidation |= child.subscription_invalidation;
        self.pointer_capture = child.pointer_capture;
        self.default_prevented = child.default_prevented;
        self.propagation_stopped = child.propagation_stopped;
        self.overflowed |= child.overflowed;
        self.remaining_outputs = child.remaining_outputs;
        for output in child.ordered.drain(..) {
            self.ordered.push(match output {
                RoutedEventOutput::Action(action) => RoutedEventOutput::Action(mapper(action)),
                RoutedEventOutput::Command {
                    target,
                    command,
                    origin,
                } => RoutedEventOutput::Command {
                    target,
                    command,
                    origin,
                },
            });
        }
        for output in child.mounted_work.drain(..) {
            self.mounted_work
                .push_output(crate::widget_mapping::map_output(output, mapper));
        }
    }

    #[allow(clippy::too_many_arguments)]
    #[must_use]
    pub(crate) const fn new(
        phase: EventPhase,
        original_target: &'a MountedNodeId,
        current_target: &'a MountedNodeId,
        related_target: Option<&'a MountedNodeId>,
        origin: CommandOrigin,
        sequence: WorkSequence,
        instant: MonotonicInstant,
        default_cancelable: bool,
        default_prevented: bool,
        propagation_stopped: bool,
        output_allowance: usize,
    ) -> Self {
        Self::new_with_pointer_facts(
            phase,
            original_target,
            current_target,
            related_target,
            origin,
            sequence,
            instant,
            None,
            None,
            &[],
            default_cancelable,
            default_prevented,
            propagation_stopped,
            output_allowance,
        )
    }

    #[allow(clippy::too_many_arguments)]
    #[must_use]
    pub(crate) const fn new_pointer(
        phase: EventPhase,
        original_target: &'a MountedNodeId,
        current_target: &'a MountedNodeId,
        related_target: Option<&'a MountedNodeId>,
        origin: CommandOrigin,
        sequence: WorkSequence,
        instant: MonotonicInstant,
        pointer_id: PointerId,
        physical_target: Option<&'a MountedNodeId>,
        physical_path: &'a [MountedNodeId],
        default_cancelable: bool,
        default_prevented: bool,
        propagation_stopped: bool,
        output_allowance: usize,
    ) -> Self {
        Self::new_with_pointer_facts(
            phase,
            original_target,
            current_target,
            related_target,
            origin,
            sequence,
            instant,
            Some(pointer_id),
            physical_target,
            physical_path,
            default_cancelable,
            default_prevented,
            propagation_stopped,
            output_allowance,
        )
    }

    #[allow(clippy::too_many_arguments)]
    const fn new_with_pointer_facts(
        phase: EventPhase,
        original_target: &'a MountedNodeId,
        current_target: &'a MountedNodeId,
        related_target: Option<&'a MountedNodeId>,
        origin: CommandOrigin,
        sequence: WorkSequence,
        instant: MonotonicInstant,
        pointer_id: Option<PointerId>,
        physical_target: Option<&'a MountedNodeId>,
        physical_path: &'a [MountedNodeId],
        default_cancelable: bool,
        default_prevented: bool,
        propagation_stopped: bool,
        output_allowance: usize,
    ) -> Self {
        Self {
            phase,
            original_target,
            current_target,
            related_target,
            origin,
            sequence,
            instant,
            pointer_id,
            physical_target,
            physical_path,
            pointer_capture: None,
            default_cancelable,
            default_prevented,
            propagation_stopped,
            invalidation: WidgetInvalidation::NONE,
            subscription_invalidation: false,
            ordered: Vec::new(),
            mounted_work: WidgetWorkCollector::new(),
            remaining_outputs: output_allowance,
            overflowed: false,
        }
    }

    #[must_use]
    pub(crate) fn into_output(mut self) -> EventContextOutput<Action> {
        EventContextOutput {
            ordered: self.ordered,
            invalidation: self.invalidation,
            subscription_invalidation: self.subscription_invalidation,
            mounted_work: self.mounted_work.take_outputs(),
            pointer_capture: self.pointer_capture,
            propagation_stopped: self.propagation_stopped,
            default_prevented: self.default_prevented,
            overflowed: self.overflowed,
            remaining_outputs: self.remaining_outputs,
        }
    }
}

#[cfg(test)]
mod tests {
    use core::num::NonZeroU64;
    use std::rc::Rc;

    use crate::{
        __runtime::{MountedEffect, PointerCaptureRequest, RoutedEventOutput, RuntimeNamespace},
        CommandDerivation, CommandOrigin, EventPhase, EventSource, MonotonicInstant, PointerId,
        SemanticCommand, WidgetInvalidation, WorkSequence,
    };

    use super::EventContext;

    fn sequence(value: u64) -> WorkSequence {
        WorkSequence::__runtime_new(
            NonZeroU64::new(value).unwrap_or_else(|| unreachable!("test sequences are non-zero")),
        )
    }

    #[test]
    fn context_facts_and_collected_outputs_are_independent_and_ordered() {
        let namespace = RuntimeNamespace::__runtime_new();
        let original = namespace.__runtime_mounted_id(1, 1);
        let current = namespace.__runtime_mounted_id(2, 1);
        let origin = CommandOrigin::automation();
        let mut context = EventContext::new(
            EventPhase::Bubble,
            &original,
            &current,
            None,
            origin,
            sequence(7),
            MonotonicInstant::__runtime_from_nanos(11),
            true,
            false,
            false,
            4,
        );
        assert_eq!(context.phase(), EventPhase::Bubble);
        assert_eq!(context.original_target(), &original);
        assert_eq!(context.current_target(), &current);
        assert_eq!(context.related_target(), None);
        assert_eq!(context.command_origin(), origin);
        assert_eq!(context.sequence().get(), 7);
        assert_eq!(context.instant().as_nanos(), 11);
        assert_eq!(context.pointer_id(), None);
        assert_eq!(context.physical_target(), None);
        assert!(context.physical_path().is_empty());
        assert!(context.default_is_cancelable());
        assert!(!context.default_is_prevented());
        assert!(!context.propagation_is_stopped());

        context.emit(String::from("first"));
        context.emit_command(SemanticCommand::OpenMenu);
        context.invalidate(WidgetInvalidation::PAINT);
        context.invalidate_subscriptions();
        context.local_task(async { Some(String::from("later")) });
        context.stop_propagation();
        context.prevent_default();
        let output = context.into_output();

        assert_eq!(output.ordered.len(), 2);
        assert!(matches!(&output.ordered[0], RoutedEventOutput::Action(value) if value == "first"));
        assert!(matches!(
            &output.ordered[1],
            RoutedEventOutput::Command { target, command: SemanticCommand::OpenMenu, origin }
                if target == &current
                    && origin.source() == EventSource::Automation
                    && origin.derivation() == CommandDerivation::Delegated
        ));
        assert!(output.invalidation.contains(WidgetInvalidation::PAINT));
        assert!(output.subscription_invalidation);
        assert!(matches!(
            output.mounted_work.as_slice(),
            [MountedEffect::LocalTask(_)]
        ));
        assert_eq!(output.pointer_capture, None);
        assert!(output.propagation_stopped);
        assert!(output.default_prevented);
        assert!(!output.overflowed);
        assert_eq!(output.remaining_outputs, 0);
    }

    #[test]
    fn pointer_context_preserves_physical_facts_and_last_capture_request() {
        let namespace = RuntimeNamespace::__runtime_new();
        let root = namespace.__runtime_mounted_id(0, 1);
        let target = namespace.__runtime_mounted_id(1, 1);
        let path = [root, target.clone()];
        let pointer_id =
            PointerId::new(7).unwrap_or_else(|| unreachable!("test pointer is non-zero"));
        let mut context = EventContext::<()>::new_pointer(
            EventPhase::Target,
            &target,
            &target,
            None,
            CommandOrigin::__runtime_pointer(),
            sequence(2),
            MonotonicInstant::ZERO,
            pointer_id,
            Some(&target),
            &path,
            true,
            false,
            false,
            2,
        );

        assert_eq!(context.pointer_id(), Some(pointer_id));
        assert_eq!(context.physical_target(), Some(&target));
        assert_eq!(context.physical_path(), path.as_slice());
        context.capture_pointer();
        context.release_pointer_capture();
        let output = context.into_output();
        assert!(matches!(
            output.pointer_capture,
            Some(PointerCaptureRequest::Release {
                pointer_id: requested,
                target: requested_target,
            }) if requested == pointer_id && requested_target == target
        ));
        assert_eq!(output.remaining_outputs, 0);
    }

    #[test]
    fn mapped_context_preserves_controls_and_maps_owned_outputs_without_clone_bounds() {
        struct NonClone(u8);

        let namespace = RuntimeNamespace::__runtime_new();
        let target = namespace.__runtime_mounted_id(0, 1);
        let mut parent = EventContext::<String>::new(
            EventPhase::Target,
            &target,
            &target,
            None,
            CommandOrigin::controller(),
            sequence(1),
            MonotonicInstant::ZERO,
            true,
            false,
            false,
            3,
        );
        let mut child = parent.mapped_child::<NonClone>();
        child.emit(NonClone(9));
        child.emit_command(SemanticCommand::CancelOrBack);
        child.invalidate(WidgetInvalidation::SEMANTICS);
        child.prevent_default();
        let mapper: Rc<dyn Fn(NonClone) -> String> = Rc::new(|value| value.0.to_string());
        parent.absorb_mapped(child.into_output(), &mapper);
        let output = parent.into_output();

        assert!(matches!(&output.ordered[0], RoutedEventOutput::Action(value) if value == "9"));
        assert!(matches!(
            &output.ordered[1],
            RoutedEventOutput::Command { target: delegated, origin, .. }
                if delegated == &target
                    && origin.source() == EventSource::Controller
                    && origin.derivation() == CommandDerivation::Delegated
        ));
        assert!(output.invalidation.contains(WidgetInvalidation::SEMANTICS));
        assert!(output.default_prevented);
        assert_eq!(output.remaining_outputs, 1);
    }

    #[test]
    fn bounded_context_reports_overflow_without_accepting_excess_output() {
        let namespace = RuntimeNamespace::__runtime_new();
        let target = namespace.__runtime_mounted_id(0, 1);
        let mut context = EventContext::<u8>::new(
            EventPhase::Target,
            &target,
            &target,
            None,
            CommandOrigin::programmatic(),
            sequence(1),
            MonotonicInstant::ZERO,
            true,
            false,
            false,
            1,
        );
        context.emit(1);
        context.emit(2);
        let output = context.into_output();
        assert_eq!(output.ordered.len(), 1);
        assert!(output.overflowed);
        assert_eq!(output.remaining_outputs, 0);
    }
}
