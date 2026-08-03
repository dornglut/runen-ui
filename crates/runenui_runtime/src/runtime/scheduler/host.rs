use super::{
    Arc, HostProtocol, HostRequestCancelError, HostRequestRef, HostRequestToken,
    HostResponseCompletion, HostResponseError, MandatoryTracePlan, MonotonicClock,
    MonotonicInstant, QueueCommitError, Runtime, RuntimeStatus, RuntimeTerminalReason,
    TraceRecordKind, WorkFamily, WorkSequence,
};

impl<State, Action, Protocol: HostProtocol> Runtime<State, Action, Protocol> {
    pub(crate) fn pending_host_requests(&self) -> Vec<HostRequestRef<'_, Protocol>> {
        self.host_requests
            .iter()
            .filter(|request| {
                self.work
                    .is_running_family(request.generation, WorkFamily::HostRequest)
            })
            .map(|request| HostRequestRef {
                token: HostRequestToken {
                    namespace: Arc::clone(&self.host_namespace),
                    generation: request.generation,
                },
                command: &request.command,
            })
            .collect()
    }

    pub(crate) fn complete_host_request(
        &mut self,
        token: &HostRequestToken,
        response: Protocol::Response,
    ) -> Result<WorkSequence, HostResponseError<Protocol::Response>> {
        match self.status {
            RuntimeStatus::Closed => return Err(HostResponseError::Closed(response)),
            RuntimeStatus::Terminal(reason) => {
                return Err(HostResponseError::Terminal { response, reason });
            }
            RuntimeStatus::Running => {}
        }
        if !Arc::ptr_eq(&self.host_namespace, &token.namespace) {
            return Err(HostResponseError::ForeignRuntime(response));
        }
        let Some(request) = self.host_requests.iter().find(|request| {
            request.generation == token.generation
                && self
                    .work
                    .is_running_family(request.generation, WorkFamily::HostRequest)
        }) else {
            return Err(HostResponseError::Stale(response));
        };
        if request.expected != Protocol::response_kind(&response) {
            let identity = self
                .trace_work_identity(token.generation)
                .unwrap_or_else(|| unreachable!("live host request has trace identity"));
            if !self.trace.can_admit(MandatoryTracePlan::one_fact()) {
                let reason = RuntimeTerminalReason::TraceSequenceExhausted;
                self.enter_terminal(reason, 0);
                return Err(HostResponseError::Terminal { response, reason });
            }
            self.record_work_fact(TraceRecordKind::HostResponseRejected, identity);
            return Err(HostResponseError::MismatchedKind(response));
        }
        if !self.callback_output_preflight(
            Some((token.generation, WorkFamily::HostRequest)),
            MandatoryTracePlan::callback_with_action(),
        ) {
            if matches!(self.status, RuntimeStatus::Running) {
                return Err(HostResponseError::Full(response));
            }
            let RuntimeStatus::Terminal(reason) = self.status else {
                unreachable!("callback preflight only closes through a terminal transition")
            };
            return Err(HostResponseError::Terminal { response, reason });
        }
        let Some(request_index) = self
            .host_requests
            .iter()
            .position(|request| request.generation == token.generation)
        else {
            return Err(HostResponseError::Stale(response));
        };
        if !self
            .completion_ingress
            .claim_direct_host_response(token.generation)
        {
            return Err(HostResponseError::Stale(response));
        }
        let identity = self
            .trace_work_identity(token.generation)
            .unwrap_or_else(|| unreachable!("live host request has trace identity"));
        self.record_work_fact(TraceRecordKind::HostResponseAccepted, identity.clone());
        let request = self.host_requests.remove(request_index);
        let action = (request.map)(response);
        let mapped = self.record_work_fact(TraceRecordKind::WorkCompletionMapped, identity);
        self.revoke_generation(token.generation);
        let Some(sequence) = self.queue_callback_action(action, mapped) else {
            unreachable!("host callback output was preflighted")
        };
        self.external_queue_commit_accepted();
        Ok(sequence)
    }

    pub(crate) fn host_response_completion(
        &mut self,
        token: &HostRequestToken,
        response: Protocol::Response,
    ) -> Result<HostResponseCompletion, HostResponseError<Protocol::Response>>
    where
        Protocol::Response: Send + 'static,
    {
        match self.status {
            RuntimeStatus::Closed => return Err(HostResponseError::Closed(response)),
            RuntimeStatus::Terminal(reason) => {
                return Err(HostResponseError::Terminal { response, reason });
            }
            RuntimeStatus::Running => {}
        }
        if !Arc::ptr_eq(&self.host_namespace, &token.namespace) {
            return Err(HostResponseError::ForeignRuntime(response));
        }
        if !self
            .work
            .is_running_family(token.generation, WorkFamily::HostRequest)
        {
            return Err(HostResponseError::Stale(response));
        }
        if !self
            .completion_ingress
            .host_response_is_open(token.generation)
        {
            return Err(HostResponseError::Stale(response));
        }
        let Some(request) = self
            .host_requests
            .iter()
            .find(|request| request.generation == token.generation)
        else {
            return Err(HostResponseError::Stale(response));
        };
        if request.expected != Protocol::response_kind(&response) {
            let identity = self
                .trace_work_identity(token.generation)
                .unwrap_or_else(|| unreachable!("live host request has trace identity"));
            if !self.trace.can_admit(MandatoryTracePlan::one_fact()) {
                let reason = RuntimeTerminalReason::TraceSequenceExhausted;
                self.enter_terminal(reason, 0);
                return Err(HostResponseError::Terminal { response, reason });
            }
            self.record_work_fact(TraceRecordKind::HostResponseRejected, identity);
            return Err(HostResponseError::MismatchedKind(response));
        }
        Ok(HostResponseCompletion::new(
            token.generation,
            Box::new(response),
            self.completion_ingress.sender(),
            self.trace_work_identity(token.generation)
                .unwrap_or_else(|| unreachable!("live host request has trace identity")),
            self.work.trace_parent(token.generation),
        ))
    }

    pub(crate) fn cancel_host_request(
        &mut self,
        token: &HostRequestToken,
    ) -> Result<WorkSequence, HostRequestCancelError> {
        match self.status {
            RuntimeStatus::Closed => return Err(HostRequestCancelError::Closed),
            RuntimeStatus::Terminal(reason) => {
                return Err(HostRequestCancelError::Terminal(reason));
            }
            RuntimeStatus::Running => {}
        }
        if !Arc::ptr_eq(&self.host_namespace, &token.namespace) {
            return Err(HostRequestCancelError::ForeignRuntime);
        }
        if !self
            .work
            .is_running_family(token.generation, WorkFamily::HostRequest)
        {
            return Err(HostRequestCancelError::Stale);
        }
        match self.queue.preflight_commit(1) {
            Ok(()) => {}
            Err(QueueCommitError::Full) => return Err(HostRequestCancelError::Full),
            Err(QueueCommitError::SequenceExhausted) => {
                let reason = RuntimeTerminalReason::WorkSequenceExhausted;
                self.enter_terminal(reason, 0);
                return Err(HostRequestCancelError::Terminal(reason));
            }
        }
        if !self
            .trace
            .can_admit(MandatoryTracePlan::work_cancellation())
        {
            let reason = RuntimeTerminalReason::TraceSequenceExhausted;
            self.enter_terminal(reason, 0);
            return Err(HostRequestCancelError::Terminal(reason));
        }
        let identity = self
            .trace_work_identity(token.generation)
            .unwrap_or_else(|| unreachable!("live host request has trace identity"));
        let lineage = self.record_invalidation_facts(
            core::slice::from_ref(&identity),
            self.work.trace_parent(token.generation),
        );
        let (_, parent) = lineage
            .get(&token.generation.get())
            .cloned()
            .unwrap_or_else(|| unreachable!("host cancellation retains trace lineage"));
        let sequence = self
            .queue
            .push_cancellation(token.generation, identity, parent)
            .unwrap_or_else(|_| unreachable!("host cancellation was preflighted"));
        self.invalidate_generation_now(token.generation);
        self.external_queue_commit_accepted();
        Ok(sequence)
    }

    pub(crate) fn advance_time(
        &self,
        duration: std::time::Duration,
    ) -> Result<MonotonicInstant, crate::MonotonicTimeError> {
        let now = self.clock.advance(duration)?;
        if self
            .timers
            .iter()
            .any(|timer| self.work.is_running(timer.generation) && timer.is_due(now))
        {
            let _ = self.wake.handle().request();
        }
        Ok(now)
    }

    pub(crate) fn now(&self) -> MonotonicInstant {
        self.host_clock
            .as_ref()
            .map_or_else(|| self.clock.now(), |clock| clock.now())
    }
}
