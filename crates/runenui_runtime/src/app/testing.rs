use runenui_core::SurfaceInputContext;

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
    pub const fn __seed_next_composition_generation_for_test(&mut self, next: Option<u64>) {
        self.runtime.seed_next_composition_generation_for_test(next);
    }

    #[cfg(feature = "internal-test-seams")]
    #[doc(hidden)]
    #[must_use]
    pub fn __composition_generation_for_test(&self, value: u64) -> crate::CompositionGeneration {
        self.runtime.composition_generation_for_test(value)
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
    pub fn __routed_trace_reservations_for_test(&self) -> usize {
        self.runtime.routed_trace_reservations_for_test()
    }

    #[cfg(feature = "internal-test-seams")]
    #[doc(hidden)]
    pub const fn __surface_publication_trace_reserved_for_test(&self) -> bool {
        self.runtime.surface_publication_trace_reserved_for_test()
    }

    #[cfg(feature = "internal-test-seams")]
    #[doc(hidden)]
    #[must_use]
    pub fn __mounted_identity_parts_for_test(&self, id: &MountedNodeId) -> Option<(u32, u64)> {
        self.runtime
            .tree
            .runtime_namespace()
            .__runtime_mounted_parts(id)
    }

    #[cfg(feature = "internal-test-seams")]
    #[doc(hidden)]
    #[must_use]
    pub fn __surface_context_for_test(
        &self,
        surface_slot: u32,
        surface_generation: u64,
        coordinate_revision: u64,
        hit_test_generation: u64,
    ) -> SurfaceInputContext {
        self.runtime.surface_context_for_test(
            surface_slot,
            surface_generation,
            coordinate_revision,
            hit_test_generation,
        )
    }

    #[cfg(feature = "internal-test-seams")]
    #[doc(hidden)]
    pub fn __replace_surface_snapshot_target_for_test(
        &mut self,
        context: &SurfaceInputContext,
        original: &MountedNodeId,
        replacement: MountedNodeId,
    ) {
        self.runtime
            .replace_surface_snapshot_target_for_test(context, original, replacement);
    }

    #[cfg(feature = "internal-test-seams")]
    #[doc(hidden)]
    pub fn __replace_current_focus_geometry_for_test(
        &mut self,
        geometry: &[(MountedNodeId, [f32; 4])],
    ) {
        self.runtime
            .replace_current_focus_geometry_for_test(geometry);
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
