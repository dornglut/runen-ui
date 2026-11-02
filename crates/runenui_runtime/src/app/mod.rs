//! Application-bound runtime operations.

mod focus;
#[cfg(test)]
mod input_arbitration_tests;
mod surface;
#[cfg(feature = "internal-test-seams")]
mod testing;

use core::marker::PhantomData;
use std::time::Duration;

use runenui_core::__runtime::SubscriptionSource;
use runenui_core::{
    CommandOrigin, ElementId, IntoEffects, SemanticCommand, UiApp, View, WorkFamily,
};

use crate::{
    FocusState, FontFamilyName, FontRegistrationError, FontSourcePolicy, FontSourceSnapshot,
    GenericFamilyMappingError, GenericFontFamily, InputObservationError, InputPumpBatch,
    InputShutdownBatch, MountedNodeId, MountedTreeIndex, PumpBudget, ReconciliationReport,
    RuntimeConfig, RuntimeStatus, SubmitActionResult, SurfaceBuildContext, SurfacePublication,
    Trace, TraceActionCategory, WorkSequence, pump, runtime::PreparedInitialApplicationWork,
    runtime::Runtime, transaction::TransactionLedger,
};

/// Why initial application construction failed.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum InitialAdmissionErrorKind {
    /// The configured canonical queue cannot reserve the declared initial frontier.
    Capacity,
    /// A declared resident work family cannot fit the configured registry.
    WorkCapacity(crate::WorkCapacityFamily),
    /// Canonical work-sequence authority cannot reserve the initial frontier.
    SequenceExhausted,
    /// Work-generation authority cannot reserve the declared initial starts.
    WorkGenerationExhausted,
    /// The initial producer profile cannot admit its mandatory mounted root work.
    InvalidProducerProfile,
    /// Mounted identity authority cannot represent the authored initial tree.
    MountedIdentityExhausted,
    /// A callback exceeded its authored declaration after initial admission.
    ProducerContractViolation(runenui_core::ProducerContractViolation),
}

/// Owned initial-construction refusal returning the same state value and config.
///
/// Capacity and declared resource feasibility are checked before initial
/// producers run. Producer-profile violations can also be detected while
/// constructing the authored tree; arbitrary callback effects are not rolled back.
pub struct InitialAdmissionError<State> {
    refused: Box<(State, RuntimeConfig)>,
    kind: InitialAdmissionErrorKind,
    frontier_requirement: Option<(usize, usize)>,
}

impl<State> InitialAdmissionError<State> {
    pub(crate) fn new(
        state: State,
        config: RuntimeConfig,
        kind: InitialAdmissionErrorKind,
    ) -> Self {
        Self {
            refused: Box::new((state, config)),
            kind,
            frontier_requirement: None,
        }
    }

    pub(crate) const fn with_frontier_requirement(
        mut self,
        required: usize,
        declared: usize,
    ) -> Self {
        self.frontier_requirement = Some((required, declared));
        self
    }

    /// Returns the pre-admission refusal category.
    #[must_use]
    pub const fn kind(&self) -> InitialAdmissionErrorKind {
        self.kind
    }

    /// Returns the required and declared root queue credits for a profile refusal.
    #[must_use]
    pub const fn frontier_requirement(&self) -> Option<(usize, usize)> {
        self.frontier_requirement
    }

    /// Borrows the returned application state.
    #[must_use]
    pub const fn state(&self) -> &State {
        &self.refused.0
    }

    /// Returns the exact config used for this construction attempt.
    #[must_use]
    pub const fn config(&self) -> RuntimeConfig {
        self.refused.1
    }

    /// Returns the state and config for a corrected retry.
    #[must_use]
    pub fn into_parts(self) -> (State, RuntimeConfig) {
        *self.refused
    }
}

impl<State> core::fmt::Debug for InitialAdmissionError<State> {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_struct("InitialAdmissionError")
            .field("kind", &self.kind)
            .field("frontier_requirement", &self.frontier_requirement)
            .finish_non_exhaustive()
    }
}

