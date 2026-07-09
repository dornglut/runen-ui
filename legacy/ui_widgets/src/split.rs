//! File: domain/ui/ui_runtime/src/widgets/split.rs
//! Purpose: Split widget constructor.

use crate::{SplitNode, UiNode, UiNodeKind, WidgetId};
use ui_math::Axis;

pub fn split(id: WidgetId, axis: Axis, ratio: f32, gap: f32, children: Vec<UiNode>) -> UiNode {
    UiNode::with_children(
        id,
        UiNodeKind::Split(SplitNode::new(axis, ratio, gap)),
        children,
    )
}
