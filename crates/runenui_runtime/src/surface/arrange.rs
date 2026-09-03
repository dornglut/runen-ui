use super::{
    measure::{
        MeasuredSurfaceLayout, child_layout_axis_without_diagnostic, finite_sum, resolved_padding,
    },
    resolve::{ResolvedSurfaceNode, ResolvedSurfaceTree},
};
use crate::LogicalPoint;
use runenui_core::{Axis, LogicalRect};

pub(super) struct SurfaceArrangementBuilder<'a> {
    measured_layout: &'a MeasuredSurfaceLayout,
    nodes: Vec<LogicalRect>,
}

impl<'a> SurfaceArrangementBuilder<'a> {
    pub(super) const fn new(measured_layout: &'a MeasuredSurfaceLayout) -> Self {
        Self {
            measured_layout,
            nodes: Vec::new(),
        }
    }

    pub(super) fn into_nodes(self) -> Vec<LogicalRect> {
        self.nodes
    }

    pub(super) fn push_node(
        &mut self,
        resolved_tree: &ResolvedSurfaceTree,
        node: &ResolvedSurfaceNode,
        origin: LogicalPoint,
    ) {
        let measured = self.measured_layout.node(node.position);
        let bounds = LogicalRect::new(origin, measured.constrained_outer_size());
        self.nodes.push(bounds);

        if let Some(child_layout) = node.child_layout() {
            self.push_child_layout_children(
                resolved_tree,
                node,
                bounds,
                child_layout_axis_without_diagnostic(child_layout),
            );
        }
    }

    fn push_child_layout_children(
        &mut self,
        resolved_tree: &ResolvedSurfaceTree,
        container_node: &ResolvedSurfaceNode,
        parent_bounds: LogicalRect,
        axis: Axis,
    ) {
        let gap = container_node.layout().gap().along(axis).get();
        let padding = resolved_padding(container_node);
        let mut cursor_x = finite_sum(parent_bounds.x(), padding.left().get());
        let mut cursor_y = finite_sum(parent_bounds.y(), padding.top().get());
        for (arranged_child_count, child_id) in container_node.children().iter().enumerate() {
            let child = resolved_tree.node(child_id);
            let measured_child = self.measured_layout.node(child.position);
            let child_size = measured_child.constrained_outer_size();

            if arranged_child_count > 0 {
                match axis {
                    Axis::Vertical => cursor_y = finite_sum(cursor_y, gap),
                    Axis::Horizontal => cursor_x = finite_sum(cursor_x, gap),
                }
            }

            let origin = LogicalPoint::new(cursor_x, cursor_y)
                .unwrap_or_else(|_| unreachable!("surface arrangement coordinates remain finite"));
            self.push_node(resolved_tree, child, origin);

            match axis {
                Axis::Vertical => cursor_y = finite_sum(cursor_y, child_size.height()),
                Axis::Horizontal => cursor_x = finite_sum(cursor_x, child_size.width()),
            }
        }
    }
}
