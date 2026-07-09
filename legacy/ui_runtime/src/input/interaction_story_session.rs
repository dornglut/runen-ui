//! Executable interaction story session.
//!
//! A session can be driven by deterministic replay scripts or live-shaped
//! normalized input samples. Both modes rebuild evidence through the existing
//! descriptor-backed `replay_interactions` path after `NormalizedInputSample` so
//! hosts cannot fake hover, press, focus, activation, or suppression state.
//! The session does not execute host commands, mutate product state, create
//! overlays, or perform text editing.

use ui_input::NormalizedInputSample;

use crate::{
    InteractionBoundaryAssertions, InteractionFormationReport, InteractionProofRenderFrame,
    InteractionReplayScript, InteractionReplayStep, InteractionVisibleState,
    InteractionVisualProof, MountedInteractionFixture, RuntimeInteractionOutcome,
    RuntimeNoTargetInteraction, RuntimeSuppressedInteraction, WidgetId,
    interaction_visual_proof_to_frame, replay_interactions,
};

/// Execution mode for an executable interaction story session.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum InteractionStoryExecutionMode {
    /// Scripted deterministic replay mode.
    Replay,
    /// Live proof-host mode fed by host-normalized input samples.
    Live,
}

impl InteractionStoryExecutionMode {
    /// Stable label used by reports and diagnostics.
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Replay => "replay",
            Self::Live => "live",
        }
    }
}

/// Runtime-owned executable interaction story session.
#[derive(Debug, Clone, PartialEq)]
pub struct InteractionStorySession {
    story_id: String,
    mode: InteractionStoryExecutionMode,
    fixture: MountedInteractionFixture,
    selected_widget: WidgetId,
    script: InteractionReplayScript,
    input_log: Vec<NormalizedInputSample>,
    report: InteractionFormationReport,
}

impl InteractionStorySession {
    /// Starts an empty executable interaction story session.
    pub fn new(
        story_id: impl Into<String>,
        fixture: MountedInteractionFixture,
        mode: InteractionStoryExecutionMode,
        selected_widget: WidgetId,
    ) -> Self {
        let story_id = story_id.into();
        let script = InteractionReplayScript::new(story_id.clone());
        let report = replay_interactions(&fixture, &script);
        Self {
            story_id,
            mode,
            fixture,
            selected_widget,
            script,
            input_log: Vec::new(),
            report,
        }
    }

    /// Starts a replay-mode executable interaction story session.
    pub fn replay(
        story_id: impl Into<String>,
        fixture: MountedInteractionFixture,
        selected_widget: WidgetId,
    ) -> Self {
        Self::new(
            story_id,
            fixture,
            InteractionStoryExecutionMode::Replay,
            selected_widget,
        )
    }

    /// Starts a live-mode executable interaction story session.
    pub fn live(
        story_id: impl Into<String>,
        fixture: MountedInteractionFixture,
        selected_widget: WidgetId,
    ) -> Self {
        Self::new(
            story_id,
            fixture,
            InteractionStoryExecutionMode::Live,
            selected_widget,
        )
    }

    /// Stable story id for this session.
    pub fn story_id(&self) -> &str {
        &self.story_id
    }

    /// Execution mode for this session.
    pub const fn mode(&self) -> InteractionStoryExecutionMode {
        self.mode
    }

    /// Mounted fixture used by this session.
    pub fn fixture(&self) -> &MountedInteractionFixture {
        &self.fixture
    }

    /// Recorded live/replay input samples.
    pub fn input_log(&self) -> &[NormalizedInputSample] {
        &self.input_log
    }

    /// Current replay script built from applied steps.
    pub fn script(&self) -> &InteractionReplayScript {
        &self.script
    }

    /// Current descriptor-backed formation report.
    pub fn report(&self) -> &InteractionFormationReport {
        &self.report
    }

    /// Applies one normalized input sample as a live/replay step.
    pub fn apply_sample(&mut self, sample: NormalizedInputSample) -> InteractionStoryStepEvidence {
        let step_id = sample.sample_id.clone();
        self.apply_step(InteractionReplayStep::new(step_id, sample))
    }

