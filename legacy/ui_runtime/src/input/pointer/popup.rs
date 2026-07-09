//! File: domain/ui/ui_runtime/src/input/pointer/popup.rs
//! Purpose: Popup pointer dismissal handling.

use ui_input::{
    EventPropagation, FocusChange, FocusTargetId, InputResponse, PointerCapture, PointerEvent,
};

use crate::{
    ComputedLayoutMap, PopupDismissPolicy, UiInputOutcome, UiInteraction, UiInteractionResults,
    UiNodeKind, UiRuntimeState, UiTree, WidgetId,
};

use super::helpers::{find_node, outcome, push_focus_change_if_needed};

#[derive(Debug, Clone, Copy)]
struct PopupStackScope {
    popup: WidgetId,
    anchor: WidgetId,
    layer_order: u32,
    tree_order: usize,
}

pub(super) fn dispatch_popup_stack_outside_dismiss(
    tree: &UiTree,
    layouts: &ComputedLayoutMap,
    state: &mut UiRuntimeState,
    event: &PointerEvent,
) -> Option<UiInputOutcome> {
    let scopes = popup_stack_scopes(tree, layouts);
    let top = scopes
        .iter()
        .max_by_key(|scope| (scope.layer_order, scope.tree_order))
        .copied()?;
    if popup_stack_contains_point(&scopes, layouts, event.position) {
        return None;
    }

    let focus_return = find_node(tree, top.anchor).map(|node| node.id);
    let focus_change = match focus_return {
        Some(widget_id) => {
            let target = FocusTargetId(widget_id.0);
            if state.focused_target == Some(target) {
                FocusChange::None
            } else {
                state.focused_target = Some(target);
                FocusChange::Set(target)
            }
        }
        None if state.focused_target.is_some() => {
            state.focused_target = None;
            FocusChange::Clear
        }
        None => FocusChange::None,
    };
    state.pressed_widget = None;
    state.captured_widget = None;
    state.middle_pan_anchor = None;
    state.middle_pan_last_position = None;
    state.scrollbar_thumb_drag = None;

    let mut interactions = UiInteractionResults::new();
    interactions.push(UiInteraction::PopupDismissRequested {
        popup: top.popup,
        focus_return,
    });
    push_focus_change_if_needed(&mut interactions, focus_change);

    Some(outcome(
        Some(top.popup),
        InputResponse {
            propagation: EventPropagation::Stop,
            capture: PointerCapture::None,
            focus_change,
            repaint: true,
            relayout: false,
        },
        interactions,
    ))
}

fn popup_stack_scopes(tree: &UiTree, layouts: &ComputedLayoutMap) -> Vec<PopupStackScope> {
    tree.walk()
        .enumerate()
        .filter_map(|(tree_order, node)| {
            let UiNodeKind::Popup(popup) = &node.kind else {
                return None;
            };
            if !matches!(popup.dismiss_policy, PopupDismissPolicy::OutsidePointerDown) {
                return None;
            }
            layouts.contains_key(&node.id).then_some(PopupStackScope {
                popup: node.id,
                anchor: popup.anchor,
                layer_order: popup.layer_order,
                tree_order,
            })
        })
        .collect()
}

fn popup_stack_contains_point(
    scopes: &[PopupStackScope],
    layouts: &ComputedLayoutMap,
    point: ui_math::UiPoint,
) -> bool {
    scopes.iter().any(|scope| {
        layouts
            .get(&scope.popup)
            .is_some_and(|layout| layout.bounds.contains(point))
            || layouts
                .get(&scope.anchor)
                .is_some_and(|layout| layout.bounds.contains(point))
    })
}
