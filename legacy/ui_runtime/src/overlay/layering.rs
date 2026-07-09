use ui_controls::{ControlOverlayKind, ControlOverlayLayerPreference, ControlOverlayTrigger};
use ui_input::{FocusChange, Key, KeyState, NormalizedInputFact, PointerEventKind};
use ui_math::{UiPoint, UiRect};

use crate::WidgetId;

use super::placement::placement_resolution;
use super::stack::{focus_return_anchor, overlay_scope};
use super::{
    MountedOverlayControl, MountedOverlayLayeringFixture, OverlayBoundaryAssertions,
    OverlayDeclarationEvidence, OverlayDismissalEvidence, OverlayFocusEvidence,
    OverlayKeyboardNavigationEvidence, OverlayLayerAssignment, OverlayLayeringReport,
    OverlayLayeringScript, OverlayLayeringStep, OverlayOpenIntent, OverlayPointerCaptureEvidence,
    OverlayStackEntry, OverlaySuppressionEvidence, OverlayViewportEvidence,
};

pub fn replay_overlay_layering(
    fixture: &MountedOverlayLayeringFixture,
    script: &OverlayLayeringScript,
) -> OverlayLayeringReport {
    let mut state = OverlayReplayState::default();
    let mut viewport_rect = fixture.viewport_rect;
    let mut report = empty_report(fixture, script);
    for step in &script.steps {
        report.input_steps.push(step.step_id.clone());
        if let Some(next_viewport) = step.viewport_rect {
            viewport_rect = next_viewport;
            recompute_placements(
                fixture,
                &step.step_id,
                viewport_rect,
                &mut report,
                true,
                false,
            );
        }
        if let Some(anchor_id) = &step.invalidated_anchor {
            invalidate_anchor(step, anchor_id, &mut report);
        }
        for fact in &step.sample.facts {
            apply_fact(fixture, step, fact, &mut state, viewport_rect, &mut report);
        }
    }
    report
}

#[derive(Default)]
struct OverlayReplayState {
    focused: Option<WidgetId>,
    captured: Option<WidgetId>,
}

fn empty_report(
    fixture: &MountedOverlayLayeringFixture,
    script: &OverlayLayeringScript,
) -> OverlayLayeringReport {
    OverlayLayeringReport {
        replay_id: script.replay_id.clone(),
        fixture_id: fixture.fixture_id.clone(),
        input_steps: Vec::new(),
        declarations: declarations(fixture),
        open_intents: Vec::new(),
        stack_entries: Vec::new(),
        placement_resolutions: Vec::new(),
        layer_assignments: Vec::new(),
        focus_evidence: Vec::new(),
        dismissal_evidence: Vec::new(),
        pointer_capture_evidence: Vec::new(),
        keyboard_navigation_evidence: Vec::new(),
        suppression_evidence: Vec::new(),
        viewport_evidence: Vec::new(),
        boundary_assertions: OverlayBoundaryAssertions::default(),
    }
}

fn declarations(fixture: &MountedOverlayLayeringFixture) -> Vec<OverlayDeclarationEvidence> {
    fixture
        .controls
        .iter()
        .flat_map(|control| {
            control
                .descriptor
                .requirements
                .iter()
                .map(move |requirement| OverlayDeclarationEvidence {
                    source_control_id: control.widget_id,
                    anchor_id: control.anchor_id.clone(),
                    overlay_kind: requirement.kind.as_str().to_owned(),
                    trigger: requirement.trigger.as_str().to_owned(),
                    layer: requirement.layer.as_str().to_owned(),
                    dismiss_policy: requirement.dismiss_policy.as_str().to_owned(),
                    focus_policy: requirement.focus_policy.as_str().to_owned(),
                })
        })
        .collect()
}

