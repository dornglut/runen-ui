//! File: domain/ui/ui_runtime/src/layout/engine/scroll.rs
//! Purpose: Scroll viewport layout and offset clamping for retained UI nodes.

use ui_math::{Axis, UiRect, UiSize};

use crate::{ComputedLayout, ComputedLayoutMap, ScrollNode, UiNode, UiRuntimeState};

use super::dispatch::layout_node;
use super::measure::measure_node;

pub(super) fn layout_scroll(
    node: &UiNode,
    scroll: &ScrollNode,
    bounds: UiRect,
    state: &UiRuntimeState,
    out: &mut ComputedLayoutMap,
) -> UiSize {
    let base_content_bounds = UiRect::new(
        bounds.x,
        bounds.y,
        bounds.width.max(0.0),
        bounds.height.max(0.0),
    );
    let measured_content = node
        .children
        .first()
        .map(measure_node)
        .unwrap_or(UiSize::ZERO);
    let content_bounds = base_content_bounds;

    if let Some(child) = node.children.first() {
        let content_width = if scroll.axes.contains(Axis::Horizontal) {
            measured_content.width.max(content_bounds.width)
        } else {
            content_bounds.width
        };
        let content_height = if scroll.axes.contains(Axis::Vertical) {
            measured_content.height.max(content_bounds.height)
        } else {
            content_bounds.height
        };
        let max_x = (content_width - content_bounds.width).max(0.0);
        let max_y = (content_height - content_bounds.height).max(0.0);
        let offset_x = if scroll.axes.contains(Axis::Horizontal) {
            state
                .scroll_offset_for_axis(node.id, Axis::Horizontal)
                .clamp(0.0, max_x)
        } else {
            0.0
        };
        let offset_y = if scroll.axes.contains(Axis::Vertical) {
            state
                .scroll_offset_for_axis(node.id, Axis::Vertical)
                .clamp(0.0, max_y)
        } else {
            0.0
        };
        let child_bounds = UiRect::new(
            content_bounds.x - offset_x,
            content_bounds.y - offset_y,
            content_width,
            content_height,
        );
        layout_node(child, child_bounds, state, out);
    }

    // Preserve the child's unconstrained content extent in measured_size so ancestor
    // scroll containers can still detect overflow through nested scroll nodes.
    let measured_size = UiSize::new(
        measured_content.width.max(content_bounds.width),
        measured_content.height.max(content_bounds.height),
    );
    out.insert(
        node.id,
        ComputedLayout::new(bounds, content_bounds, measured_size),
    );
    measured_size
}
