//! File: domain/ui/ui_tree/src/lib.rs
//! Crate: ui_tree
//! Purpose: Retained UI tree contracts and shared layout-record primitives.

pub mod computed_layout;
pub mod ids;
pub mod inspection;
pub mod tree;

pub use computed_layout::{ComputedLayout, ComputedLayoutMap};
pub use ids::*;
pub use inspection::*;
pub use tree::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tree_walk_is_preorder() {
        let root = UiNode::with_children(
            WidgetId(1),
            UiNodeKind::Split(SplitNode::new(ui_math::Axis::Horizontal, 0.5, 8.0)),
            vec![
                UiNode::new(WidgetId(2), UiNodeKind::Stack(StackNode::vertical(4.0))),
                UiNode::new(WidgetId(3), UiNodeKind::Stack(StackNode::vertical(4.0))),
            ],
        );
        let tree = UiTree::new(root);
        let order = tree.walk().map(|node| node.id).collect::<Vec<_>>();
        assert_eq!(order, vec![WidgetId(1), WidgetId(2), WidgetId(3)]);
    }
}
