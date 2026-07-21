use super::{
    CompletionKind, HostProtocol, LiveSubscriptionSource, MandatoryTracePlan,
    ReadinessCheckpointReport, Runtime, RuntimeTerminalReason, TraceRecordKind, WorkFamily,
};

impl<State, Action, Protocol: HostProtocol> Runtime<State, Action, Protocol> {
    pub(crate) fn queued_len(&self) -> usize {
        self.queue.len()
    }

    pub(crate) fn readiness_checkpoint(
        &mut self,
        max_completion_imports: usize,
        max_local_polls: usize,
        max_timer_promotions: usize,
    ) -> ReadinessCheckpointReport {
        let imported_completions = self.import_send_completions(max_completion_imports);
        let promoted_timers = self.promote_due_timers(max_timer_promotions);
        let polled_local_work = self.poll_local_work(max_local_polls);
        let report = ReadinessCheckpointReport {
            imported_completions,
            polled_local_work,
            promoted_timers,
        };
        self.record_optional(
            TraceRecordKind::ReadinessCheckpoint {
                imported_completions,
                polled_local_work,
                promoted_timers,
            },
            None,
            None,
            None,
        );
        report
    }

    #[allow(clippy::too_many_lines)]
    pub(in crate::runtime) fn import_send_completions(&mut self, limit: usize) -> usize {
        let mut imported = 0;
        while imported < limit {
            let Some((generation, kind, identity, completion_parent)) =
                self.completion_ingress.front()
            else {
                break;
            };
            let family = match kind {
                CompletionKind::SendTask => WorkFamily::SendTask,
                CompletionKind::Subscription => WorkFamily::Subscription,
                CompletionKind::HostResponse => WorkFamily::HostRequest,
            };
            if self.work.is_running_family(generation, family) {
                let trace_plan = match family {
                    WorkFamily::HostRequest => MandatoryTracePlan::host_completion(),
                    WorkFamily::SendTask | WorkFamily::Subscription => {
                        MandatoryTracePlan::send_completion()
                    }
                    WorkFamily::LocalTask | WorkFamily::Timer => unreachable!(),
                };
                if !self.callback_output_preflight(Some((generation, family)), trace_plan) {
                    break;
                }
            } else if !self.trace.can_admit(MandatoryTracePlan::one_fact()) {
                self.enter_terminal(RuntimeTerminalReason::TraceSequenceExhausted, 0);
                break;
            }
            let Some(completion) = self.completion_ingress.pop() else {
                unreachable!("completion ingress front remains owned until UI-thread pop")
            };
            imported += 1;
            if !self.work.is_running(completion.generation) {
                let stale_parent = self
                    .trace
                    .records()
                    .filter(|record| record.work() == Some(&completion.trace_identity))
                    .map(crate::TraceRecord::sequence)
                    .last()
                    .or(completion_parent);
                self.record_work_fact_with_parent(
                    TraceRecordKind::WorkCompletionRejectedStale,
                    stale_parent,
                    completion.trace_identity,
                );
                if completion.kind == CompletionKind::HostResponse {
                    self.completion_ingress
                        .release_host_response(completion.generation);
                }
                continue;
            }
            self.record_work_fact(TraceRecordKind::WorkCompletionImported, identity.clone());
            if completion.kind == CompletionKind::Subscription {
                let Some(subscription) = self.subscriptions.iter_mut().find(|subscription| {
                    subscription.generation == completion.generation && subscription.started
                }) else {
                    continue;
                };
                let LiveSubscriptionSource::Send { map, .. } = &mut subscription.source else {
                    continue;
                };
                let action = map(completion.output);
                let mapped = self.record_work_fact(TraceRecordKind::WorkCompletionMapped, identity);
                self.queue_callback_action(action, mapped);
                continue;
            }
            if completion.kind == CompletionKind::HostResponse {
                let Ok(response) = completion.output.downcast::<Protocol::Response>() else {
                    self.completion_ingress
                        .release_host_response(completion.generation);
                    continue;
                };
                let response = *response;
                let Some(request) = self.host_requests.iter().find(|request| {
                    request.generation == completion.generation
                        && self
                            .work
                            .is_running_family(request.generation, WorkFamily::HostRequest)
                }) else {
                    self.completion_ingress
                        .release_host_response(completion.generation);
                    continue;
                };
                if request.expected != Protocol::response_kind(&response) {
                    self.completion_ingress
                        .release_host_response(completion.generation);
                    continue;
                }
                self.record_work_fact(TraceRecordKind::HostResponseAccepted, identity.clone());
                let Some(request_index) = self
                    .host_requests
                    .iter()
                    .position(|request| request.generation == completion.generation)
                else {
                    self.completion_ingress
                        .release_host_response(completion.generation);
                    continue;
                };
                let request = self.host_requests.remove(request_index);
                let action = (request.map)(response);
                let mapped = self.record_work_fact(TraceRecordKind::WorkCompletionMapped, identity);
                self.revoke_generation(completion.generation);
                self.queue_callback_action(action, mapped);
                continue;
            }
            let Some(index) = self
                .send_task_mappers
                .iter()
                .position(|mapper| mapper.generation == completion.generation)
            else {
                continue;
            };
            let mapper = self.send_task_mappers.remove(index);
            let action = (mapper.map)(completion.output);
            let mapping_trace =
                self.record_work_fact(TraceRecordKind::WorkCompletionMapped, identity);
            self.revoke_generation(completion.generation);
            self.queue_callback_action(action, mapping_trace);
        }
        imported
    }
}
