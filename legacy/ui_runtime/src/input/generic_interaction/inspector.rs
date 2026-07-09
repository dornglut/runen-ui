//! Inspector, report, and proof-frame views for generic interaction replay.

use crate::WidgetId;

use super::boundary::InteractionBoundaryAssertions;
use super::fixture::MountedInteractionControl;
use super::formatting::{
    format_focus_resolution_row, format_no_target_event_row, format_runtime_event_row,
    format_runtime_fact_row, format_semantic_outcome_row, format_state_transition_row,
    format_suppressed_event_row, format_target_resolution_row,
};
use super::report::InteractionFormationReport;
use super::state_mapping::{folded_current_states, visible_state_from_transition};
use super::visual::{InteractionVisibleState, InteractionVisualProof};

/// Inspector pane data for the selected proof control.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InteractionInspectorView {
    /// Selected widget shown by the inspector proof view.
    pub selected_widget: Option<WidgetId>,

    /// Selected control kind id.
    pub control_kind_id: Option<String>,

    /// Declared trigger and outcome requirements for the selected control.
    pub declared_requirements: Vec<String>,

    /// States observed during replay for the selected control.
    pub observed_reusable_interaction_states: Vec<InteractionVisibleState>,

    /// Final state set after active/inactive transitions are folded.
    pub current_reusable_interaction_state_set: Vec<InteractionVisibleState>,

    /// Whether the descriptor declares text-intent probe support.
    pub text_intent_probe: bool,

    /// Whether the mounted selected control is read-only.
    pub read_only: bool,
}

impl InteractionInspectorView {
    pub(super) fn from_fixture_report(
        control: Option<&MountedInteractionControl>,
        report: &InteractionFormationReport,
    ) -> Self {
        let Some(control) = control else {
            return Self {
                selected_widget: None,
                control_kind_id: None,
                declared_requirements: Vec::new(),
                observed_reusable_interaction_states: Vec::new(),
                current_reusable_interaction_state_set: Vec::new(),
                text_intent_probe: false,
                read_only: false,
            };
        };
        let mut observed_state_set = Vec::new();
        for transition in report
            .state_transitions
            .iter()
            .filter(|transition| transition.target == control.widget_id)
        {
            if let Some(state) = visible_state_from_transition(&transition.state)
                && !observed_state_set.contains(&state)
            {
                observed_state_set.push(state);
            }
        }

        Self {
            selected_widget: Some(control.widget_id),
            control_kind_id: Some(control.control_kind_id.clone()),
            declared_requirements: control
                .descriptor
                .requirements
                .iter()
                .map(|requirement| {
                    let outcomes = requirement
                        .outcomes
                        .iter()
                        .map(|outcome| outcome.as_str())
                        .collect::<Vec<_>>()
                        .join("|");
                    format!("{} -> {}", requirement.trigger.as_str(), outcomes)
                })
                .collect(),
            observed_reusable_interaction_states: observed_state_set,
            current_reusable_interaction_state_set: folded_current_states(control, report),
            text_intent_probe: control.descriptor.text_intent_probe,
            read_only: control.read_only,
        }
    }
}

/// Report/event pane data for the Phase 12 visual proof.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InteractionReportView {
    /// Ordered replay step ids.
    pub replay_steps: Vec<String>,

    /// Target resolution rows formatted for renderer-neutral proof output.
    pub target_resolution: Vec<String>,

    /// Focus resolution rows formatted for renderer-neutral proof output.
    pub focus_resolution: Vec<String>,

    /// State transition rows formatted for renderer-neutral proof output.
    pub state_transitions: Vec<String>,

    /// Runtime fact rows formatted for renderer-neutral proof output.
    pub runtime_facts: Vec<String>,

    /// Runtime event rows formatted for renderer-neutral proof output.
    pub runtime_events: Vec<String>,

    /// Semantic outcome rows formatted for renderer-neutral proof output.
    pub semantic_outcomes: Vec<String>,

    /// Suppressed event rows formatted for renderer-neutral proof output.
    pub suppressed_events: Vec<String>,

    /// No-target rows formatted for renderer-neutral proof output.
    pub no_target_events: Vec<String>,

    /// Boundary counters proving no bypass occurred.
    pub boundary_assertions: InteractionBoundaryAssertions,
}

impl InteractionReportView {
    pub(super) fn from_report(report: &InteractionFormationReport) -> Self {
        Self {
            replay_steps: report.input_steps.clone(),
            target_resolution: report
                .target_resolution
                .iter()
                .map(format_target_resolution_row)
                .collect(),
            focus_resolution: report
                .focus_resolution
                .iter()
                .map(format_focus_resolution_row)
                .collect(),
            state_transitions: report
                .state_transitions
                .iter()
                .map(format_state_transition_row)
                .collect(),
            runtime_facts: report
                .runtime_facts
                .iter()
                .map(format_runtime_fact_row)
                .collect(),
            runtime_events: report
                .runtime_events
                .iter()
                .map(format_runtime_event_row)
                .collect(),
            semantic_outcomes: report
                .semantic_outcomes
                .iter()
                .map(format_semantic_outcome_row)
                .collect(),
            suppressed_events: report
                .suppressed_events
                .iter()
                .map(format_suppressed_event_row)
                .collect(),
            no_target_events: report
                .no_target_events
                .iter()
                .map(format_no_target_event_row)
                .collect(),
            boundary_assertions: report.boundary_assertions,
        }
    }
}

/// Complete proof frame that can be mounted by a renderer or inspected in tests.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InteractionProofFrame {
    /// Semantic visible proof model carried by this proof frame.
    pub proof: InteractionVisualProof,
}

impl InteractionProofFrame {
    /// Wraps a semantic visible proof in a reusable proof frame.
    pub fn new(proof: InteractionVisualProof) -> Self {
        Self { proof }
    }
}
