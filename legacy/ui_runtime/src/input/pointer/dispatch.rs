//! File: domain/ui/ui_runtime/src/input/pointer/dispatch.rs
//! Purpose: Pointer input dispatch entrypoint.

use ui_input::{
    EventPropagation, FocusChange, FocusTargetId, InputResponse, PointerButton, PointerCapture,
    PointerEvent, PointerEventKind,
};

use crate::{
    ComputedLayoutMap, UiInputOutcome, UiInteraction, UiInteractionResults, UiRuntimeState, UiTree,
    WidgetId, hit_test_widget,
    state::{ScrollbarAxisTarget, ScrollbarThumbDragState},
};

use super::{
    graph_canvas::{dispatch_graph_canvas_pointer_event, graph_canvas_node},
    helpers::{
        is_pointer_responsive, outcome, push_focus_change_if_needed, push_pressed_change_if_needed,
    },
    hover::push_hover_change_if_needed,
    middle_pan::{apply_middle_pan_delta, middle_pan_delta, scroll_owners_for_pan},
    numeric::stepped_numeric_value,
    popup::dispatch_popup_stack_outside_dismiss,
    press::activation_for_release,
    scroll::{apply_scroll_wheel_delta, find_scroll_owner_chain},
    scrollbar::{
        apply_scrollbar_thumb_drag, axis_position, axis_rect_start,
        scrollbar_axis_target_at_position, scrollbar_thumb_geometry_at_position,
    },
};

