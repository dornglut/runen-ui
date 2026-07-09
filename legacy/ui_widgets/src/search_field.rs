//! File: domain/ui/ui_widgets/src/search_field.rs
//! Purpose: Search field widget constructor.

use crate::{TextInputNode, UiNode, UiNodeKind, WidgetId};
use ui_text::TextStyle;
use ui_theme::ThemeTokens;

pub fn search_field(
    id: WidgetId,
    value: impl Into<String>,
    text_style: TextStyle,
    theme: ThemeTokens,
) -> UiNode {
    UiNode::new(
        id,
        UiNodeKind::TextInput(TextInputNode::new(value, "Search", text_style, theme)),
    )
}
