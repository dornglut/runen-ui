//! Borrowed transaction-scoped routed event context.

use core::{fmt, future::Future};

use crate::{
    ApplicationCommandDisposition, ApplicationCommandId, CommandOrigin, DragDropEvent,
    DragDropPhase, ElementId, EventPhase, LogicalPoint, LogicalSize, MonotonicInstant,
    MountedNodeId, PointerId, ScrollControlSnapshot, SemanticCommand, SendTaskStartFailure,
    TimerEffect, WidgetInvalidation, WorkFamily, WorkKey, WorkSequence, effects::MountedEffect,
    widget_context::WidgetOutputCollector,
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
    /// One same-mounted-subtree lookup, bound to an exact live descendant during
    /// collection before the canonical routed command FIFO is committed.
    DescendantScroll {
        owner: MountedNodeId,
        authored_id: ElementId,
        origin: CommandOrigin,
    },
    ApplicationCommand {
        target: MountedNodeId,
        command: ApplicationCommandId,
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
    pub application_command_disposition: Option<ApplicationCommandDisposition>,
    pub invalidation: WidgetInvalidation,
    pub subscription_invalidation: bool,
    pub mounted_work: Vec<MountedEffect<Action>>,
    pub pointer_capture: Vec<PointerCaptureRequest>,
    pub propagation_stopped: bool,
    pub default_prevented: bool,
    pub host_input_claimed: bool,
    pub accepted_drag_drop: bool,
    pub overflowed: bool,
    pub producer_violation: Option<crate::ProducerContractViolation>,
    pub work_start_used: [usize; 5],
    pub remaining_outputs: usize,
    pub used_outputs: usize,
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
    drag_drop: Option<DragDropEvent>,
    accepted_drag_drop: bool,
    pointer_id: Option<PointerId>,
    physical_target: Option<&'a MountedNodeId>,
    physical_path: &'a [MountedNodeId],
    pointer_local_position: Option<LogicalPoint>,
    pointer_local_size: Option<LogicalSize>,
    scroll_control: Option<ScrollControlSnapshot>,
    pointer_capture: Vec<PointerCaptureRequest>,
    default_cancelable: bool,
    default_prevented: bool,
    propagation_stopped: bool,
    host_input_claimed: bool,
    application_command_disposition: Option<ApplicationCommandDisposition>,
    invalidation: WidgetInvalidation,
    subscription_invalidation: bool,
    ordered: Vec<RoutedEventOutput<Action>>,
    mounted_work: WidgetOutputCollector<Action>,
    remaining_outputs: usize,
    used_outputs: usize,
    overflowed: bool,
    producer_violation: Option<crate::ProducerContractViolation>,
    output_limit: usize,
    work_start_declared: [usize; 5],
    work_start_used: [usize; 5],
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
            .field("drag_drop", &self.drag_drop)
            .field("pointer_id", &self.pointer_id)
            .field("physical_target", &self.physical_target)
            .field("physical_path", &self.physical_path)
            .field("pointer_local_position", &self.pointer_local_position)
            .field("pointer_local_size", &self.pointer_local_size)
            .field("default_cancelable", &self.default_cancelable)
            .field("default_prevented", &self.default_prevented)
            .field("propagation_stopped", &self.propagation_stopped)
            .field("host_input_claimed", &self.host_input_claimed)
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

    /// Requests admission of the current hover/drop offer from its physical hit target.
    /// This never exposes payload contents or native host handles.
    /// # Errors
    /// Returns the first sticky declaration violation before staging a request.
    pub fn try_accept_drag_drop(&mut self) -> Result<(), crate::ProducerContractViolation> {
        if let Some(cause) = self.producer_violation {
            return Err(cause);
        }

        let Some(event) = self.drag_drop else {
            return Ok(());
        };
        if self.phase != EventPhase::Target
            || self.accepted_drag_drop
            || !matches!(event.phase(), DragDropPhase::Hover | DragDropPhase::Drop)
        {
            return Ok(());
        }
        self.try_reserve()?;
        self.accepted_drag_drop = true;
        Ok(())
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

    /// Returns the current pointer position in the current routed target's exact
    /// displayed owner-local coordinate space.
    ///
    /// Non-pointer callbacks, unavailable retained geometry, and singular
    /// transforms return `None`. Captured pointer callbacks may return points
    /// outside the owner's local bounds.
    #[must_use]
    pub const fn pointer_local_position(&self) -> Option<LogicalPoint> {
        self.pointer_local_position
    }

    /// Final displayed owner-local extent from the same retained input publication
    /// as `pointer_local_position`. Unavailable or retired geometry yields None.
    #[must_use]
    pub const fn pointer_local_size(&self) -> Option<LogicalSize> {
        self.pointer_local_size
    }

    /// Returns the runtime-derived bound scroll snapshot for the current routed node.
    #[must_use]
    pub const fn scroll_control_snapshot(&self) -> Option<ScrollControlSnapshot> {
        self.scroll_control
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

    /// Requests standard #344 `ScrollIntoView` for one uniquely mounted, enabled
    /// descendant of the routed owner. Runtime resolves its exact generation
    /// *within this event transaction*; absent, ambiguous or disabled children
    /// produce no command. This never grants focus, stores scroll offsets, or
    /// retargets an authored ID after the command reaches the FIFO.
    ///
    /// A child mounted by a later application action is not eligible here.
    /// # Errors
    /// Returns the owned authored identity before resolution on declaration failure.
    pub fn try_emit_scroll_into_view_descendant(
        &mut self,
        authored_id: ElementId,
    ) -> Result<(), crate::EmitError<ElementId>> {
        if let Err(cause) = self.try_reserve() {
            return Err(crate::EmitError::new(authored_id, cause));
        }
        self.ordered.push(RoutedEventOutput::DescendantScroll {
            owner: self.current_target.clone(),
            authored_id,
            origin: CommandOrigin::delegated(self.origin.source()),
        });
        Ok(())
    }

    /// Consumes the current routed application command with one typed scope disposition.
    /// Capture-phase callbacks cannot consume contextual commands because scope resolution
    /// proceeds from the exact target outward.
    pub const fn consume_application_command(
        &mut self,
        disposition: ApplicationCommandDisposition,
    ) {
        if matches!(self.phase, EventPhase::Capture)
            || self.application_command_disposition.is_some()
        {
            return;
        }
        self.application_command_disposition = Some(disposition);
        self.propagation_stopped = true;
    }

    /// Stages capture of the current pointer by the current routed node.
    /// # Errors
    /// Returns the first sticky declaration violation before staging a request.
    pub fn try_capture_pointer(&mut self) -> Result<(), crate::ProducerContractViolation> {
        if let Some(cause) = self.producer_violation {
            return Err(cause);
        }

        let Some(pointer_id) = self.pointer_id else {
            return Ok(());
        };
        self.try_reserve()?;
        self.pointer_capture.push(PointerCaptureRequest::Capture {
            pointer_id,
            target: self.current_target.clone(),
        });
        Ok(())
    }

    /// Stages release of the current pointer when the current node owns capture.
    /// # Errors
    /// Returns the first sticky declaration violation before staging a request.
    pub fn try_release_pointer_capture(&mut self) -> Result<(), crate::ProducerContractViolation> {
        if let Some(cause) = self.producer_violation {
            return Err(cause);
        }

        let Some(pointer_id) = self.pointer_id else {
            return Ok(());
        };
        self.try_reserve()?;
        self.pointer_capture.push(PointerCaptureRequest::Release {
            pointer_id,
            target: self.current_target.clone(),
        });
        Ok(())
    }

    pub fn invalidate(&mut self, invalidation: WidgetInvalidation) {
        self.invalidation |= invalidation;
    }

    /// Explicitly claims this routed input against a concurrently embedded host.
    ///
    /// This independent transaction-local claim does not prevent `RunenUI` defaults,
    /// stop propagation, invoke a gameplay callback or allocate queued work.
    /// It becomes authoritative only after the enclosing routed transaction commits.
    pub const fn claim_host_input(&mut self) {
        self.host_input_claimed = true;
    }

    /// Reports whether this context has an explicit current-event host claim.
    #[must_use]
    pub const fn host_input_is_claimed(&self) -> bool {
        self.host_input_claimed
    }

    pub const fn stop_propagation(&mut self) {
        self.propagation_stopped = true;
    }

    pub const fn prevent_default(&mut self) {
        if self.default_cancelable {
            self.default_prevented = true;
        }
    }

    pub(crate) const fn mapped_child<ChildAction>(&self) -> EventContext<'a, ChildAction> {
        let mut child = EventContext::new_with_pointer_facts(
            self.phase,
            self.original_target,
            self.current_target,
            self.related_target,
            self.origin,
            self.sequence,
            self.instant,
            self.drag_drop,
            self.pointer_id,
            self.physical_target,
            self.physical_path,
            self.pointer_local_position,
            self.pointer_local_size,
            self.scroll_control,
            self.default_cancelable,
            self.default_prevented,
            self.propagation_stopped,
            self.remaining_outputs,
        );
        child.host_input_claimed = self.host_input_claimed;
        child.producer_violation = self.producer_violation;
        child.output_limit = self.output_limit;
        child.work_start_declared = self.work_start_declared;
        child.work_start_used = self.work_start_used;
        child
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
        self.pointer_capture.append(&mut child.pointer_capture);
        self.accepted_drag_drop |= child.accepted_drag_drop;
        self.default_prevented = child.default_prevented;
        self.propagation_stopped = child.propagation_stopped;
        self.host_input_claimed |= child.host_input_claimed;
        if child.application_command_disposition.is_some() {
            self.application_command_disposition = child.application_command_disposition;
        }
        self.work_start_used = child.work_start_used;
        self.overflowed |= child.overflowed;
        self.producer_violation = self.producer_violation.or(child.producer_violation);
        self.remaining_outputs = child.remaining_outputs;
        self.used_outputs += child.used_outputs;
        if self.producer_violation.is_some() {
            return;
        }
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
                RoutedEventOutput::DescendantScroll {
                    owner,
                    authored_id,
                    origin,
                } => RoutedEventOutput::DescendantScroll {
                    owner,
                    authored_id,
                    origin,
                },
                RoutedEventOutput::ApplicationCommand {
                    target,
                    command,
                    origin,
                } => RoutedEventOutput::ApplicationCommand {
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

    #[cfg(test)]
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
        drag_drop: Option<DragDropEvent>,
        default_cancelable: bool,
        default_prevented: bool,
        propagation_stopped: bool,
        output_allowance: usize,
    ) -> Self {
        Self::new_with_scroll_control(
            phase,
            original_target,
            current_target,
            related_target,
            origin,
            sequence,
            instant,
            drag_drop,
            None,
            default_cancelable,
            default_prevented,
            propagation_stopped,
            output_allowance,
        )
    }

    #[cfg(test)]
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
        drag_drop: Option<DragDropEvent>,
        pointer_id: PointerId,
        physical_target: Option<&'a MountedNodeId>,
        physical_path: &'a [MountedNodeId],
        default_cancelable: bool,
        default_prevented: bool,
        propagation_stopped: bool,
        output_allowance: usize,
    ) -> Self {
        Self::new_pointer_with_scroll_control(
            phase,
            original_target,
            current_target,
            related_target,
            origin,
            sequence,
            instant,
            drag_drop,
            pointer_id,
            physical_target,
            physical_path,
            None,
            None,
            None,
            default_cancelable,
            default_prevented,
            propagation_stopped,
            output_allowance,
        )
    }

    #[allow(clippy::too_many_arguments)]
    #[must_use]
    pub(crate) const fn new_with_scroll_control(
        phase: EventPhase,
        original_target: &'a MountedNodeId,
        current_target: &'a MountedNodeId,
        related_target: Option<&'a MountedNodeId>,
        origin: CommandOrigin,
        sequence: WorkSequence,
        instant: MonotonicInstant,
        drag_drop: Option<DragDropEvent>,
        scroll_control: Option<ScrollControlSnapshot>,
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
            drag_drop,
            None,
            None,
            &[],
            None,
            None,
            scroll_control,
            default_cancelable,
            default_prevented,
            propagation_stopped,
            output_allowance,
        )
    }

    #[allow(clippy::too_many_arguments)]
    #[must_use]
    pub(crate) const fn new_pointer_with_scroll_control(
        phase: EventPhase,
        original_target: &'a MountedNodeId,
        current_target: &'a MountedNodeId,
        related_target: Option<&'a MountedNodeId>,
        origin: CommandOrigin,
        sequence: WorkSequence,
        instant: MonotonicInstant,
        drag_drop: Option<DragDropEvent>,
        pointer_id: PointerId,
        physical_target: Option<&'a MountedNodeId>,
        physical_path: &'a [MountedNodeId],
        pointer_local_position: Option<LogicalPoint>,
        pointer_local_size: Option<LogicalSize>,
        scroll_control: Option<ScrollControlSnapshot>,
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
            drag_drop,
            Some(pointer_id),
            physical_target,
            physical_path,
            pointer_local_position,
            pointer_local_size,
            scroll_control,
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
        drag_drop: Option<DragDropEvent>,
        pointer_id: Option<PointerId>,
        physical_target: Option<&'a MountedNodeId>,
        physical_path: &'a [MountedNodeId],
        pointer_local_position: Option<LogicalPoint>,
        pointer_local_size: Option<LogicalSize>,
        scroll_control: Option<ScrollControlSnapshot>,
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
            drag_drop,
            accepted_drag_drop: false,
            pointer_id,
            physical_target,
            physical_path,
            pointer_local_position,
            pointer_local_size,
            scroll_control,
            pointer_capture: Vec::new(),
            default_cancelable,
            default_prevented,
            propagation_stopped,
            host_input_claimed: false,
            application_command_disposition: None,
            invalidation: WidgetInvalidation::NONE,
            subscription_invalidation: false,
            ordered: Vec::new(),
            mounted_work: WidgetOutputCollector::new(),
            remaining_outputs: output_allowance,
            used_outputs: 0,
            overflowed: false,
            producer_violation: None,
            output_limit: output_allowance,
            work_start_declared: [0; 5],
            work_start_used: [0; 5],
        }
    }

    #[must_use]
    pub(crate) fn into_output(mut self) -> EventContextOutput<Action> {
        EventContextOutput {
            ordered: self.ordered,
            application_command_disposition: self.application_command_disposition,
            invalidation: self.invalidation,
            subscription_invalidation: self.subscription_invalidation,
            mounted_work: self.mounted_work.take_outputs(),
            pointer_capture: self.pointer_capture,
            propagation_stopped: self.propagation_stopped,
            default_prevented: self.default_prevented,
            host_input_claimed: self.host_input_claimed,
            accepted_drag_drop: self.accepted_drag_drop,
            overflowed: self.overflowed,
            producer_violation: self.producer_violation,
            work_start_used: self.work_start_used,
            remaining_outputs: self.remaining_outputs,
            used_outputs: self.used_outputs,
        }
    }
}

