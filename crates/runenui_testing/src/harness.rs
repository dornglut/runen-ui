use core::{error::Error, fmt};
use std::time::Duration;

use runenui_core::{CommandOrigin, ElementId, SemanticAction, SemanticCommand, SurfaceId, UiApp};
use runenui_runtime::{
    AppRuntime, AutomationSubmission, CommandSubmission, CommittedTextEvent, CompositionGeneration,
    CompositionRange, CompositionStartSubmission, CompositionSubmission, FocusState,
    FontFamilyName, FontRegistrationError, GenericFamilyMappingError, GenericFontFamily,
    HostRequestRef, InputDeviceId, KeyboardEvent, KeyboardSubmission, LogicalPoint, ManualClock,
    MonotonicInstant, MonotonicTimeError, MountedNodeId, PointerDeviceKind, PointerEvent, PointerId,
    PointerPhase, PointerSubmission, PublishSurfaceError, PumpBudget, PumpReport,
    ReconciliationReport, RuntimeConfig, SemanticPublication, SemanticRevision, SemanticSnapshot,
    SemanticUpdateResult, SubmitAutomationError, SubmitCommandError, SubmitCompositionError,
    SubmitCompositionStartError, SubmitKeyboardError, SubmitPointerError, SubmitSemanticActionError,
    SubmitSurfaceCommandError, SubmitTextError, SurfaceBuildContext, SurfaceInputContext,
    SurfacePublication, TextSubmission, TimerFiringOutcome, TimerStartOutcome, Trace, TraceReplay,
    TraceReplayError,
};

use crate::{
    SemanticQuery, SemanticQueryMatches, SemanticTarget, SettleBudget, SettleReport,
    TestSurfaceConfig, UniqueSemanticQueryError, query_semantics, settle::outcome_for,
};

/// Operation requires a committed harness publication that does not exist yet.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MissingPublication;

impl fmt::Display for MissingPublication {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("test harness has no committed surface publication")
    }
}

impl Error for MissingPublication {}

/// Failure while requiring one unique semantic target from the latest snapshot.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HarnessSemanticQueryError {
    MissingPublication,
    Query(UniqueSemanticQueryError),
}

impl fmt::Display for HarnessSemanticQueryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingPublication => {
                formatter.write_str("test harness has no committed surface publication")
            }
            Self::Query(error) => write!(formatter, "{error}"),
        }
    }
}

impl Error for HarnessSemanticQueryError {}

/// Failure while submitting a point-resolved command from the latest publication.
#[derive(Debug)]
pub enum HarnessSurfaceCommandError {
    MissingPublication,
    Submission(Box<SubmitSurfaceCommandError>),
}

impl HarnessSurfaceCommandError {
    /// Recovers the ordinary surface-command error when submission reached runtime ingress.
    #[must_use]
    pub fn into_submission_error(self) -> Option<SubmitSurfaceCommandError> {
        match self {
            Self::MissingPublication => None,
            Self::Submission(error) => Some(*error),
        }
    }
}

impl fmt::Display for HarnessSurfaceCommandError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingPublication => {
                formatter.write_str("test harness has no committed surface publication")
            }
            Self::Submission(error) => write!(formatter, "{error}"),
        }
    }
}

impl Error for HarnessSurfaceCommandError {}

/// Public-only deterministic headless harness over one ordinary `AppRuntime`.
///
/// The harness does not maintain a parallel expected state. It delegates all
/// mutations to public runtime APIs and retains only harness configuration plus
/// the latest complete immutable surface publication for inspection and helpers.
pub struct TestHarness<App: UiApp> {
    runtime: AppRuntime<App>,
    clock: ManualClock,
    surface: TestSurfaceConfig,
    publication: Option<SurfacePublication>,
}

impl<App: UiApp> TestHarness<App> {
    /// Mounts an application using deterministic runtime and surface defaults.
    #[must_use]
    pub fn mount(state: App::State) -> Self {
        Self::mount_with_config(
            state,
            RuntimeConfig::default(),
            TestSurfaceConfig::default(),
        )
    }

    /// Mounts with explicit runtime limits and deterministic fixed-surface configuration.
    #[must_use]
    pub fn mount_with_config(
        state: App::State,
        runtime_config: RuntimeConfig,
        surface: TestSurfaceConfig,
    ) -> Self {
        let clock = ManualClock::new();
        let mut runtime = AppRuntime::<App>::mount_with_config(state, runtime_config);
        runtime.set_monotonic_clock(clock.clone());
        Self {
            runtime,
            clock,
            surface,
            publication: None,
        }
    }

    /// Returns application state owned by the ordinary runtime.
    #[must_use]
    pub const fn state(&self) -> &App::State {
        self.runtime.state()
    }