    /// Applies one replay step, preserving the step id for reports.
    pub fn apply_step(&mut self, step: InteractionReplayStep) -> InteractionStoryStepEvidence {
        let step_id = step.step_id.clone();
        let sample_id = step.sample.sample_id.clone();
        self.input_log.push(step.sample.clone());
        self.script.steps.push(step);
        self.rebuild_report();

        InteractionStoryStepEvidence {
            step_id,
            sample_id,
            applied_step_count: self.script.steps.len(),
            boundary_assertions: self.report.boundary_assertions,
        }
    }

    /// Runs a replay script through the same apply path used by live samples.
    pub fn run_script(&mut self, script: &InteractionReplayScript) -> InteractionStoryRunReport {
        for step in &script.steps {
            self.apply_step(step.clone());
        }
        self.run_report()
    }

    /// Runs a replay script and validates expected evidence.
    pub fn run_script_with_expected(
        &mut self,
        script: &InteractionReplayScript,
        expected: &InteractionStoryExpectedEvidence,
    ) -> InteractionStoryRunReport {
        for step in &script.steps {
            self.apply_step(step.clone());
        }
        self.run_report_with_expected(expected)
    }

    /// Current visible proof model.
    pub fn current_proof(&self) -> InteractionVisualProof {
        InteractionVisualProof::from_fixture_report(
            self.story_id.clone(),
            &self.fixture,
            &self.report,
            self.selected_widget,
        )
    }

    /// Current renderer-neutral proof frame.
    pub fn current_frame(&self) -> InteractionProofRenderFrame {
        interaction_visual_proof_to_frame(&self.current_proof())
    }

    /// Current run report without extra expected-evidence requirements.
    pub fn run_report(&self) -> InteractionStoryRunReport {
        self.run_report_with_expected(&InteractionStoryExpectedEvidence::default())
    }

    /// Current run report with expected-evidence validation.
    pub fn run_report_with_expected(
        &self,
        expected: &InteractionStoryExpectedEvidence,
    ) -> InteractionStoryRunReport {
        let visual_proof = self.current_proof();
        let render_summary = interaction_visual_proof_to_frame(&visual_proof).summary;
        let evidence_result = expected.validate(&visual_proof, &self.report);
        InteractionStoryRunReport {
            story_id: self.story_id.clone(),
            mode: self.mode,
            input_log: self.input_log.clone(),
            replay_script: self.script.clone(),
            formation_report: self.report.clone(),
            visual_proof,
            render_summary,
            boundary_assertions: self.report.boundary_assertions,
            evidence_result,
        }
    }

    /// Replays this session's recorded input log deterministically.
    pub fn replay_recorded_input_log(&self) -> InteractionStoryRunReport {
        let mut replay = Self::new(
            self.story_id.clone(),
            self.fixture.clone(),
            InteractionStoryExecutionMode::Replay,
            self.selected_widget,
        );
        for step in &self.script.steps {
            replay.apply_step(step.clone());
        }
        replay.run_report()
    }

    /// Compares this session with a deterministic replay of its recorded log.
    pub fn replay_live_parity_report(&self) -> InteractionReplayLiveParityReport {
        InteractionReplayLiveParityReport::from_reports(
            self.story_id.clone(),
            self.run_report(),
            self.replay_recorded_input_log(),
        )
    }

    fn rebuild_report(&mut self) {
        self.report = replay_interactions(&self.fixture, &self.script);
    }
}

/// Evidence returned after one incremental session step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InteractionStoryStepEvidence {
    /// Applied replay/live step id.
    pub step_id: String,
    /// Source normalized input sample id.
    pub sample_id: String,
    /// Number of applied steps after this update.
    pub applied_step_count: usize,
    /// Current no-bypass boundary counters.
    pub boundary_assertions: InteractionBoundaryAssertions,
}

/// Complete executable interaction story run report.
#[derive(Debug, Clone, PartialEq)]
pub struct InteractionStoryRunReport {
    /// Stable story id.
    pub story_id: String,
    /// Execution mode that produced this report.
    pub mode: InteractionStoryExecutionMode,
    /// Recorded normalized input samples.
    pub input_log: Vec<NormalizedInputSample>,
    /// Replay script represented by the applied steps.
    pub replay_script: InteractionReplayScript,
    /// Descriptor-backed interaction formation report.
    pub formation_report: InteractionFormationReport,
    /// Semantic visible proof model.
    pub visual_proof: InteractionVisualProof,
    /// Renderer-neutral frame summary derived from the proof.
    pub render_summary: crate::InteractionProofRenderSummary,
    /// No-bypass boundary counters.
    pub boundary_assertions: InteractionBoundaryAssertions,
    /// Expected-evidence validation result.
    pub evidence_result: InteractionStoryEvidenceResult,
}

