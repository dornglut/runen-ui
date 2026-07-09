//! File: domain/ui/ui_runtime/src/input/pointer/graph_canvas.rs
//! Purpose: Graph canvas pointer gesture dispatch.

use ui_input::{
    EventPropagation, FocusChange, FocusTargetId, InputResponse, PointerCapture, PointerEvent,
    PointerEventKind,
};

use crate::{
    ComputedLayoutMap, UiInputOutcome, UiInteraction, UiInteractionResults, UiNodeKind,
    UiRuntimeState, UiTree, WidgetId, hit_test_widget,
};

use super::{
    helpers::{
        find_node, is_pointer_responsive, outcome, push_focus_change_if_needed,
        push_pressed_change_if_needed,
    },
    hover::push_hover_change_if_needed,
};

pub(super) fn dispatch_graph_canvas_pointer_event(
    tree: &UiTree,
    layouts: &ComputedLayoutMap,
    state: &mut UiRuntimeState,
    target: WidgetId,
    event: &PointerEvent,
) -> UiInputOutcome {
    let Some(graph_canvas) = graph_canvas_node(tree, target) else {
        return UiInputOutcome::ignored();
    };
    let Some(layout) = layouts.get(&target) else {
        return UiInputOutcome::ignored();
    };
    let viewport = state
        .graph_canvas_viewports
        .get(&target)
        .copied()
        .unwrap_or(graph_canvas.canvas.viewport);
    let graph_point = graph_point_for_pointer(layout.bounds, viewport, event.position);
    let hit = graph_canvas.canvas.hit_test_scene.hit_test(graph_point);
    let modifiers = graph_input_modifiers(event.modifiers);

    match event.kind {
        PointerEventKind::Down => {
            let previous_hovered = state.hovered_widget;
            let previous_pressed = state.pressed_widget;
            state.hovered_widget = Some(target);
            state.pressed_widget = Some(target);
            state.captured_widget = graph_canvas.capture_pointer_drag.then_some(target);
            state.middle_pan_anchor = None;
            state.middle_pan_last_position = None;
            state.scrollbar_thumb_drag = None;

            let focus_change = if graph_canvas.focusable {
                let focus = FocusTargetId(target.0);
                if state.focused_target == Some(focus) {
                    FocusChange::None
                } else {
                    state.focused_target = Some(focus);
                    FocusChange::Set(focus)
                }
            } else {
                FocusChange::None
            };

            let mut interactions = UiInteractionResults::new();
            push_hover_change_if_needed(&mut interactions, previous_hovered, Some(target));
            push_pressed_change_if_needed(&mut interactions, previous_pressed, Some(target));
            push_focus_change_if_needed(&mut interactions, focus_change);
            push_graph_selection_intent(&mut interactions, target, &hit, modifiers);
            if let Some(action) = state
                .graph_canvas_gestures
                .entry(target)
                .or_default()
                .begin_pointer(hit, graph_point, modifiers)
            {
                interactions.push(UiInteraction::GraphCanvasAction { target, action });
            }

            outcome(
                Some(target),
                InputResponse {
                    propagation: EventPropagation::Stop,
                    capture: if graph_canvas.capture_pointer_drag {
                        PointerCapture::CaptureSelf
                    } else {
                        PointerCapture::None
                    },
                    focus_change,
                    repaint: true,
                    relayout: false,
                },
                interactions,
            )
        }
        PointerEventKind::Move => {
            let previous_hovered = state.hovered_widget;
            state.hovered_widget = Some(target);
            state.hovered_scrollbar = None;

            let action = state
                .graph_canvas_gestures
                .entry(target)
                .or_default()
                .update_pointer(graph_point, hit);
            if let Some(ui_graph_editor::GraphCanvasAction::Pan { phase, delta }) = action.as_ref()
                && matches!(
                    phase,
                    ui_graph_editor::GraphGesturePhase::Begin
                        | ui_graph_editor::GraphGesturePhase::Update
                )
            {
                let current = state
                    .graph_canvas_viewports
                    .get(&target)
                    .copied()
                    .unwrap_or(graph_canvas.canvas.viewport);
                state
                    .graph_canvas_viewports
                    .insert(target, current.pan_by(*delta));
            }

            let mut interactions = UiInteractionResults::new();
            push_hover_change_if_needed(&mut interactions, previous_hovered, Some(target));
            if let Some(action) = action {
                interactions.push(UiInteraction::GraphCanvasAction { target, action });
            }

            outcome(
                Some(target),
                InputResponse {
                    propagation: EventPropagation::Stop,
                    capture: PointerCapture::CaptureSelf,
                    focus_change: FocusChange::None,
                    repaint: true,
                    relayout: false,
                },
                interactions,
            )
        }
        PointerEventKind::Up => {
            let previous_hovered = state.hovered_widget;
            let previous_pressed = state.pressed_widget;
            let release_target = hit_test_widget(tree, layouts, event.position)
                .filter(|widget_id| is_pointer_responsive(tree, *widget_id));
            state.hovered_widget = release_target;
            state.hovered_scrollbar = None;
            state.pressed_widget = None;
            state.captured_widget = None;
            state.middle_pan_anchor = None;
            state.middle_pan_last_position = None;

            let action = state
                .graph_canvas_gestures
                .entry(target)
                .or_default()
                .end_pointer(graph_point, hit);

            let mut interactions = UiInteractionResults::new();
            push_hover_change_if_needed(&mut interactions, previous_hovered, release_target);
            push_pressed_change_if_needed(&mut interactions, previous_pressed, None);
            if let Some(action) = action {
                interactions.push(UiInteraction::GraphCanvasAction { target, action });
            }

            outcome(
                Some(target),
                InputResponse {
                    propagation: EventPropagation::Stop,
                    capture: PointerCapture::Release,
                    focus_change: FocusChange::None,
                    repaint: true,
                    relayout: false,
                },
                interactions,
            )
        }
        PointerEventKind::Scroll => {
            let previous_hovered = state.hovered_widget;
            state.hovered_widget = Some(target);
            state.hovered_scrollbar = None;

            let previous_zoom_milli = viewport.zoom_milli;
            let next_viewport = viewport.zoom_by_wheel_delta(event.delta.y.round() as i32);
            state.graph_canvas_viewports.insert(target, next_viewport);

            let mut interactions = UiInteractionResults::new();
            push_hover_change_if_needed(&mut interactions, previous_hovered, Some(target));
            interactions.push(UiInteraction::GraphCanvasAction {
                target,
                action: ui_graph_editor::GraphCanvasAction::Zoom {
                    anchor: graph_point,
                    previous_zoom_milli,
                    zoom_milli: next_viewport.zoom_milli,
                },
            });

            outcome(
                Some(target),
                InputResponse {
                    propagation: EventPropagation::Stop,
                    capture: PointerCapture::None,
                    focus_change: FocusChange::None,
                    repaint: true,
                    relayout: false,
                },
                interactions,
            )
        }
        PointerEventKind::Enter | PointerEventKind::Leave => UiInputOutcome::ignored(),
    }
}