impl<Action> EventContext<'_, Action> {
    /// Requests one coalesced subscription pass within this invocation.
    ///
    /// # Errors
    /// Returns the first sticky declaration violation.
    pub const fn try_invalidate_subscriptions(
        &mut self,
    ) -> Result<(), crate::ProducerContractViolation> {
        if let Some(cause) = self.producer_violation {
            return Err(cause);
        }
        if !self.subscription_invalidation {
            match self.try_reserve() {
                Ok(()) => self.subscription_invalidation = true,
                Err(cause) => return Err(cause),
            }
        }
        Ok(())
    }
    /// Returns the sticky violation, independently of optional trace.
    #[must_use]
    pub const fn producer_violation(&self) -> Option<crate::ProducerContractViolation> {
        self.producer_violation
    }
    const fn try_reserve(&mut self) -> Result<(), crate::ProducerContractViolation> {
        if let Some(cause) = self.producer_violation {
            return Err(cause);
        }
        if self.remaining_outputs == 0 {
            let cause = crate::ProducerContractViolation::output(
                crate::ProducerPhase::Widget(crate::WidgetProducerPhase::Event),
                self.output_limit,
            );
            self.producer_violation = Some(cause);
            self.overflowed = true;
            return Err(cause);
        }
        self.remaining_outputs -= 1;
        self.used_outputs += 1;
        Ok(())
    }
    /// Stages one owned operation before erasure.
    ///
    /// # Errors
    /// Returns the original request and the sticky declaration violation.
    pub fn try_emit(&mut self, action: Action) -> Result<(), crate::EmitError<Action>> {
        if let Err(cause) = self.try_reserve() {
            return Err(crate::EmitError::new(action, cause));
        }
        self.ordered.push(RoutedEventOutput::Action(action));
        Ok(())
    }
    /// Stages one owned operation before erasure.
    ///
    /// # Errors
    /// Returns the original request and the sticky declaration violation.
    pub fn try_emit_command(
        &mut self,
        command: SemanticCommand,
    ) -> Result<(), crate::EmitError<SemanticCommand>> {
        if let Err(cause) = self.try_reserve() {
            return Err(crate::EmitError::new(command, cause));
        }
        self.ordered.push(RoutedEventOutput::Command {
            target: self.current_target.clone(),
            command,
            origin: CommandOrigin::delegated(self.origin.source()),
        });
        Ok(())
    }
    /// Stages one owned operation before erasure.
    ///
    /// # Errors
    /// Returns the original request and the sticky declaration violation.
    pub fn try_emit_application_command(
        &mut self,
        command: ApplicationCommandId,
    ) -> Result<(), crate::EmitError<ApplicationCommandId>> {
        if let Err(cause) = self.try_reserve() {
            return Err(crate::EmitError::new(command, cause));
        }
        self.ordered.push(RoutedEventOutput::ApplicationCommand {
            target: self.original_target.clone(),
            command,
            origin: CommandOrigin::delegated(self.origin.source()),
        });
        Ok(())
    }
    /// Stages one owned operation before erasure.
    ///
    /// # Errors
    /// Returns the original request and the sticky declaration violation.
    pub fn try_local_task<F: Future<Output = Option<Action>> + 'static>(
        &mut self,
        future: F,
    ) -> Result<(), crate::EmitError<F>> {
        if let Err(cause) = self.try_reserve_work(WorkFamily::LocalTask) {
            return Err(crate::EmitError::new(future, cause));
        }
        self.mounted_work.local_task(future);
        Ok(())
    }
    /// Stages one owned operation before erasure.
    ///
    /// # Errors
    /// Returns the original request and the sticky declaration violation.
    pub fn try_keyed_local_task<F: Future<Output = Option<Action>> + 'static>(
        &mut self,
        key: WorkKey,
        future: F,
    ) -> Result<(), crate::EmitError<(WorkKey, F)>> {
        if let Err(cause) = self.try_reserve_work(WorkFamily::LocalTask) {
            return Err(crate::EmitError::new((key, future), cause));
        }
        self.mounted_work.keyed_local_task(key, future);
        Ok(())
    }
    /// Stages one owned operation before erasure.
    ///
    /// # Errors
    /// Returns the original request and the sticky declaration violation.
    pub fn try_send_task<I, F, M>(
        &mut self,
        future: F,
        map: M,
    ) -> Result<(), crate::EmitError<(F, M)>>
    where
        I: Send + 'static,
        F: Future<Output = I> + Send + 'static,
        M: FnOnce(I) -> Action + 'static,
    {
        if let Err(cause) = self.try_reserve_work(WorkFamily::SendTask) {
            return Err(crate::EmitError::new((future, map), cause));
        }
        self.mounted_work.send_task(future, map);
        Ok(())
    }
    /// Stages one owned operation before erasure.
    ///
    /// # Errors
    /// Returns the original request and the sticky declaration violation.
    pub fn try_keyed_send_task<I, F, M>(
        &mut self,
        key: WorkKey,
        future: F,
        map: M,
    ) -> Result<(), crate::EmitError<(WorkKey, F, M)>>
    where
        I: Send + 'static,
        F: Future<Output = I> + Send + 'static,
        M: FnOnce(I) -> Action + 'static,
    {
        if let Err(cause) = self.try_reserve_work(WorkFamily::SendTask) {
            return Err(crate::EmitError::new((key, future, map), cause));
        }
        self.mounted_work.keyed_send_task(key, future, map);
        Ok(())
    }
    /// Stages one owned operation before erasure.
    ///
    /// # Errors
    /// Returns the original request and the sticky declaration violation.
    pub fn try_send_task_with_failure<I, F, M, E>(
        &mut self,
        future: F,
        map: M,
        start_failure: E,
    ) -> Result<(), crate::EmitError<(F, M, E)>>
    where
        I: Send + 'static,
        F: Future<Output = I> + Send + 'static,
        M: FnOnce(I) -> Action + 'static,
        E: FnOnce(SendTaskStartFailure) -> Action + 'static,
    {
        if let Err(cause) = self.try_reserve_work(WorkFamily::SendTask) {
            return Err(crate::EmitError::new((future, map, start_failure), cause));
        }
        self.mounted_work
            .send_task_with_failure(future, map, start_failure);
        Ok(())
    }
    /// Stages one owned operation before erasure.
    ///
    /// # Errors
    /// Returns the original request and the sticky declaration violation.
    pub fn try_timer(
        &mut self,
        timer: TimerEffect<Action>,
    ) -> Result<(), crate::EmitError<TimerEffect<Action>>> {
        if let Err(cause) = self.try_reserve_work(WorkFamily::Timer) {
            return Err(crate::EmitError::new(timer, cause));
        }
        self.mounted_work.timer(timer);
        Ok(())
    }
    /// Stages one owned operation before erasure.
    ///
    /// # Errors
    /// Returns the original request and the sticky declaration violation.
    pub fn try_cancel(
        &mut self,
        family: WorkFamily,
        key: WorkKey,
    ) -> Result<(), crate::EmitError<(WorkFamily, WorkKey)>> {
        if let Err(cause) = self.try_reserve() {
            return Err(crate::EmitError::new((family, key), cause));
        }
        self.mounted_work.cancel(family, key);
        Ok(())
    }
}

