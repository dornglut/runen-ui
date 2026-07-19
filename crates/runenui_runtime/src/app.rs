//! Application-bound runtime operations.

use core::marker::PhantomData;
use std::time::Duration;

use runenui_core::{CommandOrigin, SemanticCommand, UiApp, View};

use crate::{
    FocusState, FocusTargetResult, Key, KeyPhase, KeyboardEvent, MountedNodeId, MountedTreeIndex,
    PointerButton, PointerEvent, PointerPhase, PumpBudget, PumpReport, ReconciliationReport,
    RuntimeConfig, RuntimeStatus, ShutdownReport, SubmitActionResult, SurfaceBuildContext,
    SurfacePublication, Trace, WorkSequence,
    mounted::TargetStatus,
    policy::{KeyboardFocusResult, PointerFocusResult},
    pump,
    queue::ApplicationActionOrigin,
    runtime::Runtime,
    surface::{SurfaceCache, publish_mounted_surface_cached},
};

pub struct AppRuntime<App: UiApp> {
    runtime: Runtime<App::State, App::Action, App::HostProtocol>,
    surface_cache: Option<SurfaceCache>,
    phase_report: crate::SurfacePhaseReport,
    _app: PhantomData<fn() -> App>,
}

impl<App: UiApp> AppRuntime<App> {
    /// Mounts with deterministic runtime defaults.
    #[must_use]
    pub fn mount(state: App::State) -> Self {
        Self::mount_with_config(state, RuntimeConfig::default())
    }

    /// Mounts with explicit queue and trace limits.
    #[must_use]
    pub fn mount_with_config(state: App::State, config: RuntimeConfig) -> Self {
        let mut runtime = Runtime::mount(state, |state| App::root(state).into_element(), config);
        runtime.initialize_application_work::<App>();
        runtime.rearm_wake_if_needed();
        Self {
            runtime,
            surface_cache: None,
            phase_report: crate::SurfacePhaseReport::default(),
            _app: PhantomData,
        }
    }

    /// Appends one programmatic application action to the canonical FIFO.
    ///
    /// # Errors
    ///
    /// Returns the exact unaccepted action when the queue is full or the runtime
    /// is closed or terminal.
    pub fn submit_action(&mut self, action: App::Action) -> SubmitActionResult<App::Action> {
        self.runtime.submit_action(
            action,
            ApplicationActionOrigin::DirectSubmission,
            None,
            None,
        )
    }

    /// Appends one exact-target semantic command to the canonical FIFO.
    ///
    /// # Errors
    ///
    /// Returns every owned input when the runtime, target, queue, work sequence,
    /// or enabled canonical trace cannot accept the command.
    pub fn submit_command(
        &mut self,
        target: MountedNodeId,
        command: SemanticCommand,
        origin: CommandOrigin,
    ) -> Result<crate::CommandSubmission, crate::SubmitCommandError> {
        self.runtime.submit_command(target, command, origin)
    }

    /// Processes at most the requested number of canonical work envelopes.
    pub fn pump(&mut self, budget: PumpBudget) -> PumpReport {
        self.runtime.acknowledge_wake();
        let generation_before = self.runtime.report().generation();
        let report = pump::pump::<App>(&mut self.runtime, budget);
        if self.runtime.report().generation() != generation_before {
            self.phase_report =
                crate::SurfacePhaseReport::one(crate::SurfacePhase::FocusValidation);
        }
        self.runtime.rearm_wake_if_needed();
        report
    }

    /// Advances the deterministic headless monotonic clock.
    ///
    /// # Errors
    ///
    /// Returns an overflow error when the instant cannot be represented.
    pub fn advance_time(
        &self,
        duration: Duration,
    ) -> Result<crate::MonotonicInstant, crate::MonotonicTimeError> {
        self.runtime.advance_time(duration)
    }

    /// Replaces the send-task executor used by later start envelopes.
    pub fn set_send_task_executor(&mut self, executor: impl crate::SendTaskExecutor + 'static) {
        self.runtime.set_send_task_executor(executor);
    }

    /// Installs the callback used for coalesced runtime wake requests.
    pub fn set_wake_transport(&self, transport: impl crate::WakeTransport + 'static) {
        self.runtime.set_wake_transport(transport);
    }

    #[must_use]
    pub const fn last_send_task_start_outcome(&self) -> Option<crate::SendTaskStartOutcome> {
        self.runtime.last_send_task_start_outcome()
    }

    #[must_use]
    pub const fn last_timer_start_outcome(&self) -> Option<crate::TimerStartOutcome> {
        self.runtime.last_timer_start_outcome()
    }

