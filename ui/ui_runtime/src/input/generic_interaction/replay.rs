//! Deterministic replay of normalized input against mounted controls.

use ui_controls::{
    ControlInteractionOutcome, ControlInteractionRequirement, ControlInteractionTrigger,
};
use ui_input::{
    FocusChange, FocusDirection, Key, KeyState, NormalizedInputFact, NormalizedInputSample,
    PointerEventKind, PointerInputFact,
};

use crate::WidgetId;

use super::boundary::InteractionBoundaryAssertions;
use super::fixture::{MountedInteractionControl, MountedInteractionFixture};
use super::report::{
    InteractionFocusResolution, InteractionFormationReport, InteractionStateTransition,
    InteractionTargetResolution, RuntimeControlDescriptorFact, RuntimeControlInteractionEvent,
    RuntimeInteractionFact, RuntimeInteractionOutcome, RuntimeNoTargetInteraction,
    RuntimeSuppressedInteraction,
};

/// Deterministic sequence of normalized input samples for interaction replay.
#[derive(Debug, Clone, PartialEq)]
pub struct InteractionReplayScript {
    /// Stable replay id recorded by the formation report.
    pub replay_id: String,

    /// Ordered normalized input samples resolved by replay.
    pub steps: Vec<InteractionReplayStep>,
}

impl InteractionReplayScript {
    /// Creates an empty deterministic replay script.
    pub fn new(replay_id: impl Into<String>) -> Self {
        Self {
            replay_id: replay_id.into(),
            steps: Vec::new(),
        }
    }

    /// Appends one replay step.
    pub fn with_step(mut self, step: InteractionReplayStep) -> Self {
        self.steps.push(step);
        self
    }
}

/// One replay step and the normalized input sample formed by `ui_input`.
#[derive(Debug, Clone, PartialEq)]
pub struct InteractionReplayStep {
    /// Stable step id used by runtime report rows.
    pub step_id: String,

    /// Normalized input sample to resolve for this step.
    pub sample: NormalizedInputSample,
}

impl InteractionReplayStep {
    /// Creates one replay step from a normalized input sample.
    pub fn new(step_id: impl Into<String>, sample: NormalizedInputSample) -> Self {
        Self {
            step_id: step_id.into(),
            sample,
        }
    }
}

/// Auditable report produced by resolving normalized input against a fixture.
///
/// The report separates target/focus resolution, runtime facts/events,
/// semantic outcomes, negative evidence, and no-bypass boundary counters.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
struct ReplayState {
    hovered: Option<WidgetId>,
    pressed: Option<WidgetId>,
    captured: Option<WidgetId>,
    focused: Option<WidgetId>,
    focus_visible: bool,
}

/// Replays normalized input facts against mounted descriptor-backed controls.
///
/// Replay emits reusable facts, events, outcomes, negative evidence, and
/// boundary counters only. It does not execute host commands, mutate product
/// state, open overlays, or perform text editing.
pub fn replay_interactions(
    fixture: &MountedInteractionFixture,
    script: &InteractionReplayScript,
) -> InteractionFormationReport {
    let mut state = ReplayState::default();
    let mut report = InteractionFormationReport {
        replay_id: script.replay_id.clone(),
        mounted_story_id: fixture.mounted_story_id.clone(),
        control_descriptors: fixture
            .controls
            .iter()
            .map(|control| RuntimeControlDescriptorFact {
                widget_id: control.widget_id,
                control_kind_id: control.control_kind_id.clone(),
                interaction_states: control.descriptor.summary().states,
                interaction_triggers: control.descriptor.summary().triggers,
                interaction_outcomes: control.descriptor.summary().outcomes,
            })
            .collect(),
        input_steps: Vec::new(),
        target_resolution: Vec::new(),
        focus_resolution: Vec::new(),
        state_transitions: Vec::new(),
        runtime_facts: Vec::new(),
        runtime_events: Vec::new(),
        semantic_outcomes: Vec::new(),
        suppressed_events: Vec::new(),
        no_target_events: Vec::new(),
        boundary_assertions: InteractionBoundaryAssertions::default(),
    };

    for step in &script.steps {
        report.input_steps.push(step.step_id.clone());
        for fact in &step.sample.facts {
            apply_fact(fixture, step, fact, &mut state, &mut report);
        }
    }

    report
}