pub fn dispatch_pointer_event(
    tree: &UiTree,
    layouts: &ComputedLayoutMap,
    state: &mut UiRuntimeState,
    event: &PointerEvent,
) -> UiInputOutcome {
    let target = match state.captured_widget {
        Some(widget) => Some(widget),
        None => hit_test_widget(tree, layouts, event.position),
    };

    if matches!(event.kind, PointerEventKind::Move | PointerEventKind::Up)
        && let Some(graph_target) =
            target.filter(|widget_id| graph_canvas_node(tree, *widget_id).is_some())
        && state
            .graph_canvas_gestures
            .get(&graph_target)
            .is_some_and(|gesture| gesture.active.is_some())
    {
        return dispatch_graph_canvas_pointer_event(tree, layouts, state, graph_target, event);
    }

    if matches!(event.kind, PointerEventKind::Scroll)
        && let Some(graph_target) = target.filter(|widget_id| {
            graph_canvas_node(tree, *widget_id).is_some_and(|canvas| canvas.owns_wheel_zoom)
        })
    {
        return dispatch_graph_canvas_pointer_event(tree, layouts, state, graph_target, event);
    }

    match event.kind {
        PointerEventKind::Move | PointerEventKind::Enter => {
            if let Some(drag) = state.scrollbar_thumb_drag {
                state.hovered_scrollbar =
                    Some(ScrollbarAxisTarget::new(drag.scroll_widget, drag.axis));
                let changed =
                    apply_scrollbar_thumb_drag(tree, layouts, state, drag, event.position);
                return outcome(
                    Some(drag.scroll_widget),
                    InputResponse {
                        propagation: EventPropagation::Stop,
                        capture: PointerCapture::CaptureSelf,
                        focus_change: FocusChange::None,
                        repaint: changed,
                        relayout: false,
                    },
                    UiInteractionResults::new(),
                );
            }

            if state.middle_pan_anchor.is_some() {
                let previous_hovered = state.hovered_widget;
                let previous_hovered_scrollbar = state.hovered_scrollbar;
                let raw_hover_target = hit_test_widget(tree, layouts, event.position);
                let hover_target =
                    raw_hover_target.filter(|widget_id| is_pointer_responsive(tree, *widget_id));
                state.hovered_widget = hover_target;
                state.hovered_scrollbar =
                    scrollbar_axis_target_at_position(tree, layouts, event.position);

                let mut interactions = UiInteractionResults::new();
                push_hover_change_if_needed(&mut interactions, previous_hovered, hover_target);

                let delta = middle_pan_delta(state, event.position, event.delta);
                state.middle_pan_last_position = Some(event.position);
                let owners = scroll_owners_for_pan(tree, raw_hover_target, state.middle_pan_anchor);
                let changed = apply_middle_pan_delta(tree, layouts, state, &owners, delta);

                return outcome(
                    state.middle_pan_anchor.or(hover_target),
                    InputResponse {
                        propagation: EventPropagation::Stop,
                        capture: PointerCapture::None,
                        focus_change: FocusChange::None,
                        repaint: changed || previous_hovered_scrollbar != state.hovered_scrollbar,
                        relayout: false,
                    },
                    interactions,
                );
            }

            let previous_hovered = state.hovered_widget;
            let previous_hovered_scrollbar = state.hovered_scrollbar;
            let target = target.filter(|widget_id| is_pointer_responsive(tree, *widget_id));
            state.hovered_widget = target;
            state.hovered_scrollbar =
                scrollbar_axis_target_at_position(tree, layouts, event.position);

            let mut interactions = UiInteractionResults::new();
            push_hover_change_if_needed(&mut interactions, previous_hovered, target);

            outcome(
                target,
                InputResponse {
                    repaint: previous_hovered_scrollbar != state.hovered_scrollbar,
                    ..InputResponse::ignored()
                },
                interactions,
            )
        }
        PointerEventKind::Leave => {
            if let Some(drag) = state.scrollbar_thumb_drag {
                return outcome(
                    Some(drag.scroll_widget),
                    InputResponse {
                        propagation: EventPropagation::Stop,
                        capture: PointerCapture::CaptureSelf,
                        focus_change: FocusChange::None,
                        repaint: false,
                        relayout: false,
                    },
                    UiInteractionResults::new(),
                );
            }

            let previous_hovered = state.hovered_widget;
            let previous_hovered_scrollbar = state.hovered_scrollbar;
            state.hovered_widget = None;
            state.hovered_scrollbar = None;

            let mut interactions = UiInteractionResults::new();
            push_hover_change_if_needed(&mut interactions, previous_hovered, None);

            outcome(
                None,
                InputResponse {
                    repaint: previous_hovered_scrollbar.is_some(),
                    ..InputResponse::ignored()
                },
                interactions,
            )
        }
        PointerEventKind::Down => {
            if matches!(
                event.button,
                Some(PointerButton::Primary | PointerButton::Secondary)
            ) && let Some(outcome) =
                dispatch_popup_stack_outside_dismiss(tree, layouts, state, event)
            {
                return outcome;
            }

            state.hovered_scrollbar =
                scrollbar_axis_target_at_position(tree, layouts, event.position);
            if event.button == Some(PointerButton::Primary)
                && let Some(geometry) =
                    scrollbar_thumb_geometry_at_position(tree, layouts, event.position)
            {
                let previous_hovered = state.hovered_widget;
                let previous_pressed = state.pressed_widget;
                let target = Some(geometry.scroll_widget_id);
                let pointer_grab_offset = axis_position(geometry.axis, event.position)
                    - axis_rect_start(geometry.axis, geometry.thumb_rect);
                state.hovered_widget = target;
                state.pressed_widget = target;
                state.captured_widget = target;
                state.middle_pan_anchor = None;
                state.middle_pan_last_position = None;
                state.scrollbar_thumb_drag = Some(ScrollbarThumbDragState {
                    scroll_widget: geometry.scroll_widget_id,
                    axis: geometry.axis,
                    pointer_grab_offset,
                });
                state.mark_scrollbar_active(geometry.scroll_widget_id, geometry.axis);
                let focus_change = FocusChange::Set(FocusTargetId(geometry.scroll_widget_id.0));
                state.focused_target = Some(FocusTargetId(geometry.scroll_widget_id.0));

                let mut interactions = UiInteractionResults::new();
                push_hover_change_if_needed(&mut interactions, previous_hovered, target);
                push_pressed_change_if_needed(&mut interactions, previous_pressed, target);
                push_focus_change_if_needed(&mut interactions, focus_change);

                return outcome(
                    target,
                    InputResponse {
                        propagation: EventPropagation::Stop,
                        capture: PointerCapture::CaptureSelf,
                        focus_change,
                        repaint: true,
                        relayout: false,
                    },
                    interactions,
                );
            }

            if event.button == Some(PointerButton::Primary)
                && let Some(graph_target) = target.filter(|widget_id| {
                    graph_canvas_node(tree, *widget_id)
                        .is_some_and(|canvas| canvas.capture_pointer_drag)
                })
            {
                return dispatch_graph_canvas_pointer_event(
                    tree,
                    layouts,
                    state,
                    graph_target,
                    event,
                );
            }

            if event.button == Some(PointerButton::Middle) {
                let previous_hovered = state.hovered_widget;
                let raw_target = hit_test_widget(tree, layouts, event.position);
                let hover_target =
                    raw_target.filter(|widget_id| is_pointer_responsive(tree, *widget_id));
                state.hovered_widget = hover_target;
                state.middle_pan_anchor = raw_target;
                state.middle_pan_last_position = Some(event.position);
                state.captured_widget = raw_target;

                let mut interactions = UiInteractionResults::new();
                push_hover_change_if_needed(&mut interactions, previous_hovered, hover_target);

                return outcome(
                    raw_target,
                    InputResponse {
                        propagation: if raw_target.is_some() {
                            EventPropagation::Stop
                        } else {
                            EventPropagation::Continue
                        },
                        capture: if raw_target.is_some() {
                            PointerCapture::CaptureSelf
                        } else {
                            PointerCapture::None
                        },
                        focus_change: FocusChange::None,
                        repaint: false,
                        relayout: false,
                    },
                    interactions,
                );
            }

            let previous_hovered = state.hovered_widget;
            let previous_pressed = state.pressed_widget;
            let target = target.filter(|widget_id| is_pointer_responsive(tree, *widget_id));

            state.hovered_widget = target;
            state.pressed_widget = target;
            state.captured_widget = target;
            state.middle_pan_anchor = None;
            state.middle_pan_last_position = None;
            state.scrollbar_thumb_drag = None;

            let focus_change = match target {
                Some(WidgetId(id)) => FocusChange::Set(FocusTargetId(id)),
                None => FocusChange::None,
            };

            if let FocusChange::Set(target_id) = focus_change {
                state.focused_target = Some(target_id);
            }

            let mut interactions = UiInteractionResults::new();
            push_hover_change_if_needed(&mut interactions, previous_hovered, target);
            push_pressed_change_if_needed(&mut interactions, previous_pressed, target);
            push_focus_change_if_needed(&mut interactions, focus_change);

            outcome(
                target,
                InputResponse {
                    propagation: if target.is_some() {
                        EventPropagation::Stop
                    } else {
                        EventPropagation::Continue
                    },
                    capture: if target.is_some() {
                        PointerCapture::CaptureSelf
                    } else {
                        PointerCapture::None
                    },
                    focus_change,
                    repaint: target.is_some(),
                    relayout: false,
                },
                interactions,
            )
        }
        PointerEventKind::Up => {
            if let Some(drag) = state.scrollbar_thumb_drag.take() {
                let previous_pressed = state.pressed_widget;
                let release_target = hit_test_widget(tree, layouts, event.position)
                    .filter(|widget_id| is_pointer_responsive(tree, *widget_id));
                state.hovered_widget = release_target;
                state.hovered_scrollbar =
                    scrollbar_axis_target_at_position(tree, layouts, event.position);
                state.pressed_widget = None;
                state.captured_widget = None;
                state.middle_pan_anchor = None;
                state.middle_pan_last_position = None;
                state.mark_scrollbar_active(drag.scroll_widget, drag.axis);

                let mut interactions = UiInteractionResults::new();
                push_pressed_change_if_needed(&mut interactions, previous_pressed, None);

                return outcome(
                    Some(drag.scroll_widget),
                    InputResponse {
                        propagation: EventPropagation::Stop,
                        capture: PointerCapture::Release,
                        focus_change: FocusChange::None,
                        repaint: true,
                        relayout: false,
                    },
                    interactions,
                );
            }

            if event.button == Some(PointerButton::Middle) && state.middle_pan_anchor.is_some() {
                let previous_hovered = state.hovered_widget;
                let previous_hovered_scrollbar = state.hovered_scrollbar;
                let release_target = hit_test_widget(tree, layouts, event.position)
                    .filter(|widget_id| is_pointer_responsive(tree, *widget_id));
                state.hovered_widget = release_target;
                state.hovered_scrollbar =
                    scrollbar_axis_target_at_position(tree, layouts, event.position);
                state.middle_pan_anchor = None;
                state.middle_pan_last_position = None;
                state.captured_widget = None;

                let mut interactions = UiInteractionResults::new();
                push_hover_change_if_needed(&mut interactions, previous_hovered, release_target);

                return outcome(
                    release_target,
                    InputResponse {
                        propagation: EventPropagation::Stop,
                        capture: PointerCapture::Release,
                        focus_change: FocusChange::None,
                        repaint: previous_hovered_scrollbar != state.hovered_scrollbar,
                        relayout: false,
                    },
                    interactions,
                );
            }

            let previous_hovered = state.hovered_widget;
            let previous_hovered_scrollbar = state.hovered_scrollbar;
            let previous_pressed = state.pressed_widget;
            let pressed_target = state.pressed_widget;
            let target = target.filter(|widget_id| is_pointer_responsive(tree, *widget_id));
            let release_target = hit_test_widget(tree, layouts, event.position)
                .filter(|widget_id| is_pointer_responsive(tree, *widget_id));

            state.hovered_widget = release_target;
            state.hovered_scrollbar =
                scrollbar_axis_target_at_position(tree, layouts, event.position);
            state.pressed_widget = None;
            state.captured_widget = None;
            state.middle_pan_anchor = None;
            state.middle_pan_last_position = None;
            state.scrollbar_thumb_drag = None;

            let mut interactions = UiInteractionResults::new();
            push_hover_change_if_needed(&mut interactions, previous_hovered, release_target);
            push_pressed_change_if_needed(&mut interactions, previous_pressed, None);

            if pressed_target.is_some()
                && let Some(interaction) = activation_for_release(
                    tree,
                    layouts,
                    pressed_target,
                    release_target,
                    event.position,
                )
            {
                interactions.push(interaction);
            }

            outcome(
                target,
                InputResponse {
                    propagation: if previous_pressed.is_some() {
                        EventPropagation::Stop
                    } else {
                        EventPropagation::Continue
                    },
                    capture: if previous_pressed.is_some() {
                        PointerCapture::Release
                    } else {
                        PointerCapture::None
                    },
                    focus_change: FocusChange::None,
                    repaint: previous_pressed.is_some()
                        || previous_hovered_scrollbar != state.hovered_scrollbar,
                    relayout: false,
                },
                interactions,
            )
        }
        PointerEventKind::Scroll => {
            let previous_hovered = state.hovered_widget;
            let previous_hovered_scrollbar = state.hovered_scrollbar;
            let raw_hover_target = hit_test_widget(tree, layouts, event.position);
            let hover_target =
                raw_hover_target.filter(|widget_id| is_pointer_responsive(tree, *widget_id));
            state.hovered_widget = hover_target;
            state.hovered_scrollbar =
                scrollbar_axis_target_at_position(tree, layouts, event.position);

            let mut interactions = UiInteractionResults::new();
            push_hover_change_if_needed(&mut interactions, previous_hovered, hover_target);

            if let Some(target_widget) = hover_target
                && let Some(next_value) = stepped_numeric_value(tree, target_widget, event.delta.y)
            {
                interactions.push(UiInteraction::NumericStepped {
                    target: target_widget,
                    value: next_value,
                });
                return outcome(
                    Some(target_widget),
                    InputResponse {
                        propagation: EventPropagation::Stop,
                        capture: PointerCapture::None,
                        focus_change: FocusChange::None,
                        repaint: true,
                        relayout: true,
                    },
                    interactions,
                );
            }

            let owners = raw_hover_target
                .map(|widget| find_scroll_owner_chain(tree, widget))
                .unwrap_or_default();
            let Some(ownership) = apply_scroll_wheel_delta(tree, layouts, state, &owners, event)
            else {
                return outcome(
                    target,
                    InputResponse {
                        repaint: previous_hovered_scrollbar != state.hovered_scrollbar,
                        ..InputResponse::ignored()
                    },
                    interactions,
                );
            };
            interactions.push(UiInteraction::ScrollInputOwned {
                owner: ownership.owner,
                axis: ownership.axis,
                changed: ownership.changed,
                at_boundary: ownership.at_boundary,
            });

            outcome(
                Some(ownership.owner),
                InputResponse {
                    propagation: EventPropagation::Stop,
                    capture: PointerCapture::None,
                    focus_change: FocusChange::None,
                    repaint: ownership.changed,
                    relayout: false,
                },
                interactions,
            )
        }
    }
}
