//! File: domain/ui/ui_runtime/src/widgets/stack.rs
//! Purpose: Stack widget constructors.

use crate::{StackNode, UiNode, UiNodeKind, WidgetId};
use ui_layout::SizePolicy;
use ui_math::Axis;

pub fn vstack(id: WidgetId, gap: f32, children: Vec<UiNode>) -> UiNode {
    UiNode::with_children(id, UiNodeKind::Stack(StackNode::vertical(gap)), children)
}

pub fn hstack(id: WidgetId, gap: f32, children: Vec<UiNode>) -> UiNode {
    UiNode::with_children(id, UiNodeKind::Stack(StackNode::horizontal(gap)), children)
}

pub fn vstack_with_policies(
    id: WidgetId,
    gap: f32,
    child_main_policies: Vec<SizePolicy>,
    children: Vec<UiNode>,
) -> UiNode {
    let mut node = StackNode::vertical(gap);
    node.child_main_policies = child_main_policies;
    UiNode::with_children(id, UiNodeKind::Stack(node), children)
}

pub fn hstack_with_policies(
    id: WidgetId,
    gap: f32,
    child_main_policies: Vec<SizePolicy>,
    children: Vec<UiNode>,
) -> UiNode {
    let mut node = StackNode::horizontal(gap);
    node.child_main_policies = child_main_policies;
    UiNode::with_children(id, UiNodeKind::Stack(node), children)
}

pub fn stack(id: WidgetId, axis: Axis, gap: f32, children: Vec<UiNode>) -> UiNode {
    let node = match axis {
        Axis::Horizontal => StackNode::horizontal(gap),
        Axis::Vertical => StackNode::vertical(gap),
    };

    UiNode::with_children(id, UiNodeKind::Stack(node), children)
}
