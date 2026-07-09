//! File: domain/ui/ui_runtime/src/input/pointer/scrollbar.rs
//! Purpose: Scrollbar pointer hit testing and thumb dragging.

use crate::{
    ComputedLayoutMap, UiRuntimeState, UiTree,
    output::build_ui_frame::{
        ScrollbarGeometry, scrollbar_geometries, scrollbar_geometry_for_axis,
    },
    state::{ScrollbarAxisTarget, ScrollbarThumbDragState},
};

pub(super) fn scrollbar_thumb_geometry_at_position(
    tree: &UiTree,
    layouts: &ComputedLayoutMap,
    position: ui_math::UiPoint,
) -> Option<ScrollbarGeometry> {
    let mut hit = None;
    for node in tree.walk() {
        let Some(layout) = layouts.get(&node.id) else {
            continue;
        };
        for geometry in
            scrollbar_geometries(tree, node.id, layouts, layout.bounds, layout.content_bounds)
        {
            if geometry.thumb_rect.contains(position) {
                hit = Some(geometry);
            }
        }
    }
    hit
}

pub(super) fn scrollbar_axis_target_at_position(
    tree: &UiTree,
    layouts: &ComputedLayoutMap,
    position: ui_math::UiPoint,
) -> Option<ScrollbarAxisTarget> {
    let mut hit = None;
    for node in tree.walk() {
        let Some(layout) = layouts.get(&node.id) else {
            continue;
        };
        for geometry in
            scrollbar_geometries(tree, node.id, layouts, layout.bounds, layout.content_bounds)
        {
            if geometry.track_rect.contains(position) || geometry.thumb_rect.contains(position) {
                hit = Some(ScrollbarAxisTarget::new(
                    geometry.scroll_widget_id,
                    geometry.axis,
                ));
            }
        }
    }
    hit
}

pub(super) fn apply_scrollbar_thumb_drag(
    tree: &UiTree,
    layouts: &ComputedLayoutMap,
    state: &mut UiRuntimeState,
    drag: ScrollbarThumbDragState,
    position: ui_math::UiPoint,
) -> bool {
    let Some(layout) = layouts.get(&drag.scroll_widget) else {
        state.scrollbar_thumb_drag = None;
        state.captured_widget = None;
        state.pressed_widget = None;
        return false;
    };
    let Some(geometry) = scrollbar_geometry_for_axis(
        tree,
        drag.scroll_widget,
        layouts,
        layout.bounds,
        layout.content_bounds,
        drag.axis,
    ) else {
        state.scrollbar_thumb_drag = None;
        state.captured_widget = None;
        state.pressed_widget = None;
        return false;
    };
    let thumb_extent = axis_rect_extent(geometry.axis, geometry.thumb_rect);
    let track_extent = axis_rect_extent(geometry.axis, geometry.track_rect);
    let thumb_range = (track_extent - thumb_extent).max(0.0);
    if thumb_range <= f32::EPSILON || geometry.max_offset <= f32::EPSILON {
        return false;
    }

    let pointer_main = axis_position(geometry.axis, position);
    let track_start = axis_rect_start(geometry.axis, geometry.track_rect);
    let thumb_start =
        (pointer_main - track_start - drag.pointer_grab_offset).clamp(0.0, thumb_range);
    let next_offset =
        ((thumb_start / thumb_range) * geometry.max_offset).clamp(0.0, geometry.max_offset);
    let current_offset = state
        .scroll_offset_for_axis(drag.scroll_widget, drag.axis)
        .clamp(0.0, geometry.max_offset);
    if (next_offset - current_offset).abs() <= f32::EPSILON {
        return false;
    }
    state.set_scroll_offset_for_axis(drag.scroll_widget, drag.axis, next_offset);
    state.mark_scrollbar_active(drag.scroll_widget, drag.axis);
    true
}

pub(super) fn axis_position(axis: ui_math::Axis, position: ui_math::UiPoint) -> f32 {
    match axis {
        ui_math::Axis::Horizontal => position.x,
        ui_math::Axis::Vertical => position.y,
    }
}

pub(super) fn axis_rect_start(axis: ui_math::Axis, rect: ui_math::UiRect) -> f32 {
    match axis {
        ui_math::Axis::Horizontal => rect.x,
        ui_math::Axis::Vertical => rect.y,
    }
}

fn axis_rect_extent(axis: ui_math::Axis, rect: ui_math::UiRect) -> f32 {
    match axis {
        ui_math::Axis::Horizontal => rect.width,
        ui_math::Axis::Vertical => rect.height,
    }
}