impl<Action> EventContext<'_, Action> {
    pub(crate) const fn set_work_starts(&mut self, budget: crate::WorkStartBudget) {
        self.work_start_declared = [
            budget.for_family(WorkFamily::LocalTask),
            budget.for_family(WorkFamily::SendTask),
            budget.for_family(WorkFamily::Timer),
            budget.for_family(WorkFamily::HostRequest),
            budget.for_family(WorkFamily::FrameworkService),
        ];
    }
    fn try_reserve_work(
        &mut self,
        family: WorkFamily,
    ) -> Result<(), crate::ProducerContractViolation> {
        if let Some(cause) = self.producer_violation {
            return Err(cause);
        }
        let index = crate::producer_violation::family_index(family);
        if self.work_start_used[index] >= self.work_start_declared[index] {
            let cause = crate::ProducerContractViolation::work_start(
                crate::ProducerPhase::Widget(crate::WidgetProducerPhase::Event),
                family,
                self.work_start_declared[index],
                self.work_start_used[index],
            );
            self.producer_violation = Some(cause);
            self.overflowed = true;
            return Err(cause);
        }
        self.try_reserve()?;
        self.work_start_used[index] += 1;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use core::num::NonZeroU64;
    use std::rc::Rc;

    use crate::{
        __runtime::{MountedEffect, PointerCaptureRequest, RoutedEventOutput, RuntimeNamespace},
        ApplicationCommandId, CommandDerivation, CommandOrigin, DragDropEvent, DragDropPayloadKind,
        DragDropPayloadMetadata, DragDropPhase, EventPhase, EventSource, MonotonicInstant,
        PointerId, SemanticCommand, WidgetInvalidation, WorkSequence,
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
        let mut context = EventContext::new_with_scroll_control(
            EventPhase::Bubble,
            &original,
            &current,
            None,
            origin,
            sequence(7),
            MonotonicInstant::__runtime_from_nanos(11),
            None,
            None,
            true,
            false,
            false,
            4,
        );
        assert_eq!(context.phase(), EventPhase::Bubble);
        context.set_work_starts(crate::WorkStartBudget::new(1, 0, 0, 0, 0));
        assert_eq!(context.original_target(), &original);
        assert_eq!(context.current_target(), &current);
        assert_eq!(context.related_target(), None);
        assert_eq!(context.command_origin(), origin);
        assert_eq!(context.sequence().get(), 7);
        assert_eq!(context.instant().as_nanos(), 11);
        assert_eq!(context.pointer_id(), None);
        assert_eq!(context.physical_target(), None);
        assert_eq!(context.physical_path(), []);
        assert!(context.default_is_cancelable());
        assert!(!context.default_is_prevented());
        assert!(!context.propagation_is_stopped());
        assert!(!context.host_input_is_claimed());

        let _ = context.try_emit(String::from("first"));
        let _ = context.try_emit_command(SemanticCommand::OpenMenu);
        context.invalidate(WidgetInvalidation::PAINT);
        let _ = context.try_invalidate_subscriptions();
        let _ = context.try_local_task(async { Some(String::from("later")) });
        context.stop_propagation();
        context.prevent_default();
        context.claim_host_input();
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
        assert_eq!(output.pointer_capture, Vec::new());
        assert!(output.propagation_stopped);
        assert!(output.default_prevented);
        assert!(output.host_input_claimed);
        assert!(!output.overflowed);
        assert_eq!(output.remaining_outputs, 0);
    }

    #[test]
    fn pointer_context_preserves_physical_facts_and_capture_request_order() {
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
            None,
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
        let _ = context.try_capture_pointer();
        let _ = context.try_release_pointer_capture();
        let output = context.into_output();
        assert_eq!(
            output.pointer_capture,
            [
                PointerCaptureRequest::Capture {
                    pointer_id,
                    target: target.clone(),
                },
                PointerCaptureRequest::Release { pointer_id, target },
            ]
        );
        assert_eq!(output.remaining_outputs, 0);
    }

    #[test]
    fn drag_drop_admission_requires_the_exact_target_and_never_accepts_cancellation() {
        let namespace = RuntimeNamespace::__runtime_new();
        let target = namespace.__runtime_mounted_id(1, 1);
        let pointer_id = PointerId::new(7).unwrap_or_else(|| unreachable!("non-zero pointer"));
        let payload = DragDropPayloadMetadata::new(
            DragDropPayloadKind::Files,
            core::num::NonZeroU32::MIN,
            None,
        );
        let event = DragDropEvent::new(DragDropPhase::Drop, payload);

        let mut bubbling = EventContext::<()>::new_pointer(
            EventPhase::Bubble,
            &target,
            &target,
            None,
            CommandOrigin::__runtime_pointer(),
            sequence(4),
            MonotonicInstant::ZERO,
            Some(event),
            pointer_id,
            Some(&target),
            core::slice::from_ref(&target),
            false,
            false,
            false,
            1,
        );
        let _ = bubbling.try_accept_drag_drop();
        assert!(!bubbling.into_output().accepted_drag_drop);

        let cancel = DragDropEvent::new(DragDropPhase::Cancel, payload);
        let mut target_cancel = EventContext::<()>::new_pointer(
            EventPhase::Target,
            &target,
            &target,
            None,
            CommandOrigin::__runtime_pointer(),
            sequence(5),
            MonotonicInstant::ZERO,
            Some(cancel),
            pointer_id,
            Some(&target),
            core::slice::from_ref(&target),
            false,
            false,
            false,
            1,
        );
        let _ = target_cancel.try_accept_drag_drop();
        assert!(!target_cancel.into_output().accepted_drag_drop);

        let mut target_drop = EventContext::<()>::new_pointer(
            EventPhase::Target,
            &target,
            &target,
            None,
            CommandOrigin::__runtime_pointer(),
            sequence(6),
            MonotonicInstant::ZERO,
            Some(event),
            pointer_id,
            Some(&target),
            core::slice::from_ref(&target),
            false,
            false,
            false,
            1,
        );
        let _ = target_drop.try_accept_drag_drop();
        let _ = target_drop.try_accept_drag_drop();
        let output = target_drop.into_output();
        assert!(output.accepted_drag_drop);
        assert!(!output.overflowed);
        assert_eq!(output.remaining_outputs, 0);
    }

    #[test]
    fn mapped_pointer_context_preserves_every_capture_request() {
        let namespace = RuntimeNamespace::__runtime_new();
        let target = namespace.__runtime_mounted_id(1, 1);
        let pointer_id =
            PointerId::new(8).unwrap_or_else(|| unreachable!("test pointer is non-zero"));
        let mut parent = EventContext::<String>::new_pointer(
            EventPhase::Target,
            &target,
            &target,
            None,
            CommandOrigin::__runtime_pointer(),
            sequence(3),
            MonotonicInstant::ZERO,
            None,
            pointer_id,
            Some(&target),
            core::slice::from_ref(&target),
            true,
            false,
            false,
            2,
        );
        let mut child = parent.mapped_child::<()>();
        let _ = child.try_capture_pointer();
        let _ = child.try_release_pointer_capture();
        let mapper: Rc<dyn Fn(()) -> String> = Rc::new(|()| String::new());

        parent.absorb_mapped(child.into_output(), &mapper);

        assert_eq!(
            parent.into_output().pointer_capture,
            [
                PointerCaptureRequest::Capture {
                    pointer_id,
                    target: target.clone(),
                },
                PointerCaptureRequest::Release { pointer_id, target },
            ]
        );
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
            None,
            true,
            false,
            false,
            3,
        );
        parent.claim_host_input();
        let mut child = parent.mapped_child::<NonClone>();
        assert!(child.host_input_is_claimed());
        let _ = child.try_emit(NonClone(9));
        let _ = child.try_emit_command(SemanticCommand::CancelOrBack);
        let _ = child.try_emit_application_command(
            ApplicationCommandId::from_static("save")
                .unwrap_or_else(|_| unreachable!("test command id is valid")),
        );
        child.invalidate(WidgetInvalidation::SEMANTICS);
        child.prevent_default();
        child.claim_host_input();
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
        assert!(matches!(
            &output.ordered[2],
            RoutedEventOutput::ApplicationCommand { target: delegated, command, origin }
                if delegated == &target
                    && command.as_str() == "save"
                    && origin.source() == EventSource::Controller
                    && origin.derivation() == CommandDerivation::Delegated
        ));
        assert!(output.invalidation.contains(WidgetInvalidation::SEMANTICS));
        assert!(output.default_prevented);
        assert!(output.host_input_claimed);
        assert_eq!(output.remaining_outputs, 0);
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
            None,
            true,
            false,
            false,
            1,
        );
        let _ = context.try_emit(1);
        let _ = context.try_emit(2);
        let output = context.into_output();
        assert_eq!(output.ordered.len(), 1);
        assert!(output.overflowed);
        assert_eq!(output.remaining_outputs, 0);
    }
}
