use super::{AppRuntime, MountedNodeId, UiApp};

impl<App: UiApp> AppRuntime<App> {
    #[cfg(feature = "internal-test-seams")]
    #[doc(hidden)]
    pub const fn __seed_reconciliation_generation_for_test(&mut self, generation: u64) {
        self.runtime.seed_generation_for_test(generation);
    }

    #[cfg(feature = "internal-test-seams")]
    #[doc(hidden)]
    pub const fn __seed_next_work_sequence_for_test(&mut self, next: u64) {
        self.runtime.seed_next_work_sequence_for_test(next);
    }

    #[cfg(feature = "internal-test-seams")]
    #[doc(hidden)]
    pub const fn __seed_next_work_generation_for_test(&mut self, next: u64) {
        self.runtime.seed_next_work_generation_for_test(next);
    }

    #[cfg(feature = "internal-test-seams")]
    #[doc(hidden)]
    pub const fn __seed_next_trace_sequence_for_test(&mut self, next: u64) {
        self.runtime.seed_next_trace_sequence_for_test(next);
    }

    #[cfg(feature = "internal-test-seams")]
    #[doc(hidden)]
    pub fn __routed_sequence_state_for_test(&self) -> (Option<u64>, Option<u64>) {
        self.runtime.routed_sequence_state_for_test()
    }

    #[cfg(feature = "internal-test-seams")]
    #[doc(hidden)]
    pub const fn __routed_trace_reservations_for_test(&self) -> usize {
        self.runtime.routed_trace_reservations_for_test()
    }

    #[cfg(feature = "internal-test-seams")]
    #[doc(hidden)]
    #[must_use]
    pub fn __missing_target_for_test(&self) -> MountedNodeId {
        self.runtime.tree.missing_target_for_test()
    }

    #[cfg(feature = "internal-test-seams")]
    #[doc(hidden)]
    #[must_use]
    pub fn __stale_target_for_test(&self, live: &MountedNodeId) -> MountedNodeId {
        self.runtime.tree.stale_target_for_test(live)
    }

    #[cfg(feature = "internal-test-seams")]
    #[doc(hidden)]
    pub fn __corrupt_widget_state_for_test(&mut self, target: &MountedNodeId) {
        self.runtime.tree.corrupt_state_for_test(target);
    }

    #[cfg(feature = "internal-test-seams")]
    #[doc(hidden)]
    pub fn __break_routed_topology_for_test(&mut self, target: &MountedNodeId) {
        self.runtime.tree.break_parent_link_for_test(target);
    }

    #[cfg(feature = "internal-test-seams")]
    #[doc(hidden)]
    pub const fn __fail_routed_callback_bridge_for_test(&mut self) {
        self.runtime.fail_routed_callback_bridge_for_test();
    }

    #[cfg(feature = "internal-test-seams")]
    #[doc(hidden)]
    pub const fn __fail_routed_semantic_default_for_test(&mut self) {
        self.runtime.fail_routed_semantic_default_for_test();
    }

    #[cfg(feature = "internal-test-seams")]
    #[doc(hidden)]
    pub const fn __fail_routed_commit_for_test(&mut self) {
        self.runtime.fail_routed_commit_for_test();
    }

    #[cfg(feature = "internal-test-seams")]
    #[doc(hidden)]
    pub fn __live_work_record_count_for_test(&self) -> usize {
        self.runtime.live_work_record_count_for_test()
    }

    #[cfg(feature = "internal-test-seams")]
    #[doc(hidden)]
    pub fn __host_response_slot_count_for_test(&self) -> usize {
        self.runtime.host_response_slot_count_for_test()
    }

    #[cfg(feature = "internal-test-seams")]
    #[doc(hidden)]
    pub fn __send_task_slot_count_for_test(&self) -> usize {
        self.runtime.send_task_slot_count_for_test()
    }

    #[cfg(feature = "internal-test-seams")]
    #[doc(hidden)]
    pub fn __subscription_slot_count_for_test(&self) -> usize {
        self.runtime.subscription_slot_count_for_test()
    }

    #[cfg(feature = "internal-test-seams")]
    #[doc(hidden)]
    pub fn __completion_payload_count_for_test(&self) -> usize {
        self.runtime.completion_payload_count_for_test()
    }

    #[cfg(feature = "internal-test-seams")]
    #[doc(hidden)]
    pub const fn __send_task_mapper_count_for_test(&self) -> usize {
        self.runtime.send_task_mapper_count_for_test()
    }
}