/// Semantic replay/live parity report.
#[derive(Debug, Clone, PartialEq)]
pub struct InteractionReplayLiveParityReport {
    /// Stable story id.
    pub story_id: String,
    /// Original live run report.
    pub live_report: InteractionStoryRunReport,
    /// Deterministic replay of the live input log.
    pub replayed_live_log_report: InteractionStoryRunReport,
    /// Target-resolution equivalence.
    pub equivalent_target_resolution: bool,
    /// Focus-resolution equivalence.
    pub equivalent_focus_resolution: bool,
    /// State-transition equivalence.
    pub equivalent_state_transitions: bool,
    /// Runtime-fact equivalence.
    pub equivalent_runtime_facts: bool,
    /// Runtime-event equivalence.
    pub equivalent_runtime_events: bool,
    /// Semantic-outcome equivalence.
    pub equivalent_semantic_outcomes: bool,
    /// Suppression equivalence.
    pub equivalent_suppression: bool,
    /// No-target equivalence.
    pub equivalent_no_target: bool,
    /// Observed visible marker equivalence.
    pub equivalent_observed_markers: bool,
    /// Final current-state equivalence.
    pub equivalent_final_current_states: bool,
    /// No-bypass boundary equivalence.
    pub equivalent_boundaries: bool,
}

impl InteractionReplayLiveParityReport {
    /// Builds a semantic parity report from live and replayed reports.
    pub fn from_reports(
        story_id: impl Into<String>,
        live_report: InteractionStoryRunReport,
        replayed_live_log_report: InteractionStoryRunReport,
    ) -> Self {
        let equivalent_observed_markers = proof_markers(&live_report.visual_proof)
            == proof_markers(&replayed_live_log_report.visual_proof);
        let equivalent_final_current_states = proof_current_states(&live_report.visual_proof)
            == proof_current_states(&replayed_live_log_report.visual_proof);

        Self {
            story_id: story_id.into(),
            equivalent_target_resolution: live_report.formation_report.target_resolution
                == replayed_live_log_report.formation_report.target_resolution,
            equivalent_focus_resolution: live_report.formation_report.focus_resolution
                == replayed_live_log_report.formation_report.focus_resolution,
            equivalent_state_transitions: live_report.formation_report.state_transitions
                == replayed_live_log_report.formation_report.state_transitions,
            equivalent_runtime_facts: live_report.formation_report.runtime_facts
                == replayed_live_log_report.formation_report.runtime_facts,
            equivalent_runtime_events: live_report.formation_report.runtime_events
                == replayed_live_log_report.formation_report.runtime_events,
            equivalent_semantic_outcomes: live_report.formation_report.semantic_outcomes
                == replayed_live_log_report.formation_report.semantic_outcomes,
            equivalent_suppression: live_report.formation_report.suppressed_events
                == replayed_live_log_report.formation_report.suppressed_events,
            equivalent_no_target: live_report.formation_report.no_target_events
                == replayed_live_log_report.formation_report.no_target_events,
            equivalent_observed_markers,
            equivalent_final_current_states,
            equivalent_boundaries: live_report.boundary_assertions
                == replayed_live_log_report.boundary_assertions,
            live_report,
            replayed_live_log_report,
        }
    }

    /// Returns true when all semantic parity checks pass.
    pub const fn passed(&self) -> bool {
        self.equivalent_target_resolution
            && self.equivalent_focus_resolution
            && self.equivalent_state_transitions
            && self.equivalent_runtime_facts
            && self.equivalent_runtime_events
            && self.equivalent_semantic_outcomes
            && self.equivalent_suppression
            && self.equivalent_no_target
            && self.equivalent_observed_markers
            && self.equivalent_final_current_states
            && self.equivalent_boundaries
    }
}

/// Expected evidence for an executable interaction story.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct InteractionStoryExpectedEvidence {
    /// Required observed markers by widget.
    pub required_markers: Vec<ExpectedInteractionMarker>,
    /// Required final current states by widget.
    pub required_current_states: Vec<ExpectedCurrentState>,
    /// Required semantic outcomes.
    pub required_outcomes: Vec<ExpectedInteractionOutcome>,
    /// Required suppression rows.
    pub required_suppressions: Vec<ExpectedSuppression>,
    /// Required no-target reasons.
    pub required_no_target_reasons: Vec<String>,
    /// Whether the story must prove zero host/product/overlay/text-edit bypass.
    pub require_no_bypass: bool,
}

