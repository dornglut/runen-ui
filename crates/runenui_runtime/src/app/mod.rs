//! Application-bound runtime operations.

mod focus;
mod surface;
#[cfg(feature = "internal-test-seams")]
mod testing;

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
};

pub struct AppRuntime<App: UiApp> {
    runtime: Runtime<App::State, App::Action, App::HostProtocol>,
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
            self.runtime.note_surface_focus_validation();
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
        self.runtime.shutdown()
    }
}
