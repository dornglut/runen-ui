//! File: domain/ui/ui_widgets/src/tree_widget.rs
//! Purpose: Tree widget constructor.

use crate::{TreeNode, TreeRow, UiNode, UiNodeKind, WidgetId};
use ui_text::TextStyle;
use ui_theme::ThemeTokens;

pub fn tree(
    id: WidgetId,
    rows: impl IntoIterator<Item = TreeRow>,
    text_style: TextStyle,
    theme: ThemeTokens,
) -> UiNode {
    UiNode::new(id, UiNodeKind::Tree(TreeNode::new(rows, text_style, theme)))
}
