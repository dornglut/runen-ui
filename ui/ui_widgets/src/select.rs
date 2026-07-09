//! File: domain/ui/ui_widgets/src/select.rs
//! Purpose: Select widget constructor.

use crate::{SelectNode, UiNode, UiNodeKind, WidgetId};
use ui_text::TextStyle;
use ui_theme::ThemeTokens;

pub fn select(
    id: WidgetId,
    options: impl IntoIterator<Item = impl Into<String>>,
    selected_index: Option<usize>,
    placeholder: impl Into<String>,
    text_style: TextStyle,
    theme: ThemeTokens,
) -> UiNode {
    UiNode::new(
        id,
        UiNodeKind::Select(SelectNode::new(
            options,
            selected_index,
            placeholder,
            text_style,
            theme,
        )),
    )
}
