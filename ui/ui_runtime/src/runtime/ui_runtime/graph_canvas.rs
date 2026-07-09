//! File: domain/ui/ui_runtime/src/runtime/ui_runtime/graph_canvas.rs
//! Purpose: Runtime graph-canvas keyboard helpers.

use ui_input::Key;

use crate::{UiNodeKind, UiTree, WidgetId};

pub(super) fn is_graph_canvas_widget(tree: &UiTree, widget_id: WidgetId) -> bool {
    tree.walk()
        .any(|node| node.id == widget_id && matches!(node.kind, UiNodeKind::GraphCanvas(_)))
}

pub(super) fn graph_canvas_shortcut_action(
    event: &ui_input::KeyboardEvent,
) -> Option<ui_graph_editor::GraphShortcutAction> {
    if matches!(event.key, Key::Delete | Key::Backspace) {
        return Some(ui_graph_editor::GraphShortcutAction::DeleteSelection);
    }
    match &event.key {
        Key::Character(value) if !event.modifiers.ctrl && value.eq_ignore_ascii_case("a") => {
            Some(ui_graph_editor::GraphShortcutAction::AddNode)
        }
        Key::Character(value) if event.modifiers.ctrl && value.eq_ignore_ascii_case("z") => {
            Some(ui_graph_editor::GraphShortcutAction::Undo)
        }
        Key::Character(value) if event.modifiers.ctrl && value.eq_ignore_ascii_case("y") => {
            Some(ui_graph_editor::GraphShortcutAction::Redo)
        }
        Key::Character(value) if event.modifiers.ctrl && value.eq_ignore_ascii_case("b") => {
            Some(ui_graph_editor::GraphShortcutAction::BuildPreview)
        }
        Key::Character(value) if !event.modifiers.ctrl && value.eq_ignore_ascii_case("f") => {
            Some(ui_graph_editor::GraphShortcutAction::FocusPreview)
        }
        _ => None,
    }
}