fn apply_fact(
    fixture: &MountedOverlayLayeringFixture,
    step: &OverlayLayeringStep,
    fact: &NormalizedInputFact,
    state: &mut OverlayReplayState,
    viewport_rect: UiRect,
    report: &mut OverlayLayeringReport,
) {
    match fact {
        NormalizedInputFact::Pointer(pointer) => match pointer.kind {
            PointerEventKind::Down => {
                state.captured = fixture
                    .target_at(pointer.position)
                    .map(|control| control.widget_id);
                if let Some(control) = fixture.target_at(pointer.position) {
                    open_for_trigger(
                        step,
                        control,
                        ControlOverlayTrigger::PointerPress,
                        viewport_rect,
                        report,
                    );
                    report
                        .pointer_capture_evidence
                        .push(OverlayPointerCaptureEvidence {
                            input_sample_id: step.sample.sample_id.clone(),
                            anchor_id: control.anchor_id.clone(),
                            outside_dismissal_blocked: true,
                            reason: "opening-pointer-capture".to_owned(),
                        });
                } else {
                    dismiss_outside(step, pointer.position, report);
                }
            }
            PointerEventKind::Move | PointerEventKind::Enter => {
                if let Some(control) = fixture.target_at(pointer.position) {
                    open_for_trigger(
                        step,
                        control,
                        ControlOverlayTrigger::PointerHover,
                        viewport_rect,
                        report,
                    );
                }
            }
            PointerEventKind::Scroll => {
                recompute_placements(fixture, &step.step_id, viewport_rect, report, false, true)
            }
            PointerEventKind::Up | PointerEventKind::Leave => {}
        },
        NormalizedInputFact::Focus(focus) => {
            if let FocusChange::Set(target) = focus.change {
                let widget = WidgetId(target.0);
                state.focused = Some(widget);
                if let Some(control) = fixture
                    .controls
                    .iter()
                    .find(|control| control.widget_id == widget)
                {
                    open_for_trigger(
                        step,
                        control,
                        ControlOverlayTrigger::Focus,
                        viewport_rect,
                        report,
                    );
                }
            }
        }
        NormalizedInputFact::Keyboard(keyboard) => {
            if keyboard.state == KeyState::Pressed {
                match keyboard.key {
                    Key::Escape => dismiss_escape(step, report),
                    Key::Up | Key::Down | Key::Left | Key::Right => {
                        report.keyboard_navigation_evidence.push(
                            OverlayKeyboardNavigationEvidence {
                                input_sample_id: step.sample.sample_id.clone(),
                                request_id: report
                                    .stack_entries
                                    .iter()
                                    .rev()
                                    .find(|entry| entry.closed_at_step.is_none())
                                    .map(|entry| entry.request_id.clone()),
                                key: format!("{:?}", keyboard.key),
                                navigation_intent: "overlay-keyboard-navigation".to_owned(),
                                product_commands_executed: 0,
                            },
                        );
                    }
                    _ => {}
                }
            }
        }
        NormalizedInputFact::Semantic(_)
        | NormalizedInputFact::TextIntent(_)
        | NormalizedInputFact::TextEdit(_)
        | NormalizedInputFact::TextComposition(_)
        | NormalizedInputFact::TextSelection(_) => {}
    }
}

fn open_for_trigger(
    step: &OverlayLayeringStep,
    control: &MountedOverlayControl,
    trigger: ControlOverlayTrigger,
    viewport_rect: UiRect,
    report: &mut OverlayLayeringReport,
) {
    let Some(requirement) = control
        .descriptor
        .requirements
        .iter()
        .find(|requirement| requirement.trigger == trigger)
    else {
        suppress(step, Some(control), "trigger.not_declared", report);
        return;
    };
    if !control.enabled && requirement.suppresses_when_disabled {
        suppress(step, Some(control), "anchor.disabled", report);
        return;
    }

    let request_id = format!("overlay.request.{}", report.open_intents.len() + 1);
    let stack_entry_id = format!("overlay.stack-entry.{}", report.stack_entries.len() + 1);
    let parent_request_id = if requirement.layer == ControlOverlayLayerPreference::Submenu {
        report
            .stack_entries
            .iter()
            .rev()
            .find(|entry| entry.closed_at_step.is_none())
            .map(|entry| entry.request_id.clone())
    } else {
        None
    };
    let placement = placement_resolution(control, requirement, &request_id, viewport_rect);
    let hit_region = placement.resolved_rect;

    report.open_intents.push(OverlayOpenIntent {
        input_sample_id: step.sample.sample_id.clone(),
        step_id: step.step_id.clone(),
        source_control_id: control.widget_id,
        anchor_id: control.anchor_id.clone(),
        request_id: request_id.clone(),
        overlay_kind: requirement.kind.as_str().to_owned(),
        trigger: trigger.as_str().to_owned(),
    });
    report.stack_entries.push(OverlayStackEntry {
        stack_entry_id: stack_entry_id.clone(),
        request_id: request_id.clone(),
        parent_request_id,
        scope_id: overlay_scope(requirement),
        anchor_id: control.anchor_id.clone(),
        kind: requirement.kind.as_str().to_owned(),
        layer_class: requirement.layer.as_str().to_owned(),
        opened_at_step: step.step_id.clone(),
        closed_at_step: None,
        hit_regions: vec![hit_region],
        is_topmost_dismissible: requirement.kind != ControlOverlayKind::Tooltip,
    });
    report.placement_resolutions.push(placement);
    report.layer_assignments.push(OverlayLayerAssignment {
        request_id: request_id.clone(),
        layer_id: format!("layer.{}", requirement.layer.as_str()),
        layer_class: requirement.layer.as_str().to_owned(),
        ordinal: report.layer_assignments.len(),
    });
    report.focus_evidence.push(OverlayFocusEvidence {
        request_id,
        focus_policy: requirement.focus_policy.as_str().to_owned(),
        focus_return_target: focus_return_anchor(control, requirement),
    });
    report.boundary_assertions.overlay_open_requests += 1;
    report.boundary_assertions.overlay_opened += 1;
    report.boundary_assertions.overlay_stack_entries_opened += 1;
}

fn dismiss_escape(step: &OverlayLayeringStep, report: &mut OverlayLayeringReport) {
    let Some((index, _entry)) = report
        .stack_entries
        .iter()
        .enumerate()
        .rev()
        .find(|(_, entry)| entry.closed_at_step.is_none() && entry.is_topmost_dismissible)
    else {
        return;
    };
    close_entry(index, "escape.topmost_dismissed", step, None, true, report);
}

