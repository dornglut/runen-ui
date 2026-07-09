//! File: domain/ui/ui_widgets/src/spacer.rs
//! Purpose: Spacer widget constructor.

use crate::{SpacerNode, UiNode, UiNodeKind, WidgetId};
use ui_math::UiSize;

pub fn spacer(id: WidgetId, min_size: UiSize) -> UiNode {
    UiNode::new(id, UiNodeKind::Spacer(SpacerNode::new(min_size)))
}