impl<State> core::fmt::Display for InitialAdmissionError<State> {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        match self.kind {
            InitialAdmissionErrorKind::ProducerContractViolation(cause) => write!(
                formatter,
                "initial producer declaration violated: {cause:?}"
            ),
            InitialAdmissionErrorKind::Capacity => {
                formatter.write_str("initial work frontier exceeds queue capacity")
            }
            InitialAdmissionErrorKind::WorkCapacity(family) => {
                write!(
                    formatter,
                    "initial {family:?} residency exceeds runtime capacity"
                )
            }
            InitialAdmissionErrorKind::SequenceExhausted => {
                formatter.write_str("initial work sequence authority is exhausted")
            }
            InitialAdmissionErrorKind::WorkGenerationExhausted => {
                formatter.write_str("initial work generation authority is exhausted")
            }
            InitialAdmissionErrorKind::InvalidProducerProfile => {
                formatter.write_str(
                    "initial producer profile cannot satisfy runtime work requirements",
                )?;
                if let Some((required, declared)) = self.frontier_requirement {
                    write!(
                        formatter,
                        " (root queue credits required: {required}; declared: {declared})"
                    )?;
                }
                Ok(())
            }
            InitialAdmissionErrorKind::MountedIdentityExhausted => {
                formatter.write_str("initial mounted identity capacity is exhausted")
            }
        }
    }
}

impl<State> core::error::Error for InitialAdmissionError<State> {}

pub struct AppRuntime<App: UiApp> {
    runtime: Runtime<App::State, App::Action, App::HostProtocol>,
    _app: PhantomData<fn() -> App>,
}

impl<App: UiApp> AppRuntime<App> {
    /// Mounts with deterministic runtime defaults.
    ///
    /// # Errors
    ///
    /// Returns the caller-owned state and default configuration in
    /// [`InitialAdmissionError`] when initial admission fails.
    pub fn mount(state: App::State) -> Result<Self, InitialAdmissionError<App::State>> {
        Self::mount_with_config(state, RuntimeConfig::default())
    }

    /// Mounts with explicit runtime configuration.
    ///
    /// # Errors
    ///
    /// Returns the caller-owned state and configuration in
    /// [`InitialAdmissionError`] when the declared initial producer cannot be
    /// admitted or initial mounted/work setup fails.
    pub fn mount_with_config(
        state: App::State,
        config: RuntimeConfig,
    ) -> Result<Self, InitialAdmissionError<App::State>> {
        let initial_producer_budget = App::initial_producer_budget(&state);
        let mut runtime = Runtime::mount(
            state,
            |state| App::root(state).into_element(),
            config,
            initial_producer_budget,
            Self::prepare_initial_application_work,
        )?;
        runtime.trace_action_labeler = Some(App::trace_action_label);
        runtime.action_producer_budget = Some(App::action_producer_budget);
        runtime.interaction_producer_budget = Some(App::interaction_producer_budget);
        runtime.initialize_application_work();
        if let RuntimeStatus::Terminal(crate::RuntimeTerminalReason::ProducerContractViolation(
            cause,
        )) = runtime.status
        {
            return Err(InitialAdmissionError::new(
                runtime.into_state(),
                config,
                InitialAdmissionErrorKind::ProducerContractViolation(cause),
            ));
        }
        runtime.rearm_wake_if_needed();
        Ok(Self {
            runtime,
            _app: PhantomData,
        })
    }