fn dismiss_outside(step: &OverlayLayeringStep, point: UiPoint, report: &mut OverlayLayeringReport) {
    if report
        .stack_entries
        .iter()
        .rev()
        .find(|entry| entry.closed_at_step.is_none())
        .is_some_and(|entry| {
            entry
                .hit_regions
                .iter()
                .any(|region| region.contains(point))
        })
    {
        report
            .pointer_capture_evidence
            .push(OverlayPointerCaptureEvidence {
                input_sample_id: step.sample.sample_id.clone(),
                anchor_id: "overlay.active".to_owned(),
                outside_dismissal_blocked: true,
                reason: "inside-active-overlay".to_owned(),
            });
        return;
    }
    let Some((index, _entry)) = report
        .stack_entries
        .iter()
        .enumerate()
        .rev()
        .find(|(_, entry)| entry.closed_at_step.is_none() && entry.is_topmost_dismissible)
    else {
        return;
    };
    close_entry(
        index,
        "outside-pointer.topmost_dismissed",
        step,
        Some(format!("{},{}", point.x, point.y)),
        false,
        report,
    );
}

fn close_entry(
    index: usize,
    reason: &str,
    step: &OverlayLayeringStep,
    outside_pointer_target: Option<String>,
    escape_key_seen: bool,
    report: &mut OverlayLayeringReport,
) {
    let request_id = report.stack_entries[index].request_id.clone();
    let stack_entry_id = report.stack_entries[index].stack_entry_id.clone();
    report.stack_entries[index].closed_at_step = Some(step.step_id.clone());
    report.dismissal_evidence.push(OverlayDismissalEvidence {
        request_id: Some(request_id),
        stack_entry_id: Some(stack_entry_id),
        reason: reason.to_owned(),
        input_sample_id: step.sample.sample_id.clone(),
        focus_return_target: report
            .focus_evidence
            .last()
            .and_then(|evidence| evidence.focus_return_target.clone()),
        outside_pointer_target,
        escape_key_seen,
        closed: true,
        suppressed: false,
    });
    if escape_key_seen {
        report.boundary_assertions.overlay_dismissed_by_escape += 1;
    } else {
        report
            .boundary_assertions
            .overlay_dismissed_by_outside_pointer += 1;
    }
    report.boundary_assertions.overlay_stack_entries_closed += 1;
    report.boundary_assertions.focus_returned += 1;
}

fn recompute_placements(
    fixture: &MountedOverlayLayeringFixture,
    step_id: &str,
    viewport_rect: UiRect,
    report: &mut OverlayLayeringReport,
    viewport_recomputed: bool,
    scroll_recomputed: bool,
) {
    for placement in &mut report.placement_resolutions {
        if let Some(control) = fixture.control_by_anchor(&placement.anchor_id) {
            placement.viewport_rect = viewport_rect;
            placement.anchor_rect = control.bounds;
        }
    }
    report.viewport_evidence.push(OverlayViewportEvidence {
        step_id: step_id.to_owned(),
        viewport_rect,
        scroll_recomputed,
        viewport_recomputed,
        anchor_still_valid: true,
        placement_suppressed: false,
    });
    if scroll_recomputed {
        report.boundary_assertions.placement_recomputed_after_scroll += 1;
    }
    if viewport_recomputed {
        report
            .boundary_assertions
            .placement_recomputed_after_viewport_resize += 1;
    }
}

fn invalidate_anchor(
    step: &OverlayLayeringStep,
    anchor_id: &str,
    report: &mut OverlayLayeringReport,
) {
    let mut closed = false;
    for entry in &mut report.stack_entries {
        if entry.anchor_id == anchor_id && entry.closed_at_step.is_none() {
            entry.closed_at_step = Some(step.step_id.clone());
            closed = true;
        }
    }
    if closed {
        report
            .suppression_evidence
            .push(OverlaySuppressionEvidence {
                anchor_id: Some(anchor_id.to_owned()),
                request_kind: "anchor-invalidation".to_owned(),
                trigger: "runtime-anchor-removed".to_owned(),
                reason: "anchor.removed".to_owned(),
                input_sample_id: step.sample.sample_id.clone(),
                opened: false,
                host_commands_executed: 0,
                product_mutations: 0,
            });
        report.boundary_assertions.anchor_invalidation_suppressed += 1;
        report.boundary_assertions.overlay_suppressed += 1;
    }
}

fn suppress(
    step: &OverlayLayeringStep,
    control: Option<&MountedOverlayControl>,
    reason: &str,
    report: &mut OverlayLayeringReport,
) {
    report
        .suppression_evidence
        .push(OverlaySuppressionEvidence {
            anchor_id: control.map(|control| control.anchor_id.clone()),
            request_kind: "overlay-open".to_owned(),
            trigger: "normalized-input".to_owned(),
            reason: reason.to_owned(),
            input_sample_id: step.sample.sample_id.clone(),
            opened: false,
            host_commands_executed: 0,
            product_mutations: 0,
        });
    report.boundary_assertions.overlay_suppressed += 1;
}
