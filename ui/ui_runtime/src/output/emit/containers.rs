//! File: domain/ui/ui_runtime/src/output/emit/containers.rs
//! Purpose: Container, clip, popup, radial-menu, and scrollbar emission for UI frame output.

use crate::{
    ComputedLayoutMap, PanelNode, PopupNode, RadialMenuNode, ScrollNode, ScrollbarAxisTarget,
    UiNode, UiNodeKind, UiTree, WidgetId,
};
use ui_math::{Axis, UiRect};
use ui_render_data::{BorderPrimitive, ClipPrimitive, RectPrimitive, UiLayer, UiPrimitive};

use super::super::build_ui_frame::InteractionVisualState;
use super::super::primitives::{default_draw_key, paint_from_color, sort_key};

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct ScrollbarGeometry {
    pub scroll_widget_id: WidgetId,
    pub axis: ui_math::Axis,
    pub track_rect: UiRect,
    pub thumb_rect: UiRect,
    pub max_offset: f32,
}

pub(crate) fn emit_panel(
    panel: &PanelNode,
    bounds: UiRect,
    content_bounds: UiRect,
    layer: &mut UiLayer,
    depth: u32,
    primitive_order: &mut u32,
) {
    layer.push(UiPrimitive::Rect(RectPrimitive::new(
        bounds,
        panel.theme.radius.md,
        paint_from_color(panel.theme.background_panel),
        default_draw_key(),
        sort_key(depth, *primitive_order),
    )));
    *primitive_order += 1;

    layer.push(UiPrimitive::Border(BorderPrimitive::new(
        bounds,
        panel.theme.radius.md,
        panel.theme.border_width,
        paint_from_color(panel.theme.border),
        default_draw_key(),
        sort_key(depth, *primitive_order),
    )));
    *primitive_order += 1;

    layer.push(UiPrimitive::Clip(ClipPrimitive::Push {
        rect: content_bounds,
        sort_key: sort_key(depth, *primitive_order),
    }));
    *primitive_order += 1;
}

pub(crate) fn emit_popup(
    popup: &PopupNode,
    bounds: UiRect,
    content_bounds: UiRect,
    layer: &mut UiLayer,
    depth: u32,
    primitive_order: &mut u32,
) {
    layer.push(UiPrimitive::Rect(RectPrimitive::new(
        bounds,
        popup.theme.radius.md,
        paint_from_color(popup.theme.background_panel),
        default_draw_key(),
        sort_key(depth, *primitive_order),
    )));
    *primitive_order += 1;

    layer.push(UiPrimitive::Border(BorderPrimitive::new(
        bounds,
        popup.theme.radius.md,
        popup.theme.border_width,
        paint_from_color(popup.theme.border),
        default_draw_key(),
        sort_key(depth, *primitive_order),
    )));
    *primitive_order += 1;

    layer.push(UiPrimitive::Clip(ClipPrimitive::Push {
        rect: content_bounds,
        sort_key: sort_key(depth, *primitive_order),
    }));
    *primitive_order += 1;
}

pub(crate) fn emit_radial_menu(
    radial: &RadialMenuNode,
    bounds: UiRect,
    content_bounds: UiRect,
    layer: &mut UiLayer,
    depth: u32,
    primitive_order: &mut u32,
) {
    layer.push(UiPrimitive::Rect(RectPrimitive::new(
        bounds,
        radial.outer_radius,
        paint_from_color(radial.theme.background_panel),
        default_draw_key(),
        sort_key(depth, *primitive_order),
    )));
    *primitive_order += 1;

    layer.push(UiPrimitive::Border(BorderPrimitive::new(
        bounds,
        radial.outer_radius,
        radial.theme.border_width,
        paint_from_color(radial.theme.border),
        default_draw_key(),
        sort_key(depth, *primitive_order),
    )));
    *primitive_order += 1;

    layer.push(UiPrimitive::Clip(ClipPrimitive::Push {
        rect: content_bounds,
        sort_key: sort_key(depth, *primitive_order),
    }));
    *primitive_order += 1;
}

pub(crate) fn emit_scroll_begin(
    _scroll: &ScrollNode,
    _bounds: UiRect,
    content_bounds: UiRect,
    layer: &mut UiLayer,
    depth: u32,
    primitive_order: &mut u32,
) {
    layer.push(UiPrimitive::Clip(ClipPrimitive::Push {
        rect: content_bounds,
        sort_key: sort_key(depth, *primitive_order),
    }));
    *primitive_order += 1;
}

