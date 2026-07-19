use super::{
    ApplicationActionOrigin, HostProtocol, MandatoryTracePlan, MonotonicClock, Runtime,
    RuntimeStatus, RuntimeTerminalReason, SendTaskExecutor, SendTaskStartOutcome,
    TimerFiringOutcome, TimerStartOutcome, TraceRecordKind, TraceSequence, WorkFamily,
    WorkSequence,
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

    pub(in crate::runtime) fn callback_output_preflight(
        &mut self,
        work: Option<(crate::work::WorkGeneration, WorkFamily)>,
        trace_plan: MandatoryTracePlan,
    ) -> bool {
        if !matches!(self.status, RuntimeStatus::Running) {
            return false;
        }
        if let Some((generation, family)) = work
            && !self.work.is_running_family(generation, family)
        {
            return false;
        }
        if self.queue.is_full() {
            return false;
        }
        if !self.queue.has_sequence() {
            self.enter_terminal(RuntimeTerminalReason::WorkSequenceExhausted, 0);
            return false;
        }
        if !self.trace.can_admit(trace_plan) {
            self.enter_terminal(RuntimeTerminalReason::TraceSequenceExhausted, 0);
            return false;
        }
        true
    }

    pub(in crate::runtime) fn queue_callback_action(
        &mut self,
        action: Action,
        causal_parent: Option<TraceSequence>,
    ) -> Option<WorkSequence> {
        self.commit_preflighted_action(
            action,
            causal_parent,
            None,
            ApplicationActionOrigin::ApplicationEffect,
        )
        .map_or_else(
            |_| {
                self.enter_terminal(RuntimeTerminalReason::Poisoned, 0);
                None
            },
            Some,
        )
    }
}
