//! File: domain/ui/ui_runtime/src/runtime/ui_runtime/focus.rs
//! Purpose: Runtime focus traversal and shortcut ownership helpers.

use crate::{UiNodeKind, UiTree, WidgetId};

pub(super) fn focusable_widgets(tree: &UiTree) -> Vec<WidgetId> {
    tree.walk()
        .filter_map(|node| match &node.kind {
            UiNodeKind::Button(button) if button.enabled => Some(node.id),
            UiNodeKind::TextInput(text_input) if text_input.editable => Some(node.id),
            UiNodeKind::Toggle(toggle) if toggle.enabled => Some(node.id),
            UiNodeKind::NumericInput(numeric) if numeric.enabled => Some(node.id),
            UiNodeKind::Select(select) if select.enabled => Some(node.id),
            UiNodeKind::Table(table) if table.rows.iter().any(|row| row.enabled) => Some(node.id),
            UiNodeKind::Tree(tree) if tree.rows.iter().any(|row| row.enabled) => Some(node.id),
            UiNodeKind::GraphCanvas(graph_canvas) if graph_canvas.focusable => Some(node.id),
            UiNodeKind::Tabs(_) | UiNodeKind::ViewportSurfaceEmbed(_) | UiNodeKind::Scroll(_) => {
                Some(node.id)
            }
            UiNodeKind::Panel(_)
            | UiNodeKind::Popup(_)
            | UiNodeKind::RadialMenu(_)
            | UiNodeKind::OverlayAdornment(_)
            | UiNodeKind::Label(_)
            | UiNodeKind::Button(_)
            | UiNodeKind::TextInput(_)
            | UiNodeKind::Toggle(_)
            | UiNodeKind::NumericInput(_)
            | UiNodeKind::Select(_)
            | UiNodeKind::Table(_)
            | UiNodeKind::Tree(_)
            | UiNodeKind::GraphCanvas(_)
            | UiNodeKind::Spacer(_)
            | UiNodeKind::Divider(_)
            | UiNodeKind::Image(_)
            | UiNodeKind::ProductSurface(_)
            | UiNodeKind::Stack(_)
            | UiNodeKind::Split(_) => None,
        })
        .collect()
}

pub(super) fn focused_widget_captures_viewport_shortcuts(
    tree: &UiTree,
    widget_id: WidgetId,
) -> bool {
    let Some(node) = tree.walk().find(|node| node.id == widget_id) else {
        return false;
    };
    match &node.kind {
        UiNodeKind::Button(button) => button.enabled,
        UiNodeKind::TextInput(text_input) => text_input.editable,
        UiNodeKind::Toggle(toggle) => toggle.enabled,
        UiNodeKind::NumericInput(numeric) => numeric.enabled,
        UiNodeKind::Select(select) => select.enabled,
        UiNodeKind::Table(table) => table.rows.iter().any(|row| row.enabled),
        UiNodeKind::Tree(tree) => tree.rows.iter().any(|row| row.enabled),
        UiNodeKind::GraphCanvas(graph_canvas) => graph_canvas.focusable,
        UiNodeKind::Tabs(_) | UiNodeKind::Scroll(_) => true,
        UiNodeKind::ViewportSurfaceEmbed(_)
        | UiNodeKind::Panel(_)
        | UiNodeKind::Popup(_)
        | UiNodeKind::RadialMenu(_)
        | UiNodeKind::OverlayAdornment(_)
        | UiNodeKind::Label(_)
        | UiNodeKind::Spacer(_)
        | UiNodeKind::Divider(_)
        | UiNodeKind::Image(_)
        | UiNodeKind::ProductSurface(_)
        | UiNodeKind::Stack(_)
        | UiNodeKind::Split(_) => false,
    }
}

pub(super) fn next_focus_target(
    focusable: &[WidgetId],
    current: Option<WidgetId>,
    reverse: bool,
) -> Option<WidgetId> {
    if focusable.is_empty() {
        return None;
    }

    let next_index =
        match current.and_then(|current_id| focusable.iter().position(|id| *id == current_id)) {
            Some(index) => {
                if reverse {
                    if index == 0 {
                        focusable.len() - 1
                    } else {
                        index - 1
                    }
                } else {
                    (index + 1) % focusable.len()
                }
            }
            None => {
                if reverse {
                    focusable.len() - 1
                } else {
                    0
                }
            }
        };

    Some(focusable[next_index])
}
