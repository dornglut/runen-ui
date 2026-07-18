//! Application-bound runtime operations and proof activation.

use core::marker::PhantomData;
use std::time::Duration;

use runenui_core::{ElementId, UiApp, View};

use crate::{
    FocusState, FocusTargetResult, InputEvent, Key, KeyPhase, KeyboardEvent, MountedNodeId,
    MountedTreeIndex, PointerButton, PointerEvent, PointerPhase, PumpBudget, PumpReport,
    ReconciliationReport, RuntimeConfig, RuntimeError, RuntimeStatus, ShutdownReport,
    SubmitActionResult, SurfaceBuildContext, SurfacePublication, Trace, TraceTarget, WorkSequence,
    mounted::TargetStatus,
    policy::{
        InputEventResult, KeyboardActivationResult, KeyboardFocusResult, PointerActivationResult,
        PointerFocusResult,
    },
    pump,
    queue::ApplicationActionOrigin,
    runtime::Runtime,
    surface::{SurfaceCache, publish_mounted_surface_cached},
};

/// Bounded runtime authority that may refuse conservative activation admission.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ActivationCommit {
    pub first_sequence: WorkSequence,
    pub primary_action_sequence: Option<WorkSequence>,
    pub queued_envelopes: usize,
}

/// Result of one proof-level mounted activation attempt.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ActivationCapacity {
    /// Canonical waiting-envelope FIFO capacity.
    WaitingEnvelopes,
    /// Live local-task registry capacity.
    LocalTasks,
    /// Live send-task registry capacity.
    SendTasks,
    /// Live timer registry capacity.
    Timers,
}