    /// Returns the runtime's read-only focus authority.
    #[must_use]
    pub const fn focus(&self) -> &FocusState {
        self.runtime.focus()
    }

    /// Returns the latest reconciliation report from the ordinary runtime.
    #[must_use]
    pub const fn reconciliation_report(&self) -> &ReconciliationReport {
        self.runtime.reconciliation_report()
    }

    /// Returns the canonical bounded trace.
    #[must_use]
    pub const fn trace(&self) -> &Trace {
        self.runtime.trace()
    }

    /// Returns deterministic fixed-surface configuration.
    #[must_use]
    pub const fn surface_config(&self) -> &TestSurfaceConfig {
        &self.surface
    }

    /// Replaces configuration used by later explicit publication.
    pub fn set_surface_config(&mut self, surface: TestSurfaceConfig) {
        self.surface = surface;
    }

    /// Registers immutable bundled font bytes through the ordinary runtime text authority.
    ///
    /// # Errors
    ///
    /// Returns the runtime text-source registration error unchanged.
    pub fn register_text_font_bytes(
        &mut self,
        bytes: Vec<u8>,
    ) -> Result<usize, FontRegistrationError> {
        self.runtime.register_text_font_bytes(bytes)
    }

    /// Replaces one deterministic generic-family mapping through the ordinary runtime.
    ///
    /// # Errors
    ///
    /// Returns the runtime text-source mapping error unchanged.
    pub fn set_text_generic_family_mapping(
        &mut self,
        generic: GenericFontFamily,
        families: &[FontFamilyName],
    ) -> Result<bool, GenericFamilyMappingError> {
        self.runtime
            .set_text_generic_family_mapping(generic, families)
    }

    /// Processes one explicitly bounded runtime checkpoint.
    pub fn pump(&mut self, budget: PumpBudget) -> PumpReport {
        self.runtime.pump(budget)
    }

    /// Pumps until a complete zero-progress quiescent iteration, terminal/closed
    /// state, or the explicit finite iteration limit is reached.
    ///
    /// Publication dirtiness and dormant future timers do not by themselves keep
    /// this loop alive because they are not execution progress. A zero-sized pump
    /// budget with ready work remains bounded by `max_iterations`.
    pub fn run_until_idle(&mut self, budget: SettleBudget) -> SettleReport {
        let mut iteration = 0_usize;
        loop {
            iteration += 1;
            let report = self.runtime.pump(budget.pump_budget());
            let at_limit = iteration >= budget.max_iterations().get();
            if let Some(outcome) = outcome_for(report, at_limit) {
                return SettleReport::new(iteration, report, outcome);
            }
        }
    }

    /// Advances the clone-shared deterministic manual clock without sleeping.
    ///
    /// # Errors
    ///
    /// Returns [`MonotonicTimeError::Overflow`] when logical time cannot advance.
    pub fn advance_time(&self, duration: Duration) -> Result<MonotonicInstant, MonotonicTimeError> {
        self.clock.advance(duration)
    }

    /// Returns the last accepted timer start outcome from ordinary scheduler authority.
    #[must_use]
    pub const fn last_timer_start_outcome(&self) -> Option<TimerStartOutcome> {
        self.runtime.last_timer_start_outcome()
    }

    /// Returns the last accepted timer firing outcome from ordinary scheduler authority.
    #[must_use]
    pub const fn last_timer_firing_outcome(&self) -> Option<TimerFiringOutcome> {
        self.runtime.last_timer_firing_outcome()
    }

