//! File: domain/ui/ui_runtime/src/widgets/button.rs
//! Purpose: Button widget constructor.

use crate::{ButtonNode, UiNode, UiNodeKind, WidgetId};
use ui_text::TextStyle;
use ui_theme::ThemeTokens;

pub fn button(
    id: WidgetId,
    label: impl Into<String>,
    text_style: TextStyle,
    theme: ThemeTokens,
) -> UiNode {
    button_selected(id, label, text_style, theme, false)
}

pub fn button_selected(
    id: WidgetId,
    label: impl Into<String>,
    text_style: TextStyle,
    theme: ThemeTokens,
    selected: bool,
) -> UiNode {
    let mut node = ButtonNode::new(label, text_style, theme);
    node.selected = selected;
    UiNode::new(id, UiNodeKind::Button(node))
}