fn apply_fact(
    fixture: &MountedInteractionFixture,
    step: &InteractionReplayStep,
    fact: &NormalizedInputFact,
    state: &mut ReplayState,
    report: &mut InteractionFormationReport,
) {
    match fact {
        NormalizedInputFact::Pointer(pointer) => {
            apply_pointer_fact(fixture, step, pointer, state, report);
        }
        NormalizedInputFact::Focus(focus) => {
            apply_focus_fact(fixture, step, focus, state, report);
        }
        NormalizedInputFact::Keyboard(keyboard) => {
            let Some(target) = state.focused.and_then(|id| fixture.control(id)) else {
                report.no_target_events.push(RuntimeNoTargetInteraction {
                    step_id: step.step_id.clone(),
                    reason: "keyboard.no_focus".to_owned(),
                });
                return;
            };
            let trigger = match (&keyboard.key, keyboard.state) {
                (Key::Enter | Key::Space, KeyState::Pressed) => {
                    Some(ControlInteractionTrigger::KeyboardActivate)
                }
                (Key::Up | Key::Down | Key::Left | Key::Right, KeyState::Pressed) => {
                    Some(ControlInteractionTrigger::KeyboardNavigate)
                }
                _ => None,
            };
            let Some(trigger) = trigger else {
                return;
            };
            if !target.enabled {
                suppress_disabled_for_trigger(report, step, target, trigger);
                return;
            }
            let Some(requirement) = requirement_for(target, trigger) else {
                suppress_if_declared(report, step, target, "trigger.not_declared");
                return;
            };
            if requirement.requires_focus && state.focused != Some(target.widget_id) {
                suppress_if_declared(report, step, target, "focus.required");
                return;
            }
            event(report, step, target.widget_id, trigger.as_str());
            emit_declared_outcomes(report, step, target, requirement);
            if trigger == ControlInteractionTrigger::KeyboardNavigate {
                transition(report, step, target.widget_id, "active", true);
            }
        }
        NormalizedInputFact::Semantic(semantic) => {
            let Some(target) = state.focused.and_then(|id| fixture.control(id)) else {
                report.no_target_events.push(RuntimeNoTargetInteraction {
                    step_id: step.step_id.clone(),
                    reason: "semantic.no_focus".to_owned(),
                });
                return;
            };
            if !target.enabled {
                suppress_disabled_for_trigger(
                    report,
                    step,
                    target,
                    ControlInteractionTrigger::SemanticAction,
                );
                return;
            }
            let Some(requirement) =
                requirement_for(target, ControlInteractionTrigger::SemanticAction)
            else {
                suppress_if_declared(report, step, target, "trigger.not_declared");
                return;
            };
            if requirement.requires_focus && state.focused != Some(target.widget_id) {
                suppress_if_declared(report, step, target, "focus.required");
                return;
            }
            if matches!(semantic.event.action, ui_input::UiSemanticAction::Activate) {
                event(
                    report,
                    step,
                    target.widget_id,
                    ControlInteractionTrigger::SemanticAction.as_str(),
                );
                emit_declared_outcomes(report, step, target, requirement);
            }
        }
        NormalizedInputFact::TextIntent(_) => {
            let Some(target) = state.focused.and_then(|id| fixture.control(id)) else {
                report.no_target_events.push(RuntimeNoTargetInteraction {
                    step_id: step.step_id.clone(),
                    reason: "text_intent.no_focus".to_owned(),
                });
                return;
            };
            if !target.enabled {
                suppress_disabled_for_trigger(
                    report,
                    step,
                    target,
                    ControlInteractionTrigger::TextIntent,
                );
                return;
            }
            if !target.descriptor.text_intent_probe {
                suppress_if_declared(report, step, target, "text_intent.not_declared");
                return;
            }
            let Some(requirement) = requirement_for(target, ControlInteractionTrigger::TextIntent)
            else {
                suppress_if_declared(report, step, target, "trigger.not_declared");
                return;
            };
            if requirement.requires_focus && state.focused != Some(target.widget_id) {
                suppress_if_declared(report, step, target, "focus.required");
                return;
            }
            if target.read_only {
                transition(report, step, target.widget_id, "read-only", true);
                runtime_fact(report, target.widget_id, "text-intent-read-only-probe");
                event(
                    report,
                    step,
                    target.widget_id,
                    ControlInteractionTrigger::TextIntent.as_str(),
                );
                event(
                    report,
                    step,
                    target.widget_id,
                    "text_intent_read_only_probe",
                );
            } else {
                runtime_fact(report, target.widget_id, "text-intent-probe");
                event(
                    report,
                    step,
                    target.widget_id,
                    ControlInteractionTrigger::TextIntent.as_str(),
                );
            }
            emit_declared_outcomes(report, step, target, requirement);
        }
        NormalizedInputFact::TextEdit(_)
        | NormalizedInputFact::TextComposition(_)
        | NormalizedInputFact::TextSelection(_) => {}
    }
}