    /// Returns currently exposed application host requests without changing runtime state.
    #[must_use]
    pub fn pending_host_requests(&self) -> Vec<HostRequestRef<'_, App::HostProtocol>> {
        self.runtime.pending_host_requests()
    }

    /// Publishes the configured fixed surface through the runtime-owned logical text authority.
    ///
    /// Deterministic bundled-only runtimes require callers to register controlled font bytes
    /// explicitly before publishing text-bearing surfaces.
    ///
    /// # Errors
    ///
    /// Returns the ordinary runtime's publication refusal or terminal error.
    pub fn publish(&mut self) -> Result<&SurfacePublication, PublishSurfaceError> {
        let context = SurfaceBuildContext::tight(self.surface.style_environment(), self.surface.size());
        let publication = self.runtime.publish_surface(&context)?;
        Ok(self.publication.insert(publication))
    }

    /// Publishes with a completely explicit ordinary public build context.
    ///
    /// # Errors
    ///
    /// Returns the ordinary runtime's publication refusal or terminal error.
    pub fn publish_with_context(
        &mut self,
        context: &SurfaceBuildContext<'_>,
    ) -> Result<&SurfacePublication, PublishSurfaceError> {
        let publication = self.runtime.publish_surface(context)?;
        Ok(self.publication.insert(publication))
    }

    /// Returns the latest complete immutable publication retained by the harness.
    #[must_use]
    pub const fn publication(&self) -> Option<&SurfacePublication> {
        self.publication.as_ref()
    }

    /// Returns the latest exact displayed input context.
    ///
    /// # Errors
    ///
    /// Returns [`MissingPublication`] until an explicit publication succeeds.
    pub fn input_context(&self) -> Result<&SurfaceInputContext, MissingPublication> {
        self.publication
            .as_ref()
            .map(SurfacePublication::input_context)
            .ok_or(MissingPublication)
    }

    /// Returns the latest committed semantic publication.
    ///
    /// # Errors
    ///
    /// Returns [`MissingPublication`] until an explicit publication succeeds.
    pub fn semantic_publication(&self) -> Result<&SemanticPublication, MissingPublication> {
        self.publication
            .as_ref()
            .map(SurfacePublication::semantic_publication)
            .ok_or(MissingPublication)
    }

    /// Returns the latest committed semantic snapshot.
    ///
    /// # Errors
    ///
    /// Returns [`MissingPublication`] until an explicit publication succeeds.
    pub fn semantic_snapshot(&self) -> Result<&SemanticSnapshot, MissingPublication> {
        self.semantic_publication()
            .map(SemanticPublication::snapshot)
    }

    /// Evaluates a deterministic semantic query against the latest exact snapshot.
    ///
    /// # Errors
    ///
    /// Returns [`MissingPublication`] until an explicit publication succeeds.
    pub fn query_semantics(
        &self,
        query: &SemanticQuery,
    ) -> Result<SemanticQueryMatches, MissingPublication> {
        self.semantic_snapshot()
            .map(|snapshot| query_semantics(snapshot, query))
    }

    /// Requires one unique match in the latest exact semantic snapshot.
    ///
    /// # Errors
    ///
    /// Distinguishes a missing publication from a missing or ambiguous query.
    pub fn unique_semantic_target(
        &self,
        query: &SemanticQuery,
    ) -> Result<SemanticTarget, HarnessSemanticQueryError> {
        let matches = self
            .query_semantics(query)
            .map_err(|_| HarnessSemanticQueryError::MissingPublication)?;
        matches.unique().map_err(HarnessSemanticQueryError::Query)
    }

    /// Requests semantic update selection from an exact declared surface/revision base.
    ///
    /// # Errors
    ///
    /// Returns [`MissingPublication`] until an explicit publication succeeds.
    pub fn semantic_update_from(
        &self,
        surface: &SurfaceId,
        revision: SemanticRevision,
    ) -> Result<SemanticUpdateResult<'_>, MissingPublication> {
        self.semantic_publication()
            .map(|publication| publication.update_from(surface, revision))
    }

    /// Creates a pointer event using the latest exact public surface context.
    ///
    /// # Errors
    ///
    /// Returns [`MissingPublication`] until an explicit publication succeeds.
    pub fn pointer_event(
        &self,
        pointer_id: PointerId,
        device_kind: PointerDeviceKind,
        phase: PointerPhase,
        position: LogicalPoint,
    ) -> Result<PointerEvent, MissingPublication> {
        Ok(PointerEvent::new(
            pointer_id,
            device_kind,
            phase,
            position,
            self.input_context()?.clone(),
        ))
    }

    /// Delegates one pointer event to ordinary public ingress.
    ///
    /// # Errors
    ///
    /// Returns the ordinary runtime pointer-ingress error.
    pub fn submit_pointer(
        &mut self,
        event: PointerEvent,
    ) -> Result<PointerSubmission, SubmitPointerError> {
        self.runtime.submit_pointer(event)
    }

    /// Delegates one exact mounted command to ordinary public command ingress.
    ///
    /// This is intentionally separate from semantic-query helpers: callers must
    /// already possess a legitimate public mounted identity from a renderer/frame
    /// product when testing mounted command behavior.
    ///
    /// # Errors
    ///
    /// Returns the ordinary command-ingress error with exact owned recovery.
    pub fn submit_command(
        &mut self,
        target: MountedNodeId,
        command: SemanticCommand,
        origin: CommandOrigin,
    ) -> Result<CommandSubmission, SubmitCommandError> {
        self.runtime.submit_command(target, command, origin)
    }

    /// Delegates a point-resolved semantic command through the latest exact
    /// displayed public context.
    ///
    /// # Errors
    ///
    /// Distinguishes a missing publication from ordinary surface-command ingress rejection.
    pub fn submit_surface_command(
        &mut self,
        point: LogicalPoint,
        command: SemanticCommand,
        origin: CommandOrigin,
    ) -> Result<CommandSubmission, HarnessSurfaceCommandError> {
        let context = self
            .input_context()
            .map_err(|_| HarnessSurfaceCommandError::MissingPublication)?
            .clone();
        self.runtime
            .submit_surface_command(context, point, command, origin)
            .map_err(|error| HarnessSurfaceCommandError::Submission(Box::new(error)))
    }

    /// Delegates an exact snapshot-scoped semantic action to accepted M5C ingress.
    ///
    /// # Errors
    ///
    /// Returns the ordinary semantic-action admission error with exact request recovery.
    pub fn submit_semantic_action(
        &mut self,
        target: &SemanticTarget,
        action: SemanticAction,
    ) -> Result<CommandSubmission, SubmitSemanticActionError> {
        self.runtime.submit_semantic_action(target.request(action))
    }

    /// Delegates one application action to ordinary public ingress.
    ///
    /// # Errors
    ///
    /// Returns the ordinary application-action rejection with exact action recovery.
    pub fn submit_action(
        &mut self,
        action: App::Action,
    ) -> runenui_runtime::SubmitActionResult<App::Action> {
        self.runtime.submit_action(action)
    }

    /// Delegates one public keyboard event to ordinary ingress.
    ///
    /// # Errors
    ///
    /// Returns the ordinary keyboard-ingress error.
    pub fn submit_keyboard(
        &mut self,
        event: KeyboardEvent,
    ) -> Result<KeyboardSubmission, SubmitKeyboardError> {
        self.runtime.submit_keyboard(event)
    }

    /// Delegates one public committed-text event to ordinary ingress.
    ///
    /// # Errors
    ///
    /// Returns the ordinary text-ingress error.
    pub fn submit_text(
        &mut self,
        event: CommittedTextEvent,
    ) -> Result<TextSubmission, SubmitTextError> {
        self.runtime.submit_text(event)
    }

    /// Starts composition through ordinary public ingress.
    ///
    /// # Errors
    ///
    /// Returns the ordinary composition-start error.
    pub fn start_composition(
        &mut self,
        device_id: Option<InputDeviceId>,
    ) -> Result<CompositionStartSubmission, SubmitCompositionStartError> {
        self.runtime.start_composition(device_id)
    }

    /// Delegates one composition update through ordinary public ingress.
    ///
    /// # Errors
    ///
    /// Returns the ordinary composition-ingress error.
    pub fn submit_composition_update(
        &mut self,
        generation: CompositionGeneration,
        preedit: String,
        range: Option<CompositionRange>,
    ) -> Result<CompositionSubmission, SubmitCompositionError> {
        self.runtime
            .submit_composition_update(generation, preedit, range)
    }

    /// Ends one exact composition lifetime through ordinary public ingress.
    ///
    /// # Errors
    ///
    /// Returns the ordinary composition-ingress error.
    pub fn submit_composition_end(
        &mut self,
        generation: CompositionGeneration,
    ) -> Result<CompositionSubmission, SubmitCompositionError> {
        self.runtime.submit_composition_end(generation)
    }

    /// Cancels one exact composition lifetime through ordinary public ingress.
    ///
    /// # Errors
    ///
    /// Returns the ordinary composition-ingress error.
    pub fn cancel_composition(
        &mut self,
        generation: CompositionGeneration,
    ) -> Result<CompositionSubmission, SubmitCompositionError> {
        self.runtime.cancel_composition(generation)
    }

    /// Delegates deterministic authored-ID automation to ordinary public ingress.
    ///
    /// # Errors
    ///
    /// Returns the ordinary automation-resolution or admission error.
    pub fn submit_automation_command(
        &mut self,
        authored_id: ElementId,
        command: SemanticCommand,
    ) -> Result<AutomationSubmission, SubmitAutomationError> {
        self.runtime
            .submit_automation_command(authored_id, command)
    }

    /// Exports the canonical trace through its accepted deterministic JSONL projection.
    #[must_use]
    pub fn trace_jsonl(&self) -> String {
        self.runtime.trace().export_jsonl()
    }

    /// Parses the current deterministic trace export through accepted inert replay.
    ///
    /// # Errors
    ///
    /// Returns the ordinary trace replay structural validation error.
    pub fn trace_replay(&self) -> Result<TraceReplay, TraceReplayError> {
        TraceReplay::parse_jsonl(&self.trace_jsonl())
    }

    /// Consumes the harness and recovers application state through the public runtime.
    #[must_use]
    pub fn into_state(self) -> App::State {
        self.runtime.into_state()
    }
}
