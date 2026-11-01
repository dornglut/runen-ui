use super::{
    HostProtocol, MandatoryTracePlan, MonotonicClock, Runtime, RuntimeStatus,
    RuntimeTerminalReason, SendTaskExecutor, SendTaskStartOutcome, TimerFiringOutcome,
    TimerStartOutcome, TraceActionCategory, TraceRecordKind, TraceSequence, WorkFamily,
    WorkSequence,
};
use crate::queue::RootQuotaId;

pub(in crate::runtime) const fn producer_output_budget_fits(
    budget: runenui_core::ProducerBudget,
    transaction_outputs: usize,
) -> bool {
    budget.queue_credits() != 0
        && budget.application_outputs() <= budget.outputs()
        && budget.application_outputs() <= transaction_outputs
}

impl<State, Action, Protocol: HostProtocol> Runtime<State, Action, Protocol> {
    pub(crate) fn reserve_producer(
        &mut self,
        profile: runenui_core::ProducerBudget,
    ) -> Result<crate::queue::QueueReservation, crate::queue::QueueReservationError> {
        self.queue.preflight_reservation(profile.queue_credits())?;
        self.work
            .preflight_producer_residency(&self.queue, profile)
            .map_err(crate::queue::QueueReservationError::WorkCapacity)?;
        self.queue.reserve_for_producer(profile)
    }