#[expect(
    clippy::too_many_arguments,
    reason = "private render-emission helpers keep explicit render state at each emission boundary"
)]
pub(crate) fn emit_scrollbar(
    tree: &UiTree,
    node: &UiNode,
    _scroll: &ScrollNode,
    layouts: &ComputedLayoutMap,
    bounds: UiRect,
    content_bounds: UiRect,
    layer: &mut UiLayer,
    interaction_state: InteractionVisualState,
    depth: u32,
    primitive_order: &mut u32,
) {
    let geometries = scrollbar_geometries(tree, node.id, layouts, bounds, content_bounds);
    if geometries.is_empty() {
        return;
    }
    let UiNodeKind::Scroll(scroll) = &node.kind else {
        return;
    };

    for geometry in geometries {
        let target = ScrollbarAxisTarget::new(node.id, geometry.axis);
        let scrollbar_opacity = if interaction_state.active_scrollbar == Some(target)
            || interaction_state.hovered_scrollbar == Some(target)
        {
            1.0
        } else {
            interaction_state
                .scrollbar_opacity_by_widget_id
                .get(&node.id)
                .map(|opacities| opacities.for_axis(geometry.axis))
                .unwrap_or(0.0)
                .clamp(0.0, 1.0)
        };
        if scrollbar_opacity <= 0.0 {
            continue;
        }
        let radius = match geometry.axis {
            Axis::Vertical => scroll.theme.radius.sm.min(geometry.track_rect.width * 0.5),
            Axis::Horizontal => scroll.theme.radius.sm.min(geometry.track_rect.height * 0.5),
        };

        let mut track_color = scroll.theme.border;
        track_color.a = (track_color.a * 0.35 * scrollbar_opacity).clamp(0.0, 1.0);
        layer.push(UiPrimitive::Rect(RectPrimitive::new(
            geometry.track_rect,
            radius,
            paint_from_color(track_color),
            default_draw_key(),
            sort_key(depth, *primitive_order),
        )));
        *primitive_order += 1;

        let mut thumb_color = scroll.theme.accent;
        thumb_color.a = (thumb_color.a * 0.80 * scrollbar_opacity).clamp(0.0, 1.0);
        layer.push(UiPrimitive::Rect(RectPrimitive::new(
            geometry.thumb_rect,
            radius,
            paint_from_color(thumb_color),
            default_draw_key(),
            sort_key(depth, *primitive_order),
        )));
        *primitive_order += 1;
    }
}

#[cfg(test)]
pub(crate) fn scrollbar_geometry(
    tree: &UiTree,
    scroll_widget_id: WidgetId,
    layouts: &ComputedLayoutMap,
    bounds: UiRect,
    content_bounds: UiRect,
) -> Option<ScrollbarGeometry> {
    scrollbar_geometry_for_axis(
        tree,
        scroll_widget_id,
        layouts,
        bounds,
        content_bounds,
        Axis::Vertical,
    )
    .or_else(|| {
        scrollbar_geometry_for_axis(
            tree,
            scroll_widget_id,
            layouts,
            bounds,
            content_bounds,
            Axis::Horizontal,
        )
    })
}

pub(crate) fn scrollbar_geometries(
    tree: &UiTree,
    scroll_widget_id: WidgetId,
    layouts: &ComputedLayoutMap,
    bounds: UiRect,
    content_bounds: UiRect,
) -> Vec<ScrollbarGeometry> {
    [Axis::Vertical, Axis::Horizontal]
        .into_iter()
        .filter_map(|axis| {
            scrollbar_geometry_for_axis(
                tree,
                scroll_widget_id,
                layouts,
                bounds,
                content_bounds,
                axis,
            )
        })
        .collect()
}