fn graph_point_for_pointer(
    canvas_bounds: ui_math::UiRect,
    viewport: ui_graph_editor::GraphViewport,
    position: ui_math::UiPoint,
) -> ui_graph_editor::GraphPoint {
    viewport.screen_to_graph_point(ui_graph_editor::GraphPoint::new(
        (position.x - canvas_bounds.x).round() as i32,
        (position.y - canvas_bounds.y).round() as i32,
    ))
}

fn graph_input_modifiers(modifiers: ui_input::Modifiers) -> ui_graph_editor::GraphInputModifiers {
    ui_graph_editor::GraphInputModifiers {
        shift: modifiers.shift,
        ctrl: modifiers.ctrl,
        alt: modifiers.alt,
        meta: modifiers.meta,
    }
}

fn push_graph_selection_intent(
    interactions: &mut UiInteractionResults,
    target: WidgetId,
    hit: &ui_graph_editor::GraphHitTarget,
    modifiers: ui_graph_editor::GraphInputModifiers,
) {
    let additive = modifiers.shift || modifiers.ctrl || modifiers.meta;
    let action = match hit {
        ui_graph_editor::GraphHitTarget::NodeBody(node) => {
            Some(ui_graph_editor::GraphCanvasAction::SelectNode {
                node: *node,
                additive,
            })
        }
        ui_graph_editor::GraphHitTarget::Edge(edge) => {
            Some(ui_graph_editor::GraphCanvasAction::SelectEdge {
                edge: *edge,
                additive,
            })
        }
        ui_graph_editor::GraphHitTarget::Background | ui_graph_editor::GraphHitTarget::Empty => {
            Some(ui_graph_editor::GraphCanvasAction::ClearSelection)
        }
        ui_graph_editor::GraphHitTarget::Port(_)
        | ui_graph_editor::GraphHitTarget::Selection(_) => None,
    };
    if let Some(action) = action {
        interactions.push(UiInteraction::GraphCanvasAction { target, action });
    }
}

pub(super) fn graph_canvas_node(
    tree: &UiTree,
    widget_id: WidgetId,
) -> Option<&crate::GraphCanvasNode> {
    let node = find_node(tree, widget_id)?;
    let UiNodeKind::GraphCanvas(graph_canvas) = &node.kind else {
        return None;
    };
    Some(graph_canvas)
}
