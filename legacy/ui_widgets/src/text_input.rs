//! File: domain/ui/ui_widgets/src/text_input.rs
//! Purpose: Text input widget constructor.

use crate::{TextInputNode, UiNode, UiNodeKind, WidgetId};
use ui_text::TextStyle;
use ui_theme::ThemeTokens;

pub fn text_input(
    id: WidgetId,
    value: impl Into<String>,
    placeholder: impl Into<String>,
    text_style: TextStyle,
    theme: ThemeTokens,
) -> UiNode {
    UiNode::new(
        id,
        UiNodeKind::TextInput(TextInputNode::new(value, placeholder, text_style, theme)),
    )
}