/// Result of one proof-level mounted activation attempt.
#[non_exhaustive]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ActivationResult {
    /// One or more canonical envelopes committed atomically.
    Queued(ActivationCommit),
    /// Semantic widget state/invalidation changed without queued work.
    Activated,
    /// The widget authoritatively reported no state, invalidation, or output effect.
    NoEffect,
    /// Admission was rejected by the identified bounded authority before mutation.
    Saturated(ActivationCapacity),
    Closed,
    Terminal(crate::RuntimeTerminalReason),
    NotFound,
    NotActivatable,
    Disabled,
    AmbiguousId,
    InvalidId,
    StaleTarget,
    ForeignRuntime,
    RuntimeError(RuntimeError),
}

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
            TargetStatus::Stale => FocusTargetResult::StaleTarget,
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

    /// Activates a unique authored ID through the queue-backed proof path.
    pub fn activate(&mut self, id: impl AsRef<str>) -> ActivationResult {
        match self.status() {
            RuntimeStatus::Closed => return ActivationResult::Closed,
            RuntimeStatus::Terminal(reason) => return ActivationResult::Terminal(reason),
            RuntimeStatus::Running => {}
        }
        let Ok(id) = ElementId::new(id.as_ref()) else {
            return ActivationResult::InvalidId;
        };
        let node_id = {
            let index = self.index();
            if index.diagnostics().iter().any(|diagnostic| {
                diagnostic.kind() == crate::DuplicateIdentityKind::ElementId
                    && diagnostic.value() == id.as_str()
            }) {
                return ActivationResult::AmbiguousId;
            }
            index.node_by_authored_id(&id).map(|node| node.id().clone())
        };
        node_id.map_or(ActivationResult::NotFound, |id| {
            self.activate_node_with_origin(&id, ApplicationActionOrigin::MountedActivation)
        })
    }

    /// Activates one exact mounted generation through the canonical queue.
    pub fn activate_node(&mut self, id: &MountedNodeId) -> ActivationResult {
        self.activate_node_with_origin(id, ApplicationActionOrigin::MountedActivation)
    }

    fn activate_node_with_origin(
        &mut self,
        id: &MountedNodeId,
        origin: ApplicationActionOrigin,
    ) -> ActivationResult {
        match self.status() {
            RuntimeStatus::Closed => return ActivationResult::Closed,
            RuntimeStatus::Terminal(reason) => return ActivationResult::Terminal(reason),
            RuntimeStatus::Running => {}
        }
        match self.runtime.tree.target_status(id) {
            TargetStatus::Foreign => return ActivationResult::ForeignRuntime,
            TargetStatus::Stale => return ActivationResult::StaleTarget,
            TargetStatus::Live => {}
        }
        let (activation, target) = {
            let Ok(activation) = self.runtime.tree.activation_probe(id) else {
                return ActivationResult::RuntimeError(RuntimeError::WidgetStatePayloadMismatch);
            };
            let authored = self
                .runtime
                .tree
                .node(id)
                .and_then(|node| node.authored_id.clone());
            (activation, TraceTarget::new(id.clone(), authored))
        };
        if !activation.is_actionable() {
            return ActivationResult::NotActivatable;
        }
        if !activation.enabled() {
            return ActivationResult::Disabled;
        }
        if let Err(error) = self.runtime.activation_preflight(&target) {
            return match error {
                crate::runtime::ActivationPreflightError::Saturated(capacity) => {
                    ActivationResult::Saturated(capacity)
                }
                crate::runtime::ActivationPreflightError::Status(RuntimeStatus::Running) => {
                    unreachable!("running status is not a preflight failure")
                }
                crate::runtime::ActivationPreflightError::Status(RuntimeStatus::Closed) => {
                    ActivationResult::Closed
                }
                crate::runtime::ActivationPreflightError::Status(RuntimeStatus::Terminal(
                    reason,
                )) => ActivationResult::Terminal(reason),
            };
        }
        let Ok(output) = self.runtime.tree.activate(id) else {
            return ActivationResult::RuntimeError(RuntimeError::WidgetStatePayloadMismatch);
        };
        let invalidation = output.invalidation;
        let state_changed = output.state_changed;
        let subscription_invalidation = output.subscription_invalidation;
        let Ok(commit) = self
            .runtime
            .commit_activation(id.clone(), output, &target, origin)
        else {
            if matches!(self.status(), RuntimeStatus::Running) {
                self.runtime
                    .enter_terminal(crate::RuntimeTerminalReason::Poisoned, 0);
            }
            return match self.status() {
                RuntimeStatus::Terminal(reason) => ActivationResult::Terminal(reason),
                RuntimeStatus::Closed => ActivationResult::Closed,
                RuntimeStatus::Running => unreachable!("activation commit failure terminalizes"),
            };
        };
        if invalidation.contains(runenui_core::WidgetInvalidation::INTERACTION) {
            let focused = self.runtime.focus().focused_node().cloned();
            if focused.as_ref().is_some_and(|focused| {
                self.runtime
                    .tree
                    .activation(focused)
                    .map_or(true, |activation| {
                        !activation.enabled() || !activation.is_actionable()
                    })
            }) {
                self.runtime.clear_focus();
            }
            self.runtime.tree.finish_focus_validation();
            self.phase_report =
                crate::SurfacePhaseReport::one(crate::SurfacePhase::FocusValidation);
        }
        if crate::mounted::publication_is_dirty(invalidation) {
            self.runtime.request_redraw();
        }
        if let Some(commit) = commit {
            self.runtime.external_queue_commit_accepted();
            return ActivationResult::Queued(commit);
        }
        if invalidation.is_empty() && !state_changed && !subscription_invalidation {
            match self.status() {
                RuntimeStatus::Terminal(reason) => ActivationResult::Terminal(reason),
                RuntimeStatus::Closed => ActivationResult::Closed,
                RuntimeStatus::Running => ActivationResult::NoEffect,
            }
        } else {
            ActivationResult::Activated
        }
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
    pub fn handle_keyboard_activation(
        &mut self,
        event: &KeyboardEvent,
    ) -> KeyboardActivationResult {
        if event.phase() != KeyPhase::Pressed || !matches!(event.key(), Key::Enter | Key::Space) {
            return KeyboardActivationResult::Ignored;
        }
        let Some(id) = self.focus().focused_node().cloned() else {
            return KeyboardActivationResult::NoFocusedNode;
        };
        KeyboardActivationResult::Handled(
            self.activate_node_with_origin(&id, ApplicationActionOrigin::KeyboardActivation),
        )
    }
    pub fn handle_pointer_activation(&mut self, event: &PointerEvent) -> PointerActivationResult {
        if event.phase() != PointerPhase::Pressed || event.button() != Some(PointerButton::Primary)
        {
            return PointerActivationResult::Ignored;
        }
        let Some(id) = event.target().cloned() else {
            return PointerActivationResult::NoTarget;
        };
        PointerActivationResult::Handled(
            self.activate_node_with_origin(&id, ApplicationActionOrigin::PointerActivation),
        )
    }
    pub fn handle_input_event(&mut self, event: &InputEvent) -> InputEventResult {
        match event {
            InputEvent::Pointer(event) => {
                let focus = self.handle_pointer_focus(event);
                let activation = self.handle_pointer_activation(event);
                if focus == PointerFocusResult::Ignored
                    && activation == PointerActivationResult::Ignored
                {
                    InputEventResult::Ignored
                } else {
                    InputEventResult::Pointer { focus, activation }
                }
            }
            InputEvent::Keyboard(event) => {
                let focus = self.handle_keyboard_focus(event);
                if focus != KeyboardFocusResult::Ignored {
                    return InputEventResult::KeyboardFocus(focus);
                }
                let activation = self.handle_keyboard_activation(event);
                if activation == KeyboardActivationResult::Ignored {
                    InputEventResult::Ignored
                } else {
                    InputEventResult::KeyboardActivation(activation)
                }
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