    pub(crate) const fn application_output_budget_fits(
        &self,
        budget: runenui_core::ProducerBudget,
    ) -> bool {
        producer_output_budget_fits(budget, self.limits.transaction_outputs())
    }

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
        let queue_capacity_available = if let Some((generation, family)) = work
            && let Some(root) = self.work.root_quota(generation)
        {
            if family == crate::work::WorkFamily::HostRequest {
                if self
                    .queue
                    .preflight_root_host_response(root, generation)
                    .is_err()
                {
                    self.enter_terminal(RuntimeTerminalReason::Poisoned, 0);
                    return false;
                }
                true
            } else if family == crate::work::WorkFamily::LocalTask {
                if self
                    .queue
                    .preflight_root_local_task_outcome(root, generation)
                    .is_err()
                {
                    self.enter_terminal(RuntimeTerminalReason::Poisoned, 0);
                    return false;
                }
                true
            } else if family == crate::work::WorkFamily::SendTask {
                if self
                    .queue
                    .preflight_root_send_task_outcome(root, generation)
                    .is_err()
                {
                    self.enter_terminal(RuntimeTerminalReason::Poisoned, 0);
                    return false;
                }
                true
            } else if family == crate::work::WorkFamily::Timer {
                if self
                    .queue
                    .preflight_root_timer_callback(root, generation)
                    .is_err()
                {
                    self.enter_terminal(RuntimeTerminalReason::Poisoned, 0);
                    return false;
                }
                true
            } else if family == crate::work::WorkFamily::FrameworkService {
                if self
                    .queue
                    .preflight_root_framework_service_response(root, generation)
                    .is_err()
                {
                    self.enter_terminal(RuntimeTerminalReason::Poisoned, 0);
                    return false;
                }
                true
            } else {
                match self.queue.preflight_root_commit(root, 1) {
                    Ok(()) => true,
                    Err(crate::queue::QueueReservationError::SequenceExhausted) => {
                        self.enter_terminal(RuntimeTerminalReason::WorkSequenceExhausted, 0);
                        return false;
                    }
                    Err(_) => false,
                }
            }
        } else {
            self.queue.preflight_commit(1).is_ok()
        };
        if !queue_capacity_available {
            return false;
        }
        if let Some(root) = work.and_then(|(generation, _)| self.work.root_quota(generation))
            && let Err(error) = self.queue.preflight_root_sequence(root, 1)
        {
            let reason = match error {
                crate::queue::QueueReservationError::SequenceExhausted => {
                    RuntimeTerminalReason::WorkSequenceExhausted
                }
                _ => RuntimeTerminalReason::Poisoned,
            };
            self.enter_terminal(reason, 0);
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

    pub(in crate::runtime) fn send_task_start_refusal_preflight(
        &mut self,
        generation: crate::work::WorkGeneration,
        trace_plan: MandatoryTracePlan,
    ) -> bool {
        if !matches!(self.status, RuntimeStatus::Running) {
            return false;
        }
        let queue_capacity_available = if let Some(root) = self.work.root_quota(generation) {
            if self
                .queue
                .preflight_root_send_task_outcome(root, generation)
                .is_err()
            {
                self.enter_terminal(RuntimeTerminalReason::Poisoned, 0);
                return false;
            }
            true
        } else {
            self.queue.preflight_commit(1).is_ok()
        };
        if !queue_capacity_available {
            return false;
        }
        if let Some(root) = self.work.root_quota(generation)
            && let Err(error) = self.queue.preflight_root_sequence(root, 1)
        {
            let reason = match error {
                crate::queue::QueueReservationError::SequenceExhausted => {
                    RuntimeTerminalReason::WorkSequenceExhausted
                }
                _ => RuntimeTerminalReason::Poisoned,
            };
            self.enter_terminal(reason, 0);
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
        let instant = self.now();
        self.commit_preflighted_action(
            action,
            causal_parent,
            None,
            TraceActionCategory::ApplicationEffect,
            instant,
        )
        .map_or_else(
            |_| {
                self.enter_terminal(RuntimeTerminalReason::Poisoned, 0);
                None
            },
            Some,
        )
    }

    pub(in crate::runtime) fn queue_callback_action_from_root(
        &mut self,
        action: Action,
        causal_parent: Option<TraceSequence>,
        root_quota: RootQuotaId,
    ) -> Option<WorkSequence> {
        let Some(profile) = self.action_producer_budget else {
            self.enter_terminal(RuntimeTerminalReason::Poisoned, 0);
            return None;
        };
        let producer_budget = profile(self.state(), &action);
        let result = self.commit_preflighted_action_from_root(
            action,
            producer_budget,
            root_quota,
            causal_parent,
            None,
            TraceActionCategory::ApplicationEffect,
            self.now(),
        );
        result.map_or_else(
            |_| {
                self.enter_terminal(RuntimeTerminalReason::Poisoned, 0);
                None
            },
            Some,
        )
    }

    pub(in crate::runtime) fn queue_timer_callback_action(
        &mut self,
        action: Action,
        causal_parent: Option<TraceSequence>,
        root_quota: RootQuotaId,
        generation: crate::work::WorkGeneration,
    ) -> Option<WorkSequence> {
        let Some(profile) = self.action_producer_budget else {
            self.enter_terminal(RuntimeTerminalReason::Poisoned, 0);
            return None;
        };
        let producer_budget = profile(self.state(), &action);
        self.commit_preflighted_timer_action_from_root(
            action,
            producer_budget,
            root_quota,
            generation,
            causal_parent,
            TraceActionCategory::ApplicationEffect,
            self.now(),
        )
        .map_or_else(
            |_| {
                self.enter_terminal(RuntimeTerminalReason::Poisoned, 0);
                None
            },
            Some,
        )
    }

    pub(in crate::runtime) fn queue_callback_action_for_work(
        &mut self,
        action: Action,
        causal_parent: Option<TraceSequence>,
        generation: crate::work::WorkGeneration,
    ) -> Option<WorkSequence> {
        match self.work.root_quota(generation) {
            Some(root_quota) => {
                self.queue_callback_action_from_root(action, causal_parent, root_quota)
            }
            None => self.queue_callback_action(action, causal_parent),
        }
    }

    pub(in crate::runtime) fn queue_local_task_action(
        &mut self,
        action: Action,
        causal_parent: Option<TraceSequence>,
        generation: crate::work::WorkGeneration,
    ) -> Option<WorkSequence> {
        let Some(root_quota) = self.work.root_quota(generation) else {
            return self.queue_callback_action(action, causal_parent);
        };
        let Some(profile) = self.action_producer_budget else {
            self.enter_terminal(RuntimeTerminalReason::Poisoned, 0);
            return None;
        };
        let producer_budget = profile(self.state(), &action);
        self.commit_preflighted_local_task_action_from_root(
            action,
            producer_budget,
            root_quota,
            generation,
            causal_parent,
            TraceActionCategory::ApplicationEffect,
            self.now(),
        )
        .map_or_else(
            |_| {
                self.enter_terminal(RuntimeTerminalReason::Poisoned, 0);
                None
            },
            Some,
        )
    }

    pub(in crate::runtime) fn queue_send_task_action(
        &mut self,
        action: Action,
        causal_parent: Option<TraceSequence>,
        generation: crate::work::WorkGeneration,
    ) -> Option<WorkSequence> {
        let Some(root_quota) = self.work.root_quota(generation) else {
            return self.queue_callback_action(action, causal_parent);
        };
        let Some(profile) = self.action_producer_budget else {
            self.enter_terminal(RuntimeTerminalReason::Poisoned, 0);
            return None;
        };
        let producer_budget = profile(self.state(), &action);
        self.commit_preflighted_send_task_action_from_root(
            action,
            producer_budget,
            root_quota,
            generation,
            causal_parent,
            TraceActionCategory::ApplicationEffect,
            self.now(),
        )
        .map_or_else(
            |_| {
                self.enter_terminal(RuntimeTerminalReason::Poisoned, 0);
                None
            },
            Some,
        )
    }

    pub(in crate::runtime) fn queue_host_response_action_for_work(
        &mut self,
        action: Action,
        causal_parent: Option<TraceSequence>,
        generation: crate::work::WorkGeneration,
    ) -> Option<WorkSequence> {
        let Some(root_quota) = self.work.root_quota(generation) else {
            return self.queue_callback_action(action, causal_parent);
        };
        let Some(profile) = self.action_producer_budget else {
            self.enter_terminal(RuntimeTerminalReason::Poisoned, 0);
            return None;
        };
        let producer_budget = profile(self.state(), &action);
        self.commit_preflighted_host_response_action_from_root(
            action,
            producer_budget,
            root_quota,
            generation,
            causal_parent,
            TraceActionCategory::ApplicationEffect,
            self.now(),
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
