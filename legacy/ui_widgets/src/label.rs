//! File: domain/ui/ui_runtime/src/widgets/label.rs
//! Purpose: Label widget constructor.

use crate::{LabelNode, UiNode, UiNodeKind, WidgetId};
use ui_text::TextStyle;

pub fn label(id: WidgetId, text: impl Into<String>, text_style: TextStyle) -> UiNode {
    UiNode::new(id, UiNodeKind::Label(LabelNode::new(text, text_style)))
}