pub(crate) fn scrollbar_geometry_for_axis(
    tree: &UiTree,
    scroll_widget_id: WidgetId,
    layouts: &ComputedLayoutMap,
    bounds: UiRect,
    content_bounds: UiRect,
    axis: Axis,
) -> Option<ScrollbarGeometry> {
    let node = tree.walk().find(|node| node.id == scroll_widget_id)?;
    let UiNodeKind::Scroll(scroll) = &node.kind else {
        return None;
    };
    if !scroll.axes.contains(axis) {
        return None;
    }
    let child = node.children.first()?;
    let child_layout = layouts.get(&child.id)?;
    let vertical_max_offset =
        scroll_max_offset_for_axis(scroll, child_layout.bounds, content_bounds, Axis::Vertical);
    let horizontal_max_offset = scroll_max_offset_for_axis(
        scroll,
        child_layout.bounds,
        content_bounds,
        Axis::Horizontal,
    );
    match axis {
        Axis::Vertical => {
            let track_width = scroll.bar_thickness.min(bounds.width.max(0.0));
            if track_width <= f32::EPSILON || content_bounds.height <= f32::EPSILON {
                return None;
            }
            let horizontal_track_height = if horizontal_max_offset > f32::EPSILON {
                scroll.bar_thickness.min(bounds.height.max(0.0))
            } else {
                0.0
            };
            let track_x = bounds.x + bounds.width - track_width;
            let track_height = (content_bounds.height - horizontal_track_height).max(0.0);
            if track_height <= f32::EPSILON {
                return None;
            }
            let track_rect = UiRect::new(track_x, content_bounds.y, track_width, track_height);
            let viewport_extent = content_bounds.height.max(0.0);
            let content_extent = child_layout.bounds.height.max(viewport_extent);
            let max_offset = vertical_max_offset;
            if max_offset <= f32::EPSILON {
                return None;
            }
            let scroll_offset = (content_bounds.y - child_layout.bounds.y).clamp(0.0, max_offset);
            let min_thumb = scroll.min_thumb_main_size.min(track_rect.height).max(0.0);
            let natural = (viewport_extent / content_extent) * track_rect.height;
            let thumb_extent = natural.clamp(min_thumb, track_rect.height);
            let thumb_range = (track_rect.height - thumb_extent).max(0.0);
            let thumb_y = track_rect.y + thumb_range * (scroll_offset / max_offset);
            Some(ScrollbarGeometry {
                scroll_widget_id,
                axis,
                track_rect,
                thumb_rect: UiRect::new(track_rect.x, thumb_y, track_rect.width, thumb_extent),
                max_offset,
            })
        }
        Axis::Horizontal => {
            let track_height = scroll.bar_thickness.min(bounds.height.max(0.0));
            if track_height <= f32::EPSILON || content_bounds.width <= f32::EPSILON {
                return None;
            }
            let vertical_track_width = if vertical_max_offset > f32::EPSILON {
                scroll.bar_thickness.min(bounds.width.max(0.0))
            } else {
                0.0
            };
            let track_width = (content_bounds.width - vertical_track_width).max(0.0);
            if track_width <= f32::EPSILON {
                return None;
            }
            let track_rect = UiRect::new(
                content_bounds.x,
                bounds.y + bounds.height - track_height,
                track_width,
                track_height,
            );
            let viewport_extent = content_bounds.width.max(0.0);
            let content_extent = child_layout.bounds.width.max(viewport_extent);
            let max_offset = horizontal_max_offset;
            if max_offset <= f32::EPSILON {
                return None;
            }
            let scroll_offset = (content_bounds.x - child_layout.bounds.x).clamp(0.0, max_offset);
            let min_thumb = scroll.min_thumb_main_size.min(track_rect.width).max(0.0);
            let natural = (viewport_extent / content_extent) * track_rect.width;
            let thumb_extent = natural.clamp(min_thumb, track_rect.width);
            let thumb_range = (track_rect.width - thumb_extent).max(0.0);
            let thumb_x = track_rect.x + thumb_range * (scroll_offset / max_offset);
            Some(ScrollbarGeometry {
                scroll_widget_id,
                axis,
                track_rect,
                thumb_rect: UiRect::new(thumb_x, track_rect.y, thumb_extent, track_rect.height),
                max_offset,
            })
        }
    }
}

fn scroll_max_offset_for_axis(
    scroll: &ScrollNode,
    child_bounds: UiRect,
    content_bounds: UiRect,
    axis: Axis,
) -> f32 {
    if !scroll.axes.contains(axis) {
        return 0.0;
    }
    match axis {
        Axis::Vertical => {
            let viewport_height = content_bounds.height.max(0.0);
            let content_height = child_bounds.height.max(viewport_height);
            (content_height - viewport_height).max(0.0)
        }
        Axis::Horizontal => {
            let viewport_width = content_bounds.width.max(0.0);
            let content_width = child_bounds.width.max(viewport_width);
            (content_width - viewport_width).max(0.0)
        }
    }
}