impl InteractionStoryExpectedEvidence {
    /// Validates a proof/report against this expected-evidence contract.
    pub fn validate(
        &self,
        proof: &InteractionVisualProof,
        report: &InteractionFormationReport,
    ) -> InteractionStoryEvidenceResult {
        let mut missing = Vec::new();

        for expected in &self.required_markers {
            let has_marker = proof
                .main_view
                .control(expected.widget_id)
                .is_some_and(|control| control.has_marker(expected.state));
            if !has_marker {
                missing.push(format!(
                    "marker:{:?}:{}",
                    expected.widget_id,
                    expected.state.as_str()
                ));
            }
        }

        for expected in &self.required_current_states {
            let has_state = proof
                .main_view
                .control(expected.widget_id)
                .is_some_and(|control| control.has_current_state(expected.state));
            if !has_state {
                missing.push(format!(
                    "current:{:?}:{}",
                    expected.widget_id,
                    expected.state.as_str()
                ));
            }
        }

        for expected in &self.required_outcomes {
            if !has_outcome(&report.semantic_outcomes, expected) {
                missing.push(format!(
                    "outcome:{:?}:{}",
                    expected.widget_id, expected.outcome
                ));
            }
        }

        for expected in &self.required_suppressions {
            if !has_suppression(&report.suppressed_events, expected) {
                missing.push(format!(
                    "suppression:{:?}:{}",
                    expected.widget_id, expected.reason
                ));
            }
        }

        for reason in &self.required_no_target_reasons {
            if !has_no_target(&report.no_target_events, reason) {
                missing.push(format!("no-target:{reason}"));
            }
        }

        if self.require_no_bypass && !report.boundary_assertions.no_bypass_evidence() {
            missing.push("boundary:no-bypass".to_owned());
        }

        InteractionStoryEvidenceResult { missing }
    }
}

/// Required observed marker for one widget.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExpectedInteractionMarker {
    /// Widget that must expose the marker.
    pub widget_id: WidgetId,
    /// Required observed marker state.
    pub state: InteractionVisibleState,
}

impl ExpectedInteractionMarker {
    /// Creates a required observed marker assertion.
    pub const fn new(widget_id: WidgetId, state: InteractionVisibleState) -> Self {
        Self { widget_id, state }
    }
}

/// Required final current state for one widget.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExpectedCurrentState {
    /// Widget that must retain the state.
    pub widget_id: WidgetId,
    /// Required current state.
    pub state: InteractionVisibleState,
}

impl ExpectedCurrentState {
    /// Creates a required current-state assertion.
    pub const fn new(widget_id: WidgetId, state: InteractionVisibleState) -> Self {
        Self { widget_id, state }
    }
}

/// Required semantic outcome for one widget.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpectedInteractionOutcome {
    /// Widget that must emit the outcome.
    pub widget_id: WidgetId,
    /// Stable outcome label.
    pub outcome: String,
}

impl ExpectedInteractionOutcome {
    /// Creates a required semantic-outcome assertion.
    pub fn new(widget_id: WidgetId, outcome: impl Into<String>) -> Self {
        Self {
            widget_id,
            outcome: outcome.into(),
        }
    }
}

/// Required suppression evidence for one widget.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExpectedSuppression {
    /// Widget that must emit suppression evidence.
    pub widget_id: WidgetId,
    /// Stable suppression reason.
    pub reason: String,
}

impl ExpectedSuppression {
    /// Creates a required suppression assertion.
    pub fn new(widget_id: WidgetId, reason: impl Into<String>) -> Self {
        Self {
            widget_id,
            reason: reason.into(),
        }
    }
}

/// Expected-evidence validation result.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct InteractionStoryEvidenceResult {
    /// Missing evidence rows. Empty means the evidence contract passed.
    pub missing: Vec<String>,
}

impl InteractionStoryEvidenceResult {
    /// Returns true when no expected evidence is missing.
    pub fn passed(&self) -> bool {
        self.missing.is_empty()
    }
}

