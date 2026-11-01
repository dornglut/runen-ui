use super::{
    HostProtocol, MonotonicClock, Runtime, RuntimeStatus, SendTaskExecutor, SendTaskStartOutcome,
    TimerFiringOutcome, TimerStartOutcome, TraceRecordKind,
};

impl<State, Action, Protocol: HostProtocol> Runtime<State, Action, Protocol> {
    pub(crate) fn set_send_task_executor(&mut self, executor: impl SendTaskExecutor + 'static) {
        self.send_executor = Box::new(executor);
    }

    pub(crate) const fn last_send_task_start_outcome(&self) -> Option<SendTaskStartOutcome> {
        self.last_send_task_start_outcome
    }

    pub(crate) const fn last_timer_start_outcome(&self) -> Option<TimerStartOutcome> {
        self.last_timer_start_outcome
    }

    pub(crate) const fn last_timer_firing_outcome(&self) -> Option<TimerFiringOutcome> {
        self.last_timer_firing_outcome
    }

    pub(crate) fn set_monotonic_clock(&mut self, clock: impl MonotonicClock + 'static) {
        self.host_clock = Some(Box::new(clock));
    }

    pub(crate) fn set_wake_transport(&self, transport: impl crate::WakeTransport + 'static) {
        self.wake.set_transport(transport);
    }

    pub(crate) fn acknowledge_wake(&mut self) {
        self.wake.acknowledge();
        // Completion import owns an exact mandatory trace plan. An optional wake
        // acknowledgement must not consume one of that operation's sequences.
        if self.completion_ingress.len() == 0 && self.queue.is_empty() {
            self.record_optional(TraceRecordKind::WakeAcknowledged, None, None, None);
        }
    }

    /// An observation-capacity pause is mandatory further host work even if
    /// the canonical queue is empty: another readiness checkpoint is still due.
    pub(crate) fn request_input_observation_retry_wake(&mut self) {
        if !matches!(self.status, RuntimeStatus::Running) {
            return;
        }
        if matches!(
            self.wake.handle().request(),
            crate::WakeRequestOutcome::Requested
        ) {
            self.record_optional(TraceRecordKind::WakeRequested, None, None, None);
        }
    }

    pub(crate) fn rearm_wake_if_needed(&mut self) {
        let observation = self.scheduler_observation();
        let serviceable = !self.queue.is_empty()
            || observation.completion_imports_pending
            || observation.due_timers_pending
            || observation.local_polls_pending
            || observation.mandatory_derived_work_pending;
        if !serviceable {
            return;
        }
        if matches!(
            self.wake.handle().request(),
            crate::WakeRequestOutcome::Requested
        ) {
            self.record_optional(TraceRecordKind::WakeRequested, None, None, None);
        }
    }

    pub(crate) fn queue_is_empty(&self) -> bool {
        self.queue.is_empty()
    }
}