fn apply_pointer_fact(
    fixture: &MountedInteractionFixture,
    step: &InteractionReplayStep,
    pointer: &PointerInputFact,
    state: &mut ReplayState,
    report: &mut InteractionFormationReport,
) {
    match pointer.kind {
        PointerEventKind::Move | PointerEventKind::Enter | PointerEventKind::Scroll => {
            let target = fixture.target_at(pointer.position);
            push_target_resolution(
                report,
                step,
                target.map(|control| control.widget_id),
                target
                    .map(|_| "pointer.target_resolved")
                    .unwrap_or("pointer.no_target"),
            );
            let Some(control) = target else {
                no_target(report, step, "pointer.no_target");
                return;
            };
            let trigger = pointer_trigger(pointer.kind);
            if !control.enabled {
                suppress_disabled_for_trigger(report, step, control, trigger);
                return;
            }
            let Some(requirement) = requirement_for(control, trigger) else {
                suppress_if_declared(report, step, control, "trigger.not_declared");
                return;
            };
            if pointer.kind == PointerEventKind::Scroll {
                event(report, step, control.widget_id, "pointer_scroll_fact");
                emit_declared_outcomes(report, step, control, requirement);
                return;
            }
            state.hovered = Some(control.widget_id);
            transition(report, step, control.widget_id, "hovered", true);
            event(report, step, control.widget_id, trigger.as_str());
            emit_declared_outcomes(report, step, control, requirement);
        }
        PointerEventKind::Down => {
            let target = fixture.target_at(pointer.position);
            push_target_resolution(
                report,
                step,
                target.map(|control| control.widget_id),
                target
                    .map(|_| "pointer.target_resolved")
                    .unwrap_or("pointer.no_target"),
            );
            let Some(control) = target else {
                no_target(report, step, "pointer.no_target");
                return;
            };
            if !control.enabled {
                suppress_disabled_for_trigger(
                    report,
                    step,
                    control,
                    ControlInteractionTrigger::PointerPress,
                );
                return;
            }
            if requirement_for(control, ControlInteractionTrigger::PointerPress).is_none() {
                suppress_if_declared(report, step, control, "trigger.not_declared");
                return;
            }

            state.pressed = Some(control.widget_id);
            state.captured = Some(control.widget_id);
            if control.focusable
                && requirement_for(control, ControlInteractionTrigger::Focus).is_some()
            {
                set_focus_state(fixture, step, state, report, Some(control.widget_id), false);
            }
            transition(report, step, control.widget_id, "pressed", true);
            transition(report, step, control.widget_id, "captured", true);
            event(
                report,
                step,
                control.widget_id,
                ControlInteractionTrigger::PointerPress.as_str(),
            );
        }
        PointerEventKind::Up => {
            let hit_target = fixture.target_at(pointer.position);
            let pressed = state.pressed.take();
            let captured = state.captured.take();
            push_target_resolution(
                report,
                step,
                hit_target.map(|control| control.widget_id),
                match (captured, hit_target.map(|control| control.widget_id)) {
                    (Some(captured), Some(hit)) if captured == hit => "pointer.release_inside",
                    (Some(_), _) => "pointer.release_outside",
                    (None, Some(_)) => "pointer.release_without_capture",
                    (None, None) => "pointer.no_target",
                },
            );

            let Some(pressed_id) = pressed else {
                if let Some(control) = hit_target {
                    suppress_if_declared(report, step, control, "pointer.release_without_capture");
                } else {
                    no_target(report, step, "pointer.no_target");
                }
                return;
            };
            let Some(control) = fixture.control(pressed_id) else {
                no_target(report, step, "pointer.capture_missing");
                return;
            };
            transition(report, step, control.widget_id, "pressed", false);
            transition(report, step, control.widget_id, "captured", false);

            if !control.enabled {
                suppress_disabled_for_trigger(
                    report,
                    step,
                    control,
                    ControlInteractionTrigger::PointerCancel,
                );
                return;
            }
            if requirement_for(control, ControlInteractionTrigger::PointerRelease).is_some() {
                event(
                    report,
                    step,
                    control.widget_id,
                    ControlInteractionTrigger::PointerRelease.as_str(),
                );
            } else {
                suppress_if_declared(report, step, control, "trigger.not_declared");
                return;
            }

            if hit_target.map(|target| target.widget_id) == Some(pressed_id) {
                let Some(requirement) =
                    requirement_for(control, ControlInteractionTrigger::PointerActivate)
                else {
                    suppress_if_declared(report, step, control, "trigger.not_declared");
                    return;
                };
                event(
                    report,
                    step,
                    control.widget_id,
                    ControlInteractionTrigger::PointerActivate.as_str(),
                );
                emit_declared_outcomes(report, step, control, requirement);
            } else {
                if requirement_for(control, ControlInteractionTrigger::PointerCancel).is_some() {
                    event(
                        report,
                        step,
                        control.widget_id,
                        ControlInteractionTrigger::PointerCancel.as_str(),
                    );
                }
                suppress_if_declared(report, step, control, "pointer.release_outside");
            }
        }
        PointerEventKind::Leave => {
            let target = fixture
                .target_at(pointer.position)
                .or_else(|| state.hovered.and_then(|id| fixture.control(id)))
                .or_else(|| state.captured.and_then(|id| fixture.control(id)));
            push_target_resolution(
                report,
                step,
                target.map(|control| control.widget_id),
                target
                    .map(|_| "pointer.leave_target_resolved")
                    .unwrap_or("pointer.no_target"),
            );
            let Some(control) = target else {
                state.hovered = None;
                no_target(report, step, "pointer.no_target");
                return;
            };
            state.hovered = None;
            transition(report, step, control.widget_id, "hovered", false);
            if state.pressed == Some(control.widget_id) {
                event(
                    report,
                    step,
                    control.widget_id,
                    "pointer_leave_kept_capture",
                );
            }
        }
    }
}

