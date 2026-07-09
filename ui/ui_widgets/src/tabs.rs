//! File: domain/ui/ui_widgets/src/tabs.rs
//! Purpose: Tabs widget constructor.

use crate::{TabsNode, UiNode, UiNodeKind, WidgetId};
use ui_text::TextStyle;
use ui_theme::ThemeTokens;

pub fn tabs(
    id: WidgetId,
    labels: impl IntoIterator<Item = impl Into<String>>,
    selected_index: usize,
    text_style: TextStyle,
    theme: ThemeTokens,
) -> UiNode {
    UiNode::new(
        id,
        UiNodeKind::Tabs(TabsNode::new(labels, selected_index, text_style, theme)),
    )
}