    fn prepare_initial_application_work(
        state: &App::State,
        budget: runenui_core::ProducerBudget,
    ) -> Result<
        PreparedInitialApplicationWork<App::Action, App::HostProtocol>,
        InitialAdmissionErrorKind,
    > {
        let application = TransactionLedger::collect(
            App::initial_effects(state).into_effects(),
            budget.application_outputs(),
        )
        .map_err(
            |crate::transaction::TransactionOutputError::Full { attempted, .. }| {
                InitialAdmissionErrorKind::ProducerContractViolation(
                    runenui_core::ProducerContractViolation {
                        kind: runenui_core::ProducerViolationKind::DeclarationExceeded,
                        phase: runenui_core::ProducerPhase::InitialEffects,
                        component: runenui_core::BudgetComponent::ApplicationOutputs,
                        declared: budget.application_outputs(),
                        attempted: Some(attempted),
                    },
                )
            },
        )?;
        if application.len() > budget.application_outputs() {
            return Err(InitialAdmissionErrorKind::InvalidProducerProfile);
        }
        let mut subscriptions = runenui_core::SubscriptionSet::__runtime_new_bounded(
            budget.application_subscriptions(),
        );
        let effect_starts = application
            .work_start_budget()
            .ok_or(InitialAdmissionErrorKind::InvalidProducerProfile)?;
        let _remaining = crate::transaction::remaining_work_starts(
            budget.work_starts(),
            effect_starts,
            runenui_core::ProducerPhase::InitialEffects,
        )
        .map_err(InitialAdmissionErrorKind::ProducerContractViolation)?;
        App::subscriptions(state, &mut subscriptions);
        if let Some(cause) = subscriptions.producer_violation() {
            return Err(InitialAdmissionErrorKind::ProducerContractViolation(cause));
        }
        if subscriptions.__runtime_overflowed() {
            return Err(InitialAdmissionErrorKind::InvalidProducerProfile);
        }
        let subscriptions = subscriptions.__runtime_into_declarations();
        if subscriptions.len() > budget.application_subscriptions() {
            return Err(InitialAdmissionErrorKind::InvalidProducerProfile);
        }
        let local_tasks = subscriptions
            .iter()
            .filter(|source| matches!(source.source, SubscriptionSource::Local(_)))
            .count();
        let send_tasks = subscriptions.len() - local_tasks;
        let total_starts = runenui_core::WorkStartBudget::new(
            effect_starts
                .for_family(WorkFamily::LocalTask)
                .checked_add(local_tasks)
                .ok_or(InitialAdmissionErrorKind::InvalidProducerProfile)?,
            effect_starts
                .for_family(WorkFamily::SendTask)
                .checked_add(send_tasks)
                .ok_or(InitialAdmissionErrorKind::InvalidProducerProfile)?,
            effect_starts.for_family(WorkFamily::Timer),
            effect_starts.for_family(WorkFamily::HostRequest),
            effect_starts.for_family(WorkFamily::FrameworkService),
        );
        let promised_starts = budget.work_starts();
        for family in [
            WorkFamily::LocalTask,
            WorkFamily::SendTask,
            WorkFamily::Timer,
            WorkFamily::HostRequest,
            WorkFamily::FrameworkService,
        ] {
            let declared = promised_starts.for_family(family);
            let attempted = total_starts.for_family(family);
            if attempted > declared {
                return Err(InitialAdmissionErrorKind::ProducerContractViolation(
                    runenui_core::ProducerContractViolation {
                        kind: runenui_core::ProducerViolationKind::DeclarationExceeded,
                        phase: runenui_core::ProducerPhase::InitialEffects,
                        component: runenui_core::BudgetComponent::WorkStarts(family),
                        declared,
                        attempted: Some(attempted),
                    },
                ));
            }
        }
        Ok(PreparedInitialApplicationWork {
            application,
            subscriptions,
        })
    }

    /// Registers immutable bundled font bytes with the runtime-owned text authority.
    ///
    /// # Errors
    ///
    /// Returns a text-source registration error when the bytes contain no registerable faces or
    /// when the font-source revision cannot advance.
    pub fn register_text_font_bytes(
        &mut self,
        bytes: Vec<u8>,
    ) -> Result<usize, FontRegistrationError> {
        self.runtime.register_text_font_bytes(bytes)
    }

    /// Replaces one runtime text generic-family mapping with already-registered named families.
    ///
    /// # Errors
    ///
    /// Returns a mapping error when a named family is unavailable, the generic family has no
    /// reviewed backend mapping, or the font-source revision cannot advance.
    pub fn set_text_generic_family_mapping(
        &mut self,
        generic: GenericFontFamily,
        families: &[FontFamilyName],
    ) -> Result<bool, GenericFamilyMappingError> {
        self.runtime
            .set_text_generic_family_mapping(generic, families)
    }

    /// Returns the exact cache-compatibility snapshot of the runtime-owned font-source universe.
    #[must_use]
    pub fn text_font_source_snapshot(&self) -> FontSourceSnapshot {
        self.runtime.text_font_source_snapshot()
    }

    /// Returns the runtime-owned text authority's ambient-font policy.
    #[must_use]
    pub const fn text_font_source_policy(&self) -> FontSourcePolicy {
        self.runtime.text_font_source_policy()
    }