fn apply_focus_fact(
    fixture: &MountedInteractionFixture,
    step: &InteractionReplayStep,
    focus: &ui_input::FocusInputFact,
    state: &mut ReplayState,
    report: &mut InteractionFormationReport,
) {
    let mut reason = "focus.no_change";
    if let Some(direction) = focus.direction {
        let next = traverse_focus(fixture, state.focused, direction);
        set_focus_state(fixture, step, state, report, next, focus.focus_visible);
        reason = if next.is_some() {
            "focus.target_resolved"
        } else {
            "focus.target_missing"
        };
    } else {
        match focus.change {
            FocusChange::None => {}
            FocusChange::Set(target) => {
                let requested = WidgetId(target.0);
                match validate_focus_target(fixture, requested) {
                    Ok(control) => {
                        set_focus_state(
                            fixture,
                            step,
                            state,
                            report,
                            Some(control.widget_id),
                            focus.focus_visible,
                        );
                        reason = "focus.target_resolved";
                    }
                    Err(FocusTargetRejection::Missing) => {
                        no_target(report, step, "focus.target_missing");
                        reason = "focus.target_missing";
                    }
                    Err(FocusTargetRejection::Disabled(control)) => {
                        suppress_if_declared(report, step, control, "focus.target_disabled");
                        reason = "focus.target_disabled";
                    }
                    Err(FocusTargetRejection::NotFocusable(control)) => {
                        suppress_if_declared(report, step, control, "focus.target_not_focusable");
                        reason = "focus.target_not_focusable";
                    }
                    Err(FocusTargetRejection::DoesNotDeclareFocus(control)) => {
                        suppress_if_declared(
                            report,
                            step,
                            control,
                            "focus.target_does_not_declare_focus",
                        );
                        reason = "focus.target_does_not_declare_focus";
                    }
                }
            }
            FocusChange::Clear => {
                set_focus_state(fixture, step, state, report, None, false);
                reason = "focus.cleared";
            }
        }
    }
    report.focus_resolution.push(InteractionFocusResolution {
        step_id: step.step_id.clone(),
        focused: state.focused,
        focus_visible: state.focus_visible,
        reason: reason.to_owned(),
    });
}

