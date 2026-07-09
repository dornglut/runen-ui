//! File: domain/ui/ui_runtime/src/runtime/ui_runtime/tests/graph_canvas_keyboard.rs
//! Purpose: Retained UI runtime graph-canvas keyboard behavior tests.

use super::*;

#[test]
fn graph_canvas_keyboard_shortcuts_require_focus() {
    let graph_id = WidgetId(720);
    let tree = graph_canvas_tree(graph_id);
    let bounds = UiRect::new(0.0, 0.0, 260.0, 180.0);
    let mut runtime = UiRuntime::new();
    let layouts = runtime.compute_layout(&tree, bounds);

    let unfocused = runtime.dispatch_input(
        &tree,
        &layouts,
        &UiInputEvent::Keyboard(KeyboardEvent {
            key: Key::Delete,
            state: KeyState::Pressed,
            modifiers: Modifiers::default(),
        }),
    );
    assert_eq!(unfocused.dispatch.target, None);
    assert!(unfocused.interactions.items.is_empty());

    runtime.set_focused_widget(Some(graph_id));

    let outcome = runtime.dispatch_input(
        &tree,
        &layouts,
        &UiInputEvent::Keyboard(KeyboardEvent {
            key: Key::Delete,
            state: KeyState::Pressed,
            modifiers: Modifiers::default(),
        }),
    );

    assert_eq!(outcome.dispatch.target, Some(graph_id));
    assert_eq!(
        outcome.interactions.items,
        vec![UiInteraction::GraphCanvasAction {
            target: graph_id,
            action: ui_graph_editor::GraphCanvasAction::KeyboardDeleteSelection,
        }],
        "delete/backspace may form only a generic graph intent in the substrate",
    );
}

#[test]
fn graph_canvas_keyboard_shortcuts_dispatch_graph_commands() {
    let graph_id = WidgetId(722);
    let tree = graph_canvas_tree(graph_id);
    let bounds = UiRect::new(0.0, 0.0, 260.0, 180.0);
    let mut runtime = UiRuntime::new();
    let layouts = runtime.compute_layout(&tree, bounds);
    runtime.set_focused_widget(Some(graph_id));

    let add_node = runtime.dispatch_input(
        &tree,
        &layouts,
        &UiInputEvent::Keyboard(KeyboardEvent {
            key: Key::Character("a".to_string()),
            state: KeyState::Pressed,
            modifiers: Modifiers::default(),
        }),
    );
    assert_eq!(
        add_node.interactions.items,
        vec![UiInteraction::GraphCanvasAction {
            target: graph_id,
            action: ui_graph_editor::GraphCanvasAction::KeyboardShortcut(
                ui_graph_editor::GraphShortcutAction::AddNode,
            ),
        }],
    );

    let undo = runtime.dispatch_input(
        &tree,
        &layouts,
        &UiInputEvent::Keyboard(KeyboardEvent {
            key: Key::Character("z".to_string()),
            state: KeyState::Pressed,
            modifiers: Modifiers {
                ctrl: true,
                ..Default::default()
            },
        }),
    );
    assert_eq!(
        undo.interactions.items,
        vec![UiInteraction::GraphCanvasAction {
            target: graph_id,
            action: ui_graph_editor::GraphCanvasAction::KeyboardShortcut(
                ui_graph_editor::GraphShortcutAction::Undo,
            ),
        }],
    );
}

#[test]
fn graph_canvas_escape_cancels_active_gesture() {
    let graph_id = WidgetId(721);
    let tree = graph_canvas_tree(graph_id);
    let bounds = UiRect::new(0.0, 0.0, 260.0, 180.0);
    let mut runtime = UiRuntime::new();
    let layouts = runtime.compute_layout(&tree, bounds);

    let _ = runtime.dispatch_input(
        &tree,
        &layouts,
        &UiInputEvent::Pointer(PointerEvent {
            kind: PointerEventKind::Down,
            position: UiPoint::new(32.0, 32.0),
            delta: UiVector::ZERO,
            button: Some(PointerButton::Primary),
            modifiers: Modifiers::default(),
            click_count: 1,
            ..Default::default()
        }),
    );
    assert_eq!(runtime.state().captured_widget, Some(graph_id));

    let outcome = runtime.dispatch_input(
        &tree,
        &layouts,
        &UiInputEvent::Keyboard(KeyboardEvent {
            key: Key::Escape,
            state: KeyState::Pressed,
            modifiers: Modifiers::default(),
        }),
    );

    assert_eq!(outcome.dispatch.target, Some(graph_id));
    assert_eq!(outcome.dispatch.response.capture, PointerCapture::Release);
    assert_eq!(runtime.state().captured_widget, None);
    assert!(
        runtime
            .state()
            .graph_canvas_gestures
            .get(&graph_id)
            .is_some_and(|gesture| gesture.active.is_none()),
        "escape must clear the session-scoped graph gesture",
    );
    assert!(
        outcome
            .interactions
            .items
            .contains(&UiInteraction::GraphCanvasAction {
                target: graph_id,
                action: ui_graph_editor::GraphCanvasAction::CancelGesture,
            }),
        "escape must emit a generic graph cancel intent",
    );
}
