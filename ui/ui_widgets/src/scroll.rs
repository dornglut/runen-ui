//! File: domain/ui/ui_runtime/src/widgets/scroll.rs
//! Purpose: Axis-aware scroll-container widget constructors.

use crate::{ScrollNode, UiNode, UiNodeKind, WidgetId};
use ui_math::Axis;
use ui_theme::ThemeTokens;

pub fn scroll(id: WidgetId, axis: Axis, theme: ThemeTokens, children: Vec<UiNode>) -> UiNode {
    let node = match axis {
        Axis::Horizontal => ScrollNode::horizontal(theme),
        Axis::Vertical => ScrollNode::vertical(theme),
    };
    UiNode::with_children(id, UiNodeKind::Scroll(node), children)
}

pub fn vscroll(id: WidgetId, theme: ThemeTokens, children: Vec<UiNode>) -> UiNode {
    scroll(id, Axis::Vertical, theme, children)
}

pub fn hscroll(id: WidgetId, theme: ThemeTokens, children: Vec<UiNode>) -> UiNode {
    scroll(id, Axis::Horizontal, theme, children)
}

pub fn xy_scroll(id: WidgetId, theme: ThemeTokens, children: Vec<UiNode>) -> UiNode {
    UiNode::with_children(id, UiNodeKind::Scroll(ScrollNode::both(theme)), children)
}
