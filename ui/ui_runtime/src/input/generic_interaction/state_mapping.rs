//! Visible-state folding and marker mapping for generic interaction proof.

use super::fixture::MountedInteractionControl;
use super::report::InteractionFormationReport;
use super::visual::{InteractionVisibleState, InteractionVisualMarker};

pub(super) fn push_marker(
    markers: &mut Vec<InteractionVisualMarker>,
    state: InteractionVisibleState,
) {
    if !markers.iter().any(|marker| marker.state == state) {
        markers.push(InteractionVisualMarker::new(state));
    }
}

pub(super) fn folded_current_states(
    control: &MountedInteractionControl,
    report: &InteractionFormationReport,
) -> Vec<InteractionVisibleState> {
    let mut states = Vec::new();
    if !control.enabled {
        push_state(&mut states, InteractionVisibleState::Disabled);
    }
    if control.read_only {
        push_state(&mut states, InteractionVisibleState::ReadOnly);
    }

    for transition in report
        .state_transitions
        .iter()
        .filter(|transition| transition.target == control.widget_id)
    {
        let Some(state) = visible_state_from_transition(&transition.state) else {
            continue;
        };
        if transition.active {
            push_state(&mut states, state);
        } else {
            remove_state(&mut states, state);
        }
    }
    states
}

pub(super) fn push_state(
    states: &mut Vec<InteractionVisibleState>,
    state: InteractionVisibleState,
) {
    if !states.contains(&state) {
        states.push(state);
    }
}

pub(super) fn remove_state(
    states: &mut Vec<InteractionVisibleState>,
    state: InteractionVisibleState,
) {
    states.retain(|existing| *existing != state);
}

pub(super) fn visible_state_from_transition(state: &str) -> Option<InteractionVisibleState> {
    match state {
        "hovered" => Some(InteractionVisibleState::Hovered),
        "pressed" => Some(InteractionVisibleState::Pressed),
        "captured" => Some(InteractionVisibleState::Captured),
        "focused" => Some(InteractionVisibleState::Focused),
        "focus-visible" => Some(InteractionVisibleState::FocusVisible),
        "active" => Some(InteractionVisibleState::Active),
        "read-only" => Some(InteractionVisibleState::ReadOnly),
        "suppressed" => Some(InteractionVisibleState::Suppressed),
        _ => None,
    }
}

pub(super) fn visible_state_from_fact(fact: &str) -> Option<InteractionVisibleState> {
    match fact {
        "text-intent-probe" => Some(InteractionVisibleState::TextIntentProbe),
        "text-intent-read-only-probe" => Some(InteractionVisibleState::ReadOnlyTextIntentProbe),
        _ => visible_state_from_transition(fact),
    }
}

pub(super) fn visible_state_from_outcome(outcome: &str) -> Option<InteractionVisibleState> {
    match outcome {
        "activation-requested" => Some(InteractionVisibleState::ActivationRequested),
        "action-intent" => Some(InteractionVisibleState::ActionIntent),
        "active-item-intent" => Some(InteractionVisibleState::ListActiveItemIntent),
        "node-intent" => Some(InteractionVisibleState::TreeNodeIntent),
        "cell-or-row-intent" => Some(InteractionVisibleState::TableCellOrRowIntent),
        "text-intent-seen" => Some(InteractionVisibleState::TextIntentProbe),
        _ => None,
    }
}
