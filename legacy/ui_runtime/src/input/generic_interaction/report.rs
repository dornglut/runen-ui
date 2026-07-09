//! Interaction formation report and proof facts.

use crate::WidgetId;

use super::boundary::InteractionBoundaryAssertions;

#[derive(Debug, Clone, PartialEq)]
pub struct InteractionFormationReport {
    /// Replay id copied from the script.
    pub replay_id: String,

    /// Fixture/story id copied from the mounted fixture.
    pub mounted_story_id: String,

    /// Descriptor facts proving replay used package-backed declarations.
    pub control_descriptors: Vec<RuntimeControlDescriptorFact>,

    /// Ordered input step ids processed by replay.
    pub input_steps: Vec<String>,

    /// Pointer/target resolution rows.
    pub target_resolution: Vec<InteractionTargetResolution>,

    /// Focus resolution rows.
    pub focus_resolution: Vec<InteractionFocusResolution>,

    /// Stateful interaction transitions emitted by replay.
    pub state_transitions: Vec<InteractionStateTransition>,

    /// Reusable runtime facts observed during replay.
    pub runtime_facts: Vec<RuntimeInteractionFact>,

    /// Reusable runtime events observed during replay.
    pub runtime_events: Vec<RuntimeControlInteractionEvent>,

    /// Semantic outcomes emitted only from declared requirements.
    pub semantic_outcomes: Vec<RuntimeInteractionOutcome>,

    /// Suppressed interaction evidence for negative cases.
    pub suppressed_events: Vec<RuntimeSuppressedInteraction>,

    /// No-target evidence for unmatched input.
    pub no_target_events: Vec<RuntimeNoTargetInteraction>,

    /// Boundary counters proving no host/product/overlay/text-edit bypass.
    pub boundary_assertions: InteractionBoundaryAssertions,
}

/// Descriptor fact captured for one mounted control during replay.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeControlDescriptorFact {
    /// Mounted widget id for the descriptor fact.
    pub widget_id: WidgetId,

    /// Package control kind id for the mounted control.
    pub control_kind_id: String,

    /// Declared reusable states copied from the package descriptor.
    pub interaction_states: Vec<String>,

    /// Declared reusable triggers copied from the package descriptor.
    pub interaction_triggers: Vec<String>,

    /// Declared semantic outcomes copied from the package descriptor.
    pub interaction_outcomes: Vec<String>,
}

/// Target resolution evidence for one input step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InteractionTargetResolution {
    /// Replay step id that produced this resolution.
    pub step_id: String,

    /// Resolved target, when a target existed.
    pub target: Option<WidgetId>,

    /// Stable resolution reason.
    pub reason: String,
}

/// Focus resolution evidence for one input step.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InteractionFocusResolution {
    /// Replay step id that produced this resolution.
    pub step_id: String,

    /// Focused widget after resolution.
    pub focused: Option<WidgetId>,

    /// Whether focus-visible is active after resolution.
    pub focus_visible: bool,

    /// Stable focus resolution reason.
    pub reason: String,
}

/// Stateful interaction transition emitted by replay.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InteractionStateTransition {
    /// Replay step id that produced the transition.
    pub step_id: String,

    /// Target widget for the transition.
    pub target: WidgetId,

    /// Stable state name.
    pub state: String,

    /// Whether the state became active or inactive.
    pub active: bool,
}

/// Runtime fact formed from normalized input and descriptor requirements.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeInteractionFact {
    /// Target widget for the fact.
    pub target: WidgetId,

    /// Stable fact name.
    pub fact: String,
}

/// Runtime event formed from normalized input and descriptor requirements.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeControlInteractionEvent {
    /// Replay step id that produced the event.
    pub step_id: String,

    /// Target widget for the event.
    pub target: WidgetId,

    /// Stable event name.
    pub event: String,
}

/// Semantic reusable outcome emitted from a declared requirement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeInteractionOutcome {
    /// Replay step id that produced the outcome.
    pub step_id: String,

    /// Target widget for the outcome.
    pub target: WidgetId,

    /// Stable semantic outcome name.
    pub outcome: String,
}

/// Suppressed reusable interaction evidence for one negative case.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeSuppressedInteraction {
    /// Replay step id that produced the suppression.
    pub step_id: String,

    /// Target widget whose interaction was suppressed.
    pub target: WidgetId,

    /// Stable suppression reason.
    pub reason: String,
}

/// No-target reusable interaction evidence for one unmatched input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeNoTargetInteraction {
    /// Replay step id that produced the no-target result.
    pub step_id: String,

    /// Stable no-target reason.
    pub reason: String,
}