fn set_focus_state(
    fixture: &MountedInteractionFixture,
    step: &InteractionReplayStep,
    state: &mut ReplayState,
    report: &mut InteractionFormationReport,
    next: Option<WidgetId>,
    next_focus_visible: bool,
) {
    let previous = state.focused;
    let previous_focus_visible = state.focus_visible;

    if previous != next {
        if let Some(previous) = previous {
            transition(report, step, previous, "focused", false);
            if previous_focus_visible {
                transition(report, step, previous, "focus-visible", false);
            }
        }
        state.focused = next;
        state.focus_visible = next_focus_visible;
        if let Some(next) = next
            && let Some(control) = fixture.control(next)
            && requirement_for(control, ControlInteractionTrigger::Focus).is_some()
        {
            transition(report, step, next, "focused", true);
            if next_focus_visible {
                transition(report, step, next, "focus-visible", true);
            }
        }
        return;
    }

    state.focus_visible = next_focus_visible;
    if previous_focus_visible != next_focus_visible
        && let Some(current) = next
    {
        transition(report, step, current, "focus-visible", next_focus_visible);
    }
}

enum FocusTargetRejection<'a> {
    Missing,
    Disabled(&'a MountedInteractionControl),
    NotFocusable(&'a MountedInteractionControl),
    DoesNotDeclareFocus(&'a MountedInteractionControl),
}

fn validate_focus_target(
    fixture: &MountedInteractionFixture,
    target: WidgetId,
) -> Result<&MountedInteractionControl, FocusTargetRejection<'_>> {
    let Some(control) = fixture.control(target) else {
        return Err(FocusTargetRejection::Missing);
    };
    if !control.enabled {
        return Err(FocusTargetRejection::Disabled(control));
    }
    if !control.focusable {
        return Err(FocusTargetRejection::NotFocusable(control));
    }
    if requirement_for(control, ControlInteractionTrigger::Focus).is_none() {
        return Err(FocusTargetRejection::DoesNotDeclareFocus(control));
    }
    Ok(control)
}

fn pointer_trigger(kind: PointerEventKind) -> ControlInteractionTrigger {
    match kind {
        PointerEventKind::Move | PointerEventKind::Enter | PointerEventKind::Leave => {
            ControlInteractionTrigger::PointerHover
        }
        PointerEventKind::Down => ControlInteractionTrigger::PointerPress,
        PointerEventKind::Up => ControlInteractionTrigger::PointerRelease,
        PointerEventKind::Scroll => ControlInteractionTrigger::PointerHover,
    }
}

fn requirement_for(
    control: &MountedInteractionControl,
    trigger: ControlInteractionTrigger,
) -> Option<&ControlInteractionRequirement> {
    control
        .descriptor
        .requirements
        .iter()
        .find(|requirement| requirement.trigger == trigger)
}

fn emit_declared_outcomes(
    report: &mut InteractionFormationReport,
    step: &InteractionReplayStep,
    control: &MountedInteractionControl,
    requirement: &ControlInteractionRequirement,
) {
    for declared_outcome in &requirement.outcomes {
        outcome(report, step, control.widget_id, *declared_outcome);
    }
}

fn suppress_disabled_for_trigger(
    report: &mut InteractionFormationReport,
    step: &InteractionReplayStep,
    control: &MountedInteractionControl,
    trigger: ControlInteractionTrigger,
) {
    let Some(requirement) = requirement_for(control, trigger) else {
        suppress_if_declared(report, step, control, "trigger.not_declared");
        return;
    };
    if requirement.suppresses_when_disabled {
        suppress_if_declared(report, step, control, "control.disabled");
    }
}

fn traverse_focus(
    fixture: &MountedInteractionFixture,
    current: Option<WidgetId>,
    direction: FocusDirection,
) -> Option<WidgetId> {
    let focusable = fixture
        .focusable()
        .map(|control| control.widget_id)
        .collect::<Vec<_>>();
    if focusable.is_empty() {
        return None;
    }
    let current_index =
        current.and_then(|id| focusable.iter().position(|candidate| *candidate == id));
    match direction {
        FocusDirection::Previous | FocusDirection::Left | FocusDirection::Up => current_index
            .map(|index| {
                if index == 0 {
                    focusable.len() - 1
                } else {
                    index - 1
                }
            })
            .or(Some(0))
            .map(|index| focusable[index]),
        FocusDirection::Next | FocusDirection::Right | FocusDirection::Down => current_index
            .map(|index| (index + 1) % focusable.len())
            .or(Some(0))
            .map(|index| focusable[index]),
    }
}

fn push_target_resolution(
    report: &mut InteractionFormationReport,
    step: &InteractionReplayStep,
    target: Option<WidgetId>,
    reason: &str,
) {
    report.target_resolution.push(InteractionTargetResolution {
        step_id: step.step_id.clone(),
        target,
        reason: reason.to_owned(),
    });
}

fn no_target(report: &mut InteractionFormationReport, step: &InteractionReplayStep, reason: &str) {
    report.no_target_events.push(RuntimeNoTargetInteraction {
        step_id: step.step_id.clone(),
        reason: reason.to_owned(),
    });
}

fn transition(
    report: &mut InteractionFormationReport,
    step: &InteractionReplayStep,
    target: WidgetId,
    state: &str,
    active: bool,
) {
    report.state_transitions.push(InteractionStateTransition {
        step_id: step.step_id.clone(),
        target,
        state: state.to_owned(),
        active,
    });
    if active {
        report.runtime_facts.push(RuntimeInteractionFact {
            target,
            fact: state.to_owned(),
        });
    }
}

fn runtime_fact(report: &mut InteractionFormationReport, target: WidgetId, fact: &str) {
    report.runtime_facts.push(RuntimeInteractionFact {
        target,
        fact: fact.to_owned(),
    });
}

fn event(
    report: &mut InteractionFormationReport,
    step: &InteractionReplayStep,
    target: WidgetId,
    event: &str,
) {
    report.runtime_events.push(RuntimeControlInteractionEvent {
        step_id: step.step_id.clone(),
        target,
        event: event.to_owned(),
    });
}

fn outcome(
    report: &mut InteractionFormationReport,
    step: &InteractionReplayStep,
    target: WidgetId,
    outcome_value: ControlInteractionOutcome,
) {
    report.semantic_outcomes.push(RuntimeInteractionOutcome {
        step_id: step.step_id.clone(),
        target,
        outcome: outcome_value.as_str().to_owned(),
    });
}

fn suppress_if_declared(
    report: &mut InteractionFormationReport,
    step: &InteractionReplayStep,
    control: &MountedInteractionControl,
    reason: &str,
) {
    transition(report, step, control.widget_id, "suppressed", true);
    report.suppressed_events.push(RuntimeSuppressedInteraction {
        step_id: step.step_id.clone(),
        target: control.widget_id,
        reason: reason.to_owned(),
    });
}