    #[must_use]
    pub const fn last_timer_firing_outcome(&self) -> Option<crate::TimerFiringOutcome> {
        self.runtime.last_timer_firing_outcome()
    }

    /// Returns the commands exposed by processed host-request start envelopes.
    #[must_use]
    pub fn pending_host_requests(&self) -> Vec<crate::HostRequestRef<'_, App::HostProtocol>> {
        self.runtime.pending_host_requests()
    }

    /// Queues one validated host response for later non-reentrant mapping.
    ///
    /// # Errors
    ///
    /// Returns the exact response when the token is foreign or stale, the
    /// response kind mismatches, the queue is full, or the runtime cannot accept work.
    pub fn complete_host_request(
        &mut self,
        token: &crate::HostRequestToken,
        response: <App::HostProtocol as runenui_core::HostProtocol>::Response,
    ) -> Result<
        WorkSequence,
        crate::HostResponseError<<App::HostProtocol as runenui_core::HostProtocol>::Response>,
    > {
        self.runtime.complete_host_request(token, response)
    }

    /// Creates a concrete send-capable response completion for cross-thread delivery.
    ///
    /// # Errors
    ///
    /// Returns the exact response when the token is foreign or stale or the
    /// runtime is closed or terminal.
    pub fn host_response_completion(
        &mut self,
        token: &crate::HostRequestToken,
        response: <App::HostProtocol as runenui_core::HostProtocol>::Response,
    ) -> Result<
        crate::HostResponseCompletion,
        crate::HostResponseError<<App::HostProtocol as runenui_core::HostProtocol>::Response>,
    >
    where
        <App::HostProtocol as runenui_core::HostProtocol>::Response: Send + 'static,
    {
        self.runtime.host_response_completion(token, response)
    }

    /// Queues cancellation for one exact live host request generation.
    ///
    /// # Errors
    ///
    /// Returns a structured token, saturation, or runtime-status error when
    /// cancellation cannot be accepted.
    pub fn cancel_host_request(
        &mut self,
        token: &crate::HostRequestToken,
    ) -> Result<WorkSequence, crate::HostRequestCancelError> {
        self.runtime.cancel_host_request(token)
    }

    #[must_use]
    pub fn subscription_diagnostics(&self) -> &[crate::SubscriptionDiagnostic] {
        self.runtime.subscription_diagnostics()
    }

    #[must_use]
    pub fn take_redraw_request(&mut self) -> Option<crate::RedrawRequest> {
        self.runtime.take_redraw_request()
    }

    /// Acknowledges successful publication for one consumed dirty revision.
    ///
    /// # Errors
    ///
    /// Returns a structured error for a foreign or impossible future revision.
    pub fn acknowledge_redraw(
        &mut self,
        request: &crate::RedrawRequest,
    ) -> Result<(), crate::RedrawAcknowledgeError> {
        self.runtime.acknowledge_redraw(request)
    }

    /// Uses a host-provided monotonic clock for future readiness checks.
    pub fn set_monotonic_clock(&mut self, clock: impl crate::MonotonicClock + 'static) {
        self.runtime.set_monotonic_clock(clock);
    }

    /// Returns the runtime's read-only execution status.
    #[must_use]
    pub const fn status(&self) -> RuntimeStatus {
        self.runtime.status()
    }

    /// Explicitly and idempotently closes the runtime.
    pub fn shutdown(&mut self) -> ShutdownReport {
        let report = self.runtime.shutdown();
        self.surface_cache = None;
        report
    }

    #[must_use]
    pub fn index(&mut self) -> MountedTreeIndex<'_, App::Action> {
        self.runtime.tree.index()
    }

    pub fn set_focus(&mut self, id: MountedNodeId) -> FocusTargetResult {
        if !matches!(self.status(), RuntimeStatus::Running) {
            return FocusTargetResult::NotFocusable;
        }
        match self.runtime.tree.target_status(&id) {
            TargetStatus::Foreign => FocusTargetResult::ForeignRuntime,
            TargetStatus::Stale | TargetStatus::Missing => FocusTargetResult::StaleTarget,
            TargetStatus::Live => {
                let activation = self.runtime.tree.activation(&id);
                if activation
                    .is_ok_and(|activation| activation.enabled() && activation.is_actionable())
                {
                    self.runtime.set_focus(id);
                    FocusTargetResult::Focused
                } else {
                    FocusTargetResult::NotFocusable
                }
            }
        }
    }

    pub fn focus_first(&mut self) -> Option<MountedNodeId> {
        if !matches!(self.status(), RuntimeStatus::Running) {
            return None;
        }
        let id = self.index().first_focusable_node().map(|n| n.id().clone());
        self.apply_focus_result(id)
    }
    pub fn focus_last(&mut self) -> Option<MountedNodeId> {
        if !matches!(self.status(), RuntimeStatus::Running) {
            return None;
        }
        let id = self.index().last_focusable_node().map(|n| n.id().clone());
        self.apply_focus_result(id)
    }
    pub fn focus_next(&mut self) -> Option<MountedNodeId> {
        if !matches!(self.status(), RuntimeStatus::Running) {
            return None;
        }
        let current = self.focus().focused_node().cloned();
        let id = {
            let index = self.index();
            current.as_ref().map_or_else(
                || index.first_focusable_node().map(|n| n.id().clone()),
                |current| {
                    index
                        .next_focusable_after(current)
                        .or_else(|| index.first_focusable_node())
                        .map(|n| n.id().clone())
                },
            )
        };
        self.apply_focus_result(id)
    }
    pub fn focus_previous(&mut self) -> Option<MountedNodeId> {
        if !matches!(self.status(), RuntimeStatus::Running) {
            return None;
        }
        let current = self.focus().focused_node().cloned();
        let id = {
            let index = self.index();
            current.as_ref().map_or_else(
                || index.last_focusable_node().map(|n| n.id().clone()),
                |current| {
                    index
                        .previous_focusable_before(current)
                        .or_else(|| index.last_focusable_node())
                        .map(|n| n.id().clone())
                },
            )
        };
        self.apply_focus_result(id)
    }
    fn apply_focus_result(&mut self, id: Option<MountedNodeId>) -> Option<MountedNodeId> {
        if let Some(id) = id {
            self.runtime.set_focus(id.clone());
            Some(id)
        } else {
            self.runtime.clear_focus();
            None
        }
    }
    pub fn clear_focus(&mut self) {
        if matches!(self.status(), RuntimeStatus::Running) {
            self.runtime.clear_focus();
        }
    }
    #[must_use]
    pub const fn focus(&self) -> &FocusState {
        self.runtime.focus()
    }
    #[must_use]
    pub const fn state(&self) -> &App::State {
        self.runtime.state()
    }
    #[must_use]
    pub const fn trace(&self) -> &Trace {
        self.runtime.trace()
    }
    #[must_use]
    pub const fn reconciliation_report(&self) -> &ReconciliationReport {
        self.runtime.report()
    }
    #[must_use]
    pub fn publish_surface(&mut self, context: &SurfaceBuildContext<'_>) -> SurfacePublication {
        let redraw = self.runtime.take_redraw_request();
        let (publication, report) = publish_mounted_surface_cached(
            &mut self.runtime.tree,
            context,
            &mut self.surface_cache,
        );
        self.phase_report = report;
        if let Some(redraw) = redraw {
            self.runtime
                .acknowledge_redraw(&redraw)
                .unwrap_or_else(|_| unreachable!("runtime-issued redraw request remains local"));
        }
        publication
    }
    #[must_use]
    pub const fn last_surface_phase_report(&self) -> &crate::SurfacePhaseReport {
        &self.phase_report
    }
    #[must_use]
    pub fn into_state(self) -> App::State {
        self.runtime.into_state()
    }

    pub fn handle_keyboard_focus(&mut self, event: &KeyboardEvent) -> KeyboardFocusResult {
        if event.phase() != KeyPhase::Pressed || !matches!(event.key(), Key::Tab) {
            return KeyboardFocusResult::Ignored;
        }
        let id = if event.modifiers().shift() {
            self.focus_previous()
        } else {
            self.focus_next()
        };
        id.map_or(
            KeyboardFocusResult::NoFocusableNode,
            KeyboardFocusResult::Moved,
        )
    }
    pub fn handle_pointer_focus(&mut self, event: &PointerEvent) -> PointerFocusResult {
        if event.phase() != PointerPhase::Pressed || event.button() != Some(PointerButton::Primary)
        {
            return PointerFocusResult::Ignored;
        }
        let Some(id) = event.target().cloned() else {
            return PointerFocusResult::NoTarget;
        };
        match self.set_focus(id.clone()) {
            FocusTargetResult::Focused => PointerFocusResult::Moved(id),
            FocusTargetResult::NotFocusable => PointerFocusResult::NotFocusable,
            FocusTargetResult::StaleTarget | FocusTargetResult::ForeignRuntime => {
                PointerFocusResult::NotFound
            }
        }
    }
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