    /// Appends one programmatic application action to the canonical FIFO.
    ///
    /// # Errors
    ///
    /// Returns the exact unaccepted action when the queue is full or the runtime
    /// is closed or terminal.
    pub fn submit_action(&mut self, action: App::Action) -> SubmitActionResult<App::Action> {
        let budget = App::action_producer_budget(self.runtime.state(), &action);
        self.runtime.submit_profiled_action(
            action,
            budget,
            TraceActionCategory::DirectSubmission,
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

    /// Appends one immutable pointer event to the canonical FIFO.
    ///
    /// Acceptance performs no callback, hit-test mutation, focus change, or
    /// interaction-state mutation. Processing occurs later through [`Self::pump`].
    ///
    /// # Errors
    ///
    /// Returns the exact unaccepted event when the queue, work sequence, trace,
    /// or runtime status cannot accept it.
    pub fn submit_pointer(
        &mut self,
        event: crate::PointerEvent,
    ) -> Result<crate::PointerSubmission, crate::SubmitPointerError> {
        self.runtime.submit_pointer(event)
    }

    /// Appends a host-neutral keyboard event bound to the current focused lifetime.
    ///
    /// # Errors
    ///
    /// Returns the exact event when ingress cannot be admitted.
    pub fn submit_keyboard(
        &mut self,
        event: crate::KeyboardEvent,
    ) -> Result<crate::KeyboardSubmission, crate::SubmitKeyboardError> {
        self.runtime.submit_keyboard(event)
    }

    /// Appends committed Unicode text bound to the current text-capable focused lifetime.
    ///
    /// # Errors
    ///
    /// Returns the exact event when focus or ingress admission rejects it.
    pub fn submit_text(
        &mut self,
        event: crate::CommittedTextEvent,
    ) -> Result<crate::TextSubmission, crate::SubmitTextError> {
        self.runtime.submit_text(event)
    }

    /// Starts a composition lifetime bound to the current composition-capable focus.
    ///
    /// # Errors
    ///
    /// Returns a structured composition rejection without allocating a generation.
    pub fn start_composition(
        &mut self,
        device_id: Option<crate::InputDeviceId>,
    ) -> Result<crate::CompositionStartSubmission, crate::SubmitCompositionStartError> {
        self.runtime.start_composition(device_id)
    }

    /// Queues a preedit update for an active or pending composition generation.
    ///
    /// # Errors
    ///
    /// Returns the exact event when its generation or ingress is rejected.
    pub fn submit_composition_update(
        &mut self,
        generation: crate::CompositionGeneration,
        preedit: String,
        range: Option<crate::CompositionRange>,
    ) -> Result<crate::CompositionSubmission, crate::SubmitCompositionError> {
        self.runtime
            .submit_composition_update(generation, preedit, range)
    }

    /// Queues lifecycle closure for an active or pending composition generation.
    ///
    /// # Errors
    ///
    /// Returns the exact event when its generation or ingress is rejected.
    pub fn submit_composition_end(
        &mut self,
        generation: crate::CompositionGeneration,
    ) -> Result<crate::CompositionSubmission, crate::SubmitCompositionError> {
        self.runtime.submit_composition_end(generation)
    }

    /// Queues explicit cancellation for an active or pending composition generation.
    ///
    /// # Errors
    ///
    /// Returns the exact event when its generation or ingress is rejected.
    pub fn cancel_composition(
        &mut self,
        generation: crate::CompositionGeneration,
    ) -> Result<crate::CompositionSubmission, crate::SubmitCompositionError> {
        self.runtime.cancel_composition(generation)
    }

    /// Resolves a unique authored ID in logical preorder and queues its command.
    ///
    /// # Errors
    ///
    /// Returns the original authored request when resolution or command ingress rejects it.
    pub fn submit_automation_command(
        &mut self,
        authored_id: ElementId,
        command: SemanticCommand,
    ) -> Result<crate::AutomationSubmission, crate::SubmitAutomationError> {
        self.runtime
            .submit_public_automation_command(authored_id, command)
    }

    /// Observes exact immutable current UI input ownership; never host gameplay policy.
    ///
    /// This method also observes accepted synchronous focus/composition/publication
    /// changes that can occur without a canonical pump envelope.
    ///
    /// # Errors
    /// Returns a structured error if a bounded snapshot cannot be allocated or
    /// its non-wrapping revision cannot advance.
    pub fn input_ownership(
        &mut self,
    ) -> Result<crate::InputOwnershipSnapshot, crate::InputObservationError> {
        self.runtime.input_ownership()
    }

    /// Processes the canonical FIFO and returns ordered public input observations.
    ///
    /// # Errors
    /// Returns an observation-admission error if the required bounded capacity
    /// cannot be reserved before processing.
    pub fn pump(&mut self, budget: PumpBudget) -> Result<InputPumpBatch, InputObservationError> {
        self.runtime.acknowledge_wake();
        let generation_before = self.runtime.report().generation();
        let batch = pump::pump_recorded::<App>(&mut self.runtime, budget);
        if self.runtime.report().generation() != generation_before {
            self.runtime.note_surface_focus_validation();
        }
        // Wake acknowledgement precedes the first fallible snapshot. Even
        // when the FIFO is empty, a failed observation must remain retriable
        // through the normal host wake path rather than silently disarming it.
        if batch.is_err() {
            self.runtime.request_input_observation_retry_wake();
        }
        self.runtime.rearm_wake_if_needed();
        batch
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

    /// Replaces the send-task executor used for later start envelopes.
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

    /// Returns framework-service requests whose committed start envelopes ran.
    #[must_use]
    pub fn pending_framework_services(&self) -> Vec<crate::FrameworkServiceRef<'_>> {
        self.runtime.pending_framework_services()
    }

    /// Queues one typed framework-service response for queue-front validation.
    ///
    /// # Errors
    ///
    /// Returns the exact response when the token is foreign or stale, the
    /// response kind mismatches, the queue is full, or the runtime is closed.
    pub fn complete_framework_service(
        &mut self,
        token: &crate::FrameworkServiceToken,
        response: runenui_core::FrameworkServiceResponse,
    ) -> Result<WorkSequence, crate::FrameworkServiceResponseError> {
        self.runtime.complete_framework_service(token, response)
    }

    /// Creates a send-capable service completion for cross-thread delivery.
    ///
    /// # Errors
    ///
    /// Returns the response when the token is foreign, stale, or has a
    /// mismatched response kind, or when the runtime is closed or terminal.
    pub fn framework_service_response_completion(
        &mut self,
        token: &crate::FrameworkServiceToken,
        response: runenui_core::FrameworkServiceResponse,
    ) -> Result<crate::FrameworkServiceResponseCompletion, crate::FrameworkServiceResponseError>
    {
        self.runtime
            .framework_service_response_completion(token, response)
    }

    /// Queues cancellation for one exact live framework-service request.
    ///
    /// # Errors
    ///
    /// Returns an error when the token is foreign or stale, the runtime is
    /// closed or terminal, the queue is full, or sequence allocation is
    /// exhausted.
    pub fn cancel_framework_service(
        &mut self,
        token: &crate::FrameworkServiceToken,
    ) -> Result<WorkSequence, crate::FrameworkServiceCancelError> {
        self.runtime.cancel_framework_service(token)
    }

    #[must_use]
    pub fn subscription_diagnostics(&self) -> &[crate::SubscriptionDiagnostic] {
        self.runtime.subscription_diagnostics()
    }

    /// Takes the configured bounded trace sink receiver exactly once.
    ///
    /// Returns `None` when sink delivery was not configured or the receiver was
    /// already taken. Receiving is always nonblocking through
    /// [`crate::TraceSinkReceiver::try_recv`].
    #[must_use]
    pub fn take_trace_sink_receiver(&mut self) -> Option<crate::TraceSinkReceiver> {
        self.runtime.trace.take_sink_receiver()
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

    /// Closes the runtime with the final revisioned ownership and retirement.
    ///
    /// # Errors
    /// No closure occurs if mandatory capacity or final revision headroom
    /// cannot be reserved before the irreversible shutdown transition.
    pub fn shutdown(&mut self) -> Result<InputShutdownBatch, InputObservationError> {
        self.runtime.shutdown_observed()
    }
}
