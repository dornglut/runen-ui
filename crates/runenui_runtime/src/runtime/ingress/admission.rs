use super::{
    HostProtocol, MandatoryTracePlan, Runtime, RuntimeStatus, RuntimeTerminalReason, WorkFamily,
};

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
}
