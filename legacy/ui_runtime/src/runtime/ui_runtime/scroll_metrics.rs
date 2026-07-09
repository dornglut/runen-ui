//! File: domain/ui/ui_runtime/src/runtime/ui_runtime/scroll_metrics.rs
//! Purpose: Runtime scroll metric helpers.

use ui_math::{Axis, UiRect};

use crate::{ComputedLayoutMap, UiNodeKind, UiTree, WidgetId};

use super::UiRuntime;

impl UiRuntime {
    pub fn max_scroll_offset(
        &self,
        tree: &UiTree,
        bounds: UiRect,
        scroll_widget: WidgetId,
    ) -> Option<f32> {
        self.max_scroll_offset_for_axis(tree, bounds, scroll_widget, Axis::Vertical)
    }
    pub fn max_scroll_offset_for_axis(
        &self,
        tree: &UiTree,
        bounds: UiRect,
        scroll_widget: WidgetId,
        axis: Axis,
    ) -> Option<f32> {
        let layouts = self.compute_layout(tree, bounds);
        self.max_scroll_offset_for_layout_axis(tree, &layouts, scroll_widget, axis)
    }
    pub fn max_scroll_offset_for_layout(
        &self,
        tree: &UiTree,
        layouts: &ComputedLayoutMap,
        scroll_widget: WidgetId,
    ) -> Option<f32> {
        self.max_scroll_offset_for_layout_axis(tree, layouts, scroll_widget, Axis::Vertical)
    }
    pub fn max_scroll_offset_for_layout_axis(
        &self,
        tree: &UiTree,
        layouts: &ComputedLayoutMap,
        scroll_widget: WidgetId,
        axis: Axis,
    ) -> Option<f32> {
        let scroll_layout = layouts.get(&scroll_widget)?;
        let scroll_node = tree.walk().find(|node| node.id == scroll_widget)?;
        let UiNodeKind::Scroll(scroll) = &scroll_node.kind else {
            return None;
        };
        if !scroll.axes.contains(axis) {
            return None;
        }
        let child_id = scroll_node.children.first()?.id;
        let child_layout = layouts.get(&child_id)?;
        match axis {
            Axis::Vertical => {
                let viewport_height = scroll_layout.content_bounds.height.max(0.0);
                let content_height = child_layout.bounds.height.max(viewport_height);
                Some((content_height - viewport_height).max(0.0))
            }
            Axis::Horizontal => {
                let viewport_width = scroll_layout.content_bounds.width.max(0.0);
                let content_width = child_layout.bounds.width.max(viewport_width);
                Some((content_width - viewport_width).max(0.0))
            }
        }
    }
}
