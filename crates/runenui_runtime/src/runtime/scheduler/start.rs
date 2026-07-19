use super::{
    Effect, HostProtocol, LiveHostRequest, LocalTask, MandatoryTracePlan, Runtime,
    RuntimeTerminalReason, SendSubscriptionSink, SendSubscriptionStartOutcome, SendTaskJob,
    SendTaskMapper, SendTaskStartError, SendTaskStartFailure, SendTaskStartOutcome, Timer,
    TimerStartError, TimerStartOutcome, TraceRecordKind, TraceSequence, TraceWorkIdentity,
    TraceWorkStartRefusal, WorkFamily, WorkSequence,
};

impl<State, Action, Protocol: HostProtocol> Runtime<State, Action, Protocol> {
    #[allow(clippy::too_many_lines)]
    pub(crate) fn process_effect_start(
        &mut self,
        sequence: WorkSequence,
        generation: crate::work::WorkGeneration,
    ) {
        let Some(family) = self.work.pending_family(generation) else {
            return;
        };
        let Some(identity) = self.trace_work_identity(generation) else {
            return;
        };
        if !self.trace.can_admit(MandatoryTracePlan::work_start(
            family == WorkFamily::HostRequest,
        )) {
            self.enter_terminal(RuntimeTerminalReason::TraceSequenceExhausted, 0);
            return;
        }
        self.record_work_fact_from_envelope(
            TraceRecordKind::WorkStartAttempted,
            sequence,
            identity.clone(),
        );
        match family {
            WorkFamily::LocalTask => {
                let Some(Effect::LocalTask(task)) = self.work.take_pending_effect(generation)
                else {
                    self.enter_terminal(RuntimeTerminalReason::Poisoned, 0);
                    return;
                };
                if self.work.mark_running(generation).is_none() {
                    return;
                }
                self.local_tasks
                    .push(LocalTask::new(generation, task.future, self.wake.handle()));
                self.record_work_fact_from_envelope(
                    TraceRecordKind::WorkStartAccepted,
                    sequence,
                    identity,
                );
            }
            WorkFamily::Timer => {
                let Some(Effect::Timer(timer)) = self.work.take_pending_effect(generation) else {
                    self.enter_terminal(RuntimeTerminalReason::Poisoned, 0);
                    return;
                };
                match Timer::new(generation, self.now(), timer) {
                    Ok(timer) => {
                        if self.work.mark_running(generation).is_none() {
                            return;
                        }
                        self.timers.push(timer);
                        self.last_timer_start_outcome = Some(TimerStartOutcome::Started);
                        self.record_work_fact_from_envelope(
                            TraceRecordKind::WorkStartAccepted,
                            sequence,
                            identity,
                        );
                    }
                    Err(error) => {
                        let (outcome, refusal) = match error {
                            TimerStartError::ZeroInterval => (
                                TimerStartOutcome::ZeroInterval,
                                TraceWorkStartRefusal::TimerZeroInterval,
                            ),
                            TimerStartError::DeadlineOverflow => (
                                TimerStartOutcome::DeadlineOverflow,
                                TraceWorkStartRefusal::TimerDeadlineOverflow,
                            ),
                        };
                        self.last_timer_start_outcome = Some(outcome);
                        self.record_work_fact_from_envelope(
                            TraceRecordKind::WorkStartRefused { outcome: refusal },
                            sequence,
                            identity,
                        );
                        self.revoke_generation(generation);
                    }
                }
            }
            WorkFamily::SendTask => {
                let Some(Effect::SendTask(task)) = self.work.take_pending_effect(generation) else {
                    self.enter_terminal(RuntimeTerminalReason::Poisoned, 0);
                    return;
                };
                let job = SendTaskJob::new(
                    generation,
                    task.future,
                    self.completion_ingress.sender(),
                    identity.clone(),
                    self.work.trace_parent(generation),
                );
                self.completion_ingress
                    .register_send_task_starting(generation);
                match self.send_executor.start(job) {
                    Ok(()) => {
                        if !self
                            .completion_ingress
                            .promote_send_task_running(generation)
                            || self.work.mark_running(generation).is_none()
                        {
                            self.revoke_generation(generation);
                            return;
                        }
                        self.send_task_mappers.push(SendTaskMapper {
                            generation,
                            map: task.map,
                        });
                        self.last_send_task_start_outcome = Some(SendTaskStartOutcome::Started);
                        self.record_work_fact_from_envelope(
                            TraceRecordKind::WorkStartAccepted,
                            sequence,
                            identity,
                        );
                    }
                    Err(error) => {
                        let (outcome, failure, refusal) = match error {
                            SendTaskStartError::Unavailable(_job) => (
                                SendTaskStartOutcome::Unavailable,
                                SendTaskStartFailure::Unavailable,
                                TraceWorkStartRefusal::ExecutorUnavailable,
                            ),
                            SendTaskStartError::Full(_job) => (
                                SendTaskStartOutcome::Full,
                                SendTaskStartFailure::Full,
                                TraceWorkStartRefusal::ExecutorFull,
                            ),
                            SendTaskStartError::Closed(_job) => (
                                SendTaskStartOutcome::Closed,
                                SendTaskStartFailure::Closed,
                                TraceWorkStartRefusal::ExecutorClosed,
                            ),
                            SendTaskStartError::Rejected(_job) => (
                                SendTaskStartOutcome::Rejected,
                                SendTaskStartFailure::Rejected,
                                TraceWorkStartRefusal::ExecutorRejected,
                            ),
                        };
                        self.last_send_task_start_outcome = Some(outcome);
                        let refusal_parent = self.record_work_fact_from_envelope(
                            TraceRecordKind::WorkStartRefused { outcome: refusal },
                            sequence,
                            identity,
                        );
                        self.revoke_generation(generation);
                        if let Some(map) = task.start_failure
                            && self.callback_output_preflight(
                                None,
                                MandatoryTracePlan::typed_start_refusal_with_action(),
                            )
                        {
                            let action = map(failure);
                            self.queue_callback_action(action, refusal_parent);
                        }
                    }
                }
            }
            WorkFamily::Subscription => {
                let Some(subscription_index) = self
                    .subscriptions
                    .iter()
                    .position(|subscription| subscription.generation == generation)
                else {
                    self.enter_terminal(RuntimeTerminalReason::Poisoned, 0);
                    return;
                };
                self.completion_ingress
                    .register_subscription_starting(generation);
                let sender = self.completion_ingress.sender();
                let completion_identity = identity.clone();
                let completion_parent = self.work.trace_parent(generation);
                let sink = SendSubscriptionSink::__runtime_new(move |output| {
                    sender.submit_subscription(
                        generation,
                        output,
                        completion_identity.clone(),
                        completion_parent,
                    )
                });
                let outcome = self.subscriptions[subscription_index].start_send(sink);
                match outcome {
                    None => {
                        self.completion_ingress.release_subscription(generation);
                        if self.work.mark_running(generation).is_none() {
                            return;
                        }
                        self.subscriptions[subscription_index].started = true;
                        self.record_work_fact_from_envelope(
                            TraceRecordKind::WorkStartAccepted,
                            sequence,
                            identity,
                        );
                    }
                    Some(SendSubscriptionStartOutcome::Started) => {
                        if !self
                            .completion_ingress
                            .promote_subscription_running(generation)
                            || self.work.mark_running(generation).is_none()
                        {
                            self.completion_ingress.release_subscription(generation);
                            return;
                        }
                        self.subscriptions[subscription_index].started = true;
                        self.record_work_fact_from_envelope(
                            TraceRecordKind::WorkStartAccepted,
                            sequence,
                            identity,
                        );
                    }
                    Some(outcome) => {
                        self.completion_ingress.release_subscription(generation);
                        let outcome = match outcome {
                            SendSubscriptionStartOutcome::Unavailable => {
                                TraceWorkStartRefusal::SubscriptionUnavailable
                            }
                            SendSubscriptionStartOutcome::Full => {
                                TraceWorkStartRefusal::SubscriptionFull
                            }
                            SendSubscriptionStartOutcome::Closed => {
                                TraceWorkStartRefusal::SubscriptionClosed
                            }
                            SendSubscriptionStartOutcome::Rejected => {
                                TraceWorkStartRefusal::SubscriptionRejected
                            }
                            SendSubscriptionStartOutcome::Started => unreachable!(),
                            _ => TraceWorkStartRefusal::SubscriptionRejected,
                        };
                        self.record_work_fact_from_envelope(
                            TraceRecordKind::WorkStartRefused { outcome },
                            sequence,
                            identity,
                        );
                        self.revoke_generation(generation);
                    }
                }
            }
            WorkFamily::HostRequest => {
                let Some(Effect::HostRequest(request)) = self.work.take_pending_effect(generation)
                else {
                    self.enter_terminal(RuntimeTerminalReason::Poisoned, 0);
                    return;
                };
                if self.work.mark_running(generation).is_none() {
                    return;
                }
                self.host_requests
                    .push(LiveHostRequest::new(generation, request));
                self.completion_ingress.register_host_response(generation);
                self.record_work_fact_from_envelope(
                    TraceRecordKind::WorkStartAccepted,
                    sequence,
                    identity.clone(),
                );
                self.record_work_fact_from_envelope(
                    TraceRecordKind::HostRequestExposed,
                    sequence,
                    identity,
                );
            }
        }
    }

    pub(crate) fn process_work_cancellation(
        &mut self,
        sequence: WorkSequence,
        generation: crate::work::WorkGeneration,
        identity: TraceWorkIdentity,
        causal_parent: Option<TraceSequence>,
    ) {
        if !self.trace.can_admit(MandatoryTracePlan::one_fact()) {
            self.enter_terminal(RuntimeTerminalReason::TraceSequenceExhausted, 0);
            return;
        }
        self.trace.record_work(
            TraceRecordKind::WorkCleanupProcessed,
            Some(sequence),
            causal_parent,
            None,
            None,
            None,
            identity,
        );
        self.revoke_generation(generation);
    }
}
