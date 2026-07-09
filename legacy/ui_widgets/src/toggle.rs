//! File: domain/ui/ui_widgets/src/toggle.rs
//! Purpose: Toggle widget constructor.

use crate::{ToggleNode, UiNode, UiNodeKind, WidgetId};
use ui_text::TextStyle;
use ui_theme::ThemeTokens;

pub fn toggle(
    id: WidgetId,
    label: impl Into<String>,
    checked: bool,
    text_style: TextStyle,
    theme: ThemeTokens,
) -> UiNode {
    UiNode::new(
        id,
        UiNodeKind::Toggle(ToggleNode::new(label, checked, text_style, theme)),
    )
}
