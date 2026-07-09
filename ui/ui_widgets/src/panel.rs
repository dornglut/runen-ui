//! File: domain/ui/ui_runtime/src/widgets/panel.rs
//! Purpose: Panel widget constructor.

use crate::{PanelNode, UiNode, UiNodeKind, WidgetId};
use ui_theme::ThemeTokens;

pub fn panel(id: WidgetId, theme: ThemeTokens, children: Vec<UiNode>) -> UiNode {
    UiNode::with_children(id, UiNodeKind::Panel(PanelNode::new(theme)), children)
}
