//! File: domain/ui/ui_runtime/src/input/pointer/scroll.rs
//! Purpose: Wheel and owned scroll dispatch helpers.

use ui_input::PointerEvent;

use crate::{ComputedLayoutMap, ScrollInputPolicy, UiNodeKind, UiRuntimeState, UiTree, WidgetId};

use super::helpers::find_node;

const SCROLL_DELTA_CLAMP: f32 = 8.0;
const SCROLL_STEP_PX: f32 = 28.0;

pub(super) fn find_scroll_owner_chain(tree: &UiTree, target: WidgetId) -> Vec<WidgetId> {
    let mut chain_from_root = Vec::new();
    let mut out = Vec::new();
    let _ = find_scroll_owner_chain_inner(&tree.root, target, &mut chain_from_root, &mut out);
    out
}

fn find_scroll_owner_chain_inner(
    node: &crate::UiNode,
    target: WidgetId,
    chain_from_root: &mut Vec<WidgetId>,
    out: &mut Vec<WidgetId>,
) -> bool {
    let pushed = if matches!(node.kind, UiNodeKind::Scroll(_)) {
        chain_from_root.push(node.id);
        true
    } else {
        false
    };

    if node.id == target {
        out.extend(chain_from_root.iter().rev().copied());
        if pushed {
            chain_from_root.pop();
        }
        return true;
    }

    for child in &node.children {
        if find_scroll_owner_chain_inner(child, target, chain_from_root, out) {
            if pushed {
                chain_from_root.pop();
            }
            return true;
        }
    }

    if pushed {
        chain_from_root.pop();
    }

    false
}

fn scroll_max_offset(
    tree: &UiTree,
    layouts: &ComputedLayoutMap,
    scroll_widget: WidgetId,
    axis: ui_math::Axis,
) -> Option<f32> {
    let scroll_layout = layouts.get(&scroll_widget)?;
    let scroll_node = find_node(tree, scroll_widget)?;
    let UiNodeKind::Scroll(scroll) = &scroll_node.kind else {
        return None;
    };
    if !scroll.axes.contains(axis) {
        return None;
    }
    let child_id = scroll_node.children.first()?.id;
    let child_layout = layouts.get(&child_id)?;
    match axis {
        ui_math::Axis::Vertical => {
            let viewport_height = scroll_layout.content_bounds.height.max(0.0);
            let content_height = child_layout.bounds.height.max(viewport_height);
            Some((content_height - viewport_height).max(0.0))
        }
        ui_math::Axis::Horizontal => {
            let viewport_width = scroll_layout.content_bounds.width.max(0.0);
            let content_width = child_layout.bounds.width.max(viewport_width);
            Some((content_width - viewport_width).max(0.0))
        }
    }
}

fn scroll_primary_delta(
    tree: &UiTree,
    scroll_widget: WidgetId,
    axis: ui_math::Axis,
    event: &PointerEvent,
) -> Option<f32> {
    let scroll_node = find_node(tree, scroll_widget)?;
    let UiNodeKind::Scroll(scroll) = &scroll_node.kind else {
        return None;
    };
    if !scroll.axes.contains(axis) {
        return None;
    }

    match axis {
        ui_math::Axis::Vertical => {
            if event.delta.y.abs() > f32::EPSILON {
                Some(event.delta.y)
            } else if event.delta.x.abs() > f32::EPSILON
                && !scroll.axes.contains(ui_math::Axis::Horizontal)
            {
                Some(-event.delta.x)
            } else {
                None
            }
        }
        ui_math::Axis::Horizontal => {
            if event.delta.x.abs() > f32::EPSILON {
                Some(-event.delta.x)
            } else if event.delta.y.abs() > f32::EPSILON
                && !scroll.axes.contains(ui_math::Axis::Vertical)
            {
                Some(event.delta.y)
            } else {
                None
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub(super) struct ScrollWheelOwnership {
    pub(super) owner: WidgetId,
    pub(super) axis: ui_math::Axis,
    pub(super) changed: bool,
    pub(super) at_boundary: bool,
}

pub(super) fn apply_scroll_wheel_delta(
    tree: &UiTree,
    layouts: &ComputedLayoutMap,
    state: &mut UiRuntimeState,
    owners: &[WidgetId],
    event: &PointerEvent,
) -> Option<ScrollWheelOwnership> {
    for &owner in owners {
        let Some(node) = find_node(tree, owner) else {
            continue;
        };
        let UiNodeKind::Scroll(scroll) = &node.kind else {
            continue;
        };
        for axis in [ui_math::Axis::Vertical, ui_math::Axis::Horizontal] {
            let Some(raw_delta) = scroll_primary_delta(tree, owner, axis, event) else {
                continue;
            };
            if !matches!(
                scroll.input_policies.for_axis(axis),
                ScrollInputPolicy::WheelOnly | ScrollInputPolicy::WheelAndMiddleDrag
            ) {
                continue;
            }
            let max_offset = scroll_max_offset(tree, layouts, owner, axis).unwrap_or(0.0);
            if max_offset <= f32::EPSILON {
                continue;
            }
            let current_offset = state
                .scroll_offset_for_axis(owner, axis)
                .clamp(0.0, max_offset);
            let next_offset = (current_offset - scroll_pixels(raw_delta)).clamp(0.0, max_offset);
            let changed = (next_offset - current_offset).abs() > f32::EPSILON;
            if changed {
                state.set_scroll_offset_for_axis(owner, axis, next_offset);
                state.mark_scrollbar_active(owner, axis);
            }
            return Some(ScrollWheelOwnership {
                owner,
                axis,
                changed,
                at_boundary: !changed,
            });
        }
    }

    None
}

pub(super) fn apply_scroll_delta_for_axis(
    tree: &UiTree,
    layouts: &ComputedLayoutMap,
    state: &mut UiRuntimeState,
    owners: &[WidgetId],
    axis: ui_math::Axis,
    raw_delta: f32,
) -> bool {
    for &owner in owners {
        let Some(node) = find_node(tree, owner) else {
            continue;
        };
        let UiNodeKind::Scroll(scroll) = &node.kind else {
            continue;
        };
        if !scroll.axes.contains(axis) {
            continue;
        }
        if !matches!(
            scroll.input_policies.for_axis(axis),
            ScrollInputPolicy::MiddleDragOnly | ScrollInputPolicy::WheelAndMiddleDrag
        ) {
            continue;
        }
        let max_offset = scroll_max_offset(tree, layouts, owner, axis).unwrap_or(0.0);
        if max_offset <= f32::EPSILON {
            continue;
        }
        let current_offset = state
            .scroll_offset_for_axis(owner, axis)
            .clamp(0.0, max_offset);
        let next_offset = (current_offset - raw_delta).clamp(0.0, max_offset);
        if (next_offset - current_offset).abs() <= f32::EPSILON {
            continue;
        }
        state.set_scroll_offset_for_axis(owner, axis, next_offset);
        state.mark_scrollbar_active(owner, axis);
        return true;
    }
    false
}

fn scroll_pixels(raw_delta: f32) -> f32 {
    raw_delta.clamp(-SCROLL_DELTA_CLAMP, SCROLL_DELTA_CLAMP) * SCROLL_STEP_PX
}
