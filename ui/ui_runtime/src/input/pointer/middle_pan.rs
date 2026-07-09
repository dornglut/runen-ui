//! File: domain/ui/ui_runtime/src/input/pointer/middle_pan.rs
//! Purpose: Middle-button pan ownership and scroll application.

use crate::{ComputedLayoutMap, UiRuntimeState, UiTree, WidgetId};

use super::scroll::{apply_scroll_delta_for_axis, find_scroll_owner_chain};

const PAN_SCROLL_SPEED: f32 = 1.5;

pub(super) fn scroll_owners_for_pan(
    tree: &UiTree,
    _raw_hover_target: Option<WidgetId>,
    pan_anchor: Option<WidgetId>,
) -> Vec<WidgetId> {
    // Keep middle-drag scrolling sticky to the anchor where drag started so
    // panning remains continuous even when pointer crosses other UI regions.
    // If the anchor is not scroll-owned, this drag belongs to another owner
    // such as the viewport bridge and must not adopt a hovered scroll later.
    if let Some(anchor) = pan_anchor {
        return find_scroll_owner_chain(tree, anchor);
    }
    Vec::new()
}

pub(super) fn middle_pan_delta(
    state: &UiRuntimeState,
    position: ui_math::UiPoint,
    event_delta: ui_math::UiVector,
) -> ui_math::UiVector {
    if event_delta.x.abs() > f32::EPSILON || event_delta.y.abs() > f32::EPSILON {
        event_delta
    } else if let Some(last) = state.middle_pan_last_position {
        position - last
    } else {
        ui_math::UiVector::ZERO
    }
}

pub(super) fn apply_middle_pan_delta(
    tree: &UiTree,
    layouts: &ComputedLayoutMap,
    state: &mut UiRuntimeState,
    owners: &[WidgetId],
    delta: ui_math::UiVector,
) -> bool {
    let mut changed = false;
    if delta.x.abs() > f32::EPSILON {
        changed |= apply_scroll_delta_for_axis(
            tree,
            layouts,
            state,
            owners,
            ui_math::Axis::Horizontal,
            delta.x * PAN_SCROLL_SPEED,
        );
    }
    if delta.y.abs() > f32::EPSILON {
        changed |= apply_scroll_delta_for_axis(
            tree,
            layouts,
            state,
            owners,
            ui_math::Axis::Vertical,
            delta.y * PAN_SCROLL_SPEED,
        );
    }
    changed
}