/// Expected evidence for the canonical base-controls executable interaction story.
pub fn base_controls_executable_interaction_expected_evidence() -> InteractionStoryExpectedEvidence
{
    InteractionStoryExpectedEvidence {
        required_markers: vec![
            ExpectedInteractionMarker::new(WidgetId(1), InteractionVisibleState::Hovered),
            ExpectedInteractionMarker::new(WidgetId(1), InteractionVisibleState::Pressed),
            ExpectedInteractionMarker::new(WidgetId(1), InteractionVisibleState::Captured),
            ExpectedInteractionMarker::new(WidgetId(1), InteractionVisibleState::Focused),
            ExpectedInteractionMarker::new(WidgetId(1), InteractionVisibleState::FocusVisible),
            ExpectedInteractionMarker::new(
                WidgetId(1),
                InteractionVisibleState::ActivationRequested,
            ),
            ExpectedInteractionMarker::new(WidgetId(7), InteractionVisibleState::Disabled),
            ExpectedInteractionMarker::new(WidgetId(7), InteractionVisibleState::Suppressed),
            ExpectedInteractionMarker::new(
                WidgetId(4),
                InteractionVisibleState::ListActiveItemIntent,
            ),
            ExpectedInteractionMarker::new(WidgetId(5), InteractionVisibleState::TreeNodeIntent),
            ExpectedInteractionMarker::new(
                WidgetId(6),
                InteractionVisibleState::TableCellOrRowIntent,
            ),
            ExpectedInteractionMarker::new(WidgetId(3), InteractionVisibleState::TextIntentProbe),
            ExpectedInteractionMarker::new(
                WidgetId(8),
                InteractionVisibleState::ReadOnlyTextIntentProbe,
            ),
        ],
        required_current_states: vec![
            ExpectedCurrentState::new(WidgetId(7), InteractionVisibleState::Disabled),
            ExpectedCurrentState::new(WidgetId(8), InteractionVisibleState::ReadOnly),
            ExpectedCurrentState::new(WidgetId(8), InteractionVisibleState::Focused),
        ],
        required_outcomes: vec![
            ExpectedInteractionOutcome::new(WidgetId(1), "activation-requested"),
            ExpectedInteractionOutcome::new(WidgetId(2), "action-intent"),
            ExpectedInteractionOutcome::new(WidgetId(4), "active-item-intent"),
            ExpectedInteractionOutcome::new(WidgetId(5), "node-intent"),
            ExpectedInteractionOutcome::new(WidgetId(6), "cell-or-row-intent"),
            ExpectedInteractionOutcome::new(WidgetId(3), "text-intent-seen"),
            ExpectedInteractionOutcome::new(WidgetId(8), "text-intent-seen"),
        ],
        required_suppressions: vec![ExpectedSuppression::new(WidgetId(7), "control.disabled")],
        required_no_target_reasons: vec!["pointer.no_target".to_owned()],
        require_no_bypass: true,
    }
}

fn has_outcome(
    outcomes: &[RuntimeInteractionOutcome],
    expected: &ExpectedInteractionOutcome,
) -> bool {
    outcomes
        .iter()
        .any(|outcome| outcome.target == expected.widget_id && outcome.outcome == expected.outcome)
}

fn has_suppression(
    suppressed: &[RuntimeSuppressedInteraction],
    expected: &ExpectedSuppression,
) -> bool {
    suppressed
        .iter()
        .any(|event| event.target == expected.widget_id && event.reason == expected.reason)
}

fn has_no_target(no_targets: &[RuntimeNoTargetInteraction], reason: &str) -> bool {
    no_targets.iter().any(|event| event.reason == reason)
}

fn proof_markers(proof: &InteractionVisualProof) -> Vec<(WidgetId, Vec<InteractionVisibleState>)> {
    proof
        .main_view
        .controls
        .iter()
        .map(|control| {
            (
                control.widget_id,
                control
                    .observed_markers
                    .iter()
                    .map(|marker| marker.state)
                    .collect::<Vec<_>>(),
            )
        })
        .collect()
}

fn proof_current_states(
    proof: &InteractionVisualProof,
) -> Vec<(WidgetId, Vec<InteractionVisibleState>)> {
    proof
        .main_view
        .controls
        .iter()
        .map(|control| (control.widget_id, control.current_states.clone()))
        .collect()
}
