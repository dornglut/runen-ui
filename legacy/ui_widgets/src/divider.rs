//! File: domain/ui/ui_widgets/src/divider.rs
//! Purpose: Divider widget constructors.

use crate::{DividerNode, UiNode, UiNodeKind, WidgetId};
use ui_layout::SizePolicy;
use ui_math::Axis;
use ui_theme::UiColor;

pub fn divider(
    id: WidgetId,
    axis: Axis,
    thickness: f32,
    length_policy: SizePolicy,
    color: UiColor,
) -> UiNode {
    UiNode::new(
        id,
        UiNodeKind::Divider(DividerNode::new(axis, thickness, length_policy, color)),
    )
}

pub fn hdivider(id: WidgetId, thickness: f32, length_policy: SizePolicy, color: UiColor) -> UiNode {
    divider(id, Axis::Horizontal, thickness, length_policy, color)
}

pub fn vdivider(id: WidgetId, thickness: f32, length_policy: SizePolicy, color: UiColor) -> UiNode {
    divider(id, Axis::Vertical, thickness, length_policy, color)
}
