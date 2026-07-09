//! Report-row string formatting helpers for generic interaction proof output.

use super::report::{
    InteractionFocusResolution, InteractionStateTransition, InteractionTargetResolution,
    RuntimeControlInteractionEvent, RuntimeInteractionFact, RuntimeInteractionOutcome,
    RuntimeNoTargetInteraction, RuntimeSuppressedInteraction,
};

pub(super) fn format_target_resolution_row(resolution: &InteractionTargetResolution) -> String {
    format!(
        "{}:{:?}:{}",
        resolution.step_id, resolution.target, resolution.reason
    )
}

pub(super) fn format_focus_resolution_row(resolution: &InteractionFocusResolution) -> String {
    format!(
        "{}:{:?}:{}:{}",
        resolution.step_id, resolution.focused, resolution.focus_visible, resolution.reason
    )
}

pub(super) fn format_state_transition_row(transition: &InteractionStateTransition) -> String {
    format!(
        "{}:{:?}:{}:{}",
        transition.step_id, transition.target, transition.state, transition.active
    )
}

pub(super) fn format_runtime_fact_row(fact: &RuntimeInteractionFact) -> String {
    format!("{:?}:{}", fact.target, fact.fact)
}

pub(super) fn format_runtime_event_row(event: &RuntimeControlInteractionEvent) -> String {
    format!("{}:{:?}:{}", event.step_id, event.target, event.event)
}

pub(super) fn format_semantic_outcome_row(outcome: &RuntimeInteractionOutcome) -> String {
    format!(
        "{}:{:?}:{}",
        outcome.step_id, outcome.target, outcome.outcome
    )
}

pub(super) fn format_suppressed_event_row(event: &RuntimeSuppressedInteraction) -> String {
    format!("{}:{:?}:{}", event.step_id, event.target, event.reason)
}

pub(super) fn format_no_target_event_row(event: &RuntimeNoTargetInteraction) -> String {
    format!("{}:{}", event.step_id, event.reason)
}
