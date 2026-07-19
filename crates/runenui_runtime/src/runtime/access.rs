use super::{
    FocusState, HostProtocol, MandatoryTracePlan, MountedNodeId, ReconciliationReport, Runtime,
    RuntimeStatus, Trace, TraceRecordKind, TraceSequence, TraceTarget, WorkSequence,
};

impl<State, Action, Protocol: HostProtocol> Runtime<State, Action, Protocol> {
    pub(crate) fn record_optional(
        &mut self,
        kind: TraceRecordKind,
        work_sequence: Option<WorkSequence>,
        causal_parent: Option<TraceSequence>,
        target: Option<TraceTarget>,
    ) {
        if self.trace.can_admit(MandatoryTracePlan::one_fact()) {
            self.trace
                .record(kind, work_sequence, causal_parent, None, None, target);
        }
    }

    #[must_use]
    pub(crate) const fn state(&self) -> &State {
        match &self.state {
            Some(state) => state,
            None => unreachable!(),
        }
    }
    #[must_use]
    pub(crate) const fn trace(&self) -> &Trace {
        &self.trace
    }
    #[must_use]
    pub(crate) const fn focus(&self) -> &FocusState {
        &self.focus
    }
    pub(crate) fn set_focus(&mut self, id: MountedNodeId) {
        self.focus.set(id);
    }
    pub(crate) fn clear_focus(&mut self) {
        self.focus.clear();
    }
    #[must_use]
    pub(crate) const fn report(&self) -> &ReconciliationReport {
        &self.report
    }
    #[must_use]
    pub(crate) const fn status(&self) -> RuntimeStatus {
        self.status
    }

    #[cfg(test)]
    pub(crate) const fn note_readiness_checkpoint(&mut self) {
        self.readiness_checkpoint_count += 1;
    }

    #[cfg(test)]
    pub(crate) const fn readiness_checkpoint_count_for_test(&self) -> usize {
        self.readiness_checkpoint_count
    }

    #[cfg(feature = "internal-test-seams")]
    pub(crate) fn live_work_record_count_for_test(&self) -> usize {
        self.work.live_record_count_for_test()
    }

    #[cfg(feature = "internal-test-seams")]
    pub(crate) fn host_response_slot_count_for_test(&self) -> usize {
        self.completion_ingress.host_response_slot_count_for_test()
    }

    #[cfg(feature = "internal-test-seams")]
    pub(crate) fn send_task_slot_count_for_test(&self) -> usize {
        self.completion_ingress.send_task_slot_count_for_test()
    }

    #[cfg(feature = "internal-test-seams")]
    pub(crate) fn subscription_slot_count_for_test(&self) -> usize {
        self.completion_ingress.subscription_slot_count_for_test()
    }

    #[cfg(feature = "internal-test-seams")]
    pub(crate) fn completion_payload_count_for_test(&self) -> usize {
        self.completion_ingress.payload_count_for_test()
    }

    #[cfg(feature = "internal-test-seams")]
    pub(crate) const fn send_task_mapper_count_for_test(&self) -> usize {
        self.send_task_mappers.len()
    }

    #[cfg(feature = "internal-test-seams")]
    pub(crate) const fn seed_generation_for_test(&mut self, generation: u64) {
        self.generation = generation;
    }

    #[cfg(feature = "internal-test-seams")]
    pub(crate) const fn seed_next_work_sequence_for_test(&mut self, next: u64) {
        self.queue.seed_next_sequence_for_test(next);
    }

    #[cfg(feature = "internal-test-seams")]
    pub(crate) const fn seed_next_work_generation_for_test(&mut self, next: u64) {
        self.work.seed_next_generation_for_test(next);
    }

    #[cfg(feature = "internal-test-seams")]
    pub(crate) const fn seed_next_trace_sequence_for_test(&mut self, next: u64) {
        self.trace.seed_next_sequence_for_test(next);
    }

    #[cfg(feature = "internal-test-seams")]
    pub(crate) fn routed_sequence_state_for_test(&self) -> (Option<u64>, Option<u64>) {
        (
            self.queue.next_sequence().map(WorkSequence::get),
            self.trace.next_sequence_for_test(),
        )
    }

    #[cfg(feature = "internal-test-seams")]
    pub(crate) const fn routed_trace_reservations_for_test(&self) -> usize {
        self.trace.reserved_records_for_test()
    }

    #[cfg(feature = "internal-test-seams")]
    pub(crate) const fn fail_routed_callback_bridge_for_test(&mut self) {
        self.routed_callback_bridge_failure_for_test = true;
    }

    #[cfg(feature = "internal-test-seams")]
    pub(crate) const fn fail_routed_semantic_default_for_test(&mut self) {
        self.routed_semantic_default_failure_for_test = true;
    }

    #[cfg(feature = "internal-test-seams")]
    pub(crate) const fn fail_routed_commit_for_test(&mut self) {
        self.routed_commit_failure_for_test = true;
    }
}
