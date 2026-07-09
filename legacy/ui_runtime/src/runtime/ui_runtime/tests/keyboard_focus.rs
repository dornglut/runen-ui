//! File: domain/ui/ui_runtime/src/runtime/ui_runtime/tests/keyboard_focus.rs
//! Purpose: Retained UI runtime keyboard focus behavior tests.

use super::*;

#[test]
fn keyboard_event_routes_to_focused_widget() {
    let (tree, bounds, button_a, _) = sample_tree();
    let mut runtime = UiRuntime::new();
    let layouts = runtime.compute_layout(&tree, bounds);
    focus_by_pointer_down(&mut runtime, &tree, &layouts, button_a);
    let layouts = runtime.compute_layout(&tree, bounds);

    let event = KeyboardEvent {
        key: Key::Character("k".to_string()),
        state: KeyState::Pressed,
        modifiers: Modifiers::default(),
    };
    let outcome = runtime.dispatch_input(&tree, &layouts, &UiInputEvent::Keyboard(event.clone()));

    assert_eq!(outcome.dispatch.target, Some(button_a));
    assert_eq!(
        outcome.interactions.items,
        vec![UiInteraction::KeyboardInput {
            target: button_a,
            event,
        }],
    );
    assert_eq!(outcome.invalidation, UiInvalidation::default());
}

#[test]
fn text_event_routes_to_focused_widget_and_signals_relayout() {
    let (tree, bounds, button_a, _) = sample_tree();
    let mut runtime = UiRuntime::new();
    let layouts = runtime.compute_layout(&tree, bounds);
    focus_by_pointer_down(&mut runtime, &tree, &layouts, button_a);
    let layouts = runtime.compute_layout(&tree, bounds);

    let event = TextInputEvent {
        text: "abc".to_string(),
    };
    let outcome = runtime.dispatch_input(&tree, &layouts, &UiInputEvent::Text(event.clone()));

    assert_eq!(outcome.dispatch.target, Some(button_a));
    assert_eq!(
        outcome.interactions.items,
        vec![UiInteraction::TextInput {
            target: button_a,
            event,
        }],
    );
    assert_eq!(
        outcome.invalidation,
        UiInvalidation {
            repaint: true,
            relayout: true,
        },
    );
}

#[test]
fn tab_and_shift_tab_traverse_focusable_widgets() {
    let (tree, bounds, button_a, button_b) = sample_tree();
    let mut runtime = UiRuntime::new();
    let layouts = runtime.compute_layout(&tree, bounds);
    focus_by_pointer_down(&mut runtime, &tree, &layouts, button_a);
    let layouts = runtime.compute_layout(&tree, bounds);

    let tab_outcome = runtime.dispatch_input(
        &tree,
        &layouts,
        &UiInputEvent::Keyboard(KeyboardEvent {
            key: Key::Tab,
            state: KeyState::Pressed,
            modifiers: Modifiers::default(),
        }),
    );
    assert_eq!(tab_outcome.dispatch.target, Some(button_b));
    assert_eq!(
        runtime.state().focused_target,
        Some(FocusTargetId(button_b.0)),
    );
    assert_eq!(
        tab_outcome.interactions.items,
        vec![UiInteraction::FocusChanged(FocusChange::Set(
            FocusTargetId(button_b.0,)
        ))],
    );

    let shift_tab_outcome = runtime.dispatch_input(
        &tree,
        &layouts,
        &UiInputEvent::Keyboard(KeyboardEvent {
            key: Key::Tab,
            state: KeyState::Pressed,
            modifiers: Modifiers {
                shift: true,
                ctrl: false,
                alt: false,
                meta: false,
            },
        }),
    );
    assert_eq!(shift_tab_outcome.dispatch.target, Some(button_a));
    assert_eq!(
        runtime.state().focused_target,
        Some(FocusTargetId(button_a.0)),
    );
    assert_eq!(
        shift_tab_outcome.invalidation,
        UiInvalidation {
            repaint: true,
            relayout: false,
        },
    );
}

#[test]
fn focus_traversal_skips_disabled_and_read_only_controls() {
    let theme = ThemeTokens::default();
    let text_style = TextStyle::default();
    let enabled_button_id = WidgetId(2);
    let disabled_button_id = WidgetId(3);
    let read_only_text_id = WidgetId(4);
    let numeric_id = WidgetId(5);

    let mut disabled_button = ButtonNode::new("Disabled", text_style.clone(), theme.clone());
    disabled_button.enabled = false;
    let mut read_only_text =
        TextInputNode::new("value", "placeholder", text_style.clone(), theme.clone());
    read_only_text.editable = false;

    let tree = UiTree::new(UiNode::with_children(
        WidgetId(1),
        UiNodeKind::Stack(StackNode::vertical(theme.spacing.sm)),
        vec![
            UiNode::new(
                enabled_button_id,
                UiNodeKind::Button(ButtonNode::new(
                    "Enabled",
                    text_style.clone(),
                    theme.clone(),
                )),
            ),
            UiNode::new(disabled_button_id, UiNodeKind::Button(disabled_button)),
            UiNode::new(read_only_text_id, UiNodeKind::TextInput(read_only_text)),
            UiNode::new(
                numeric_id,
                UiNodeKind::NumericInput(NumericInputNode::new(
                    1.0, 0.25, None, None, 2, text_style, theme,
                )),
            ),
        ],
    ));
    let bounds = UiRect::new(0.0, 0.0, 320.0, 160.0);
    let mut runtime = UiRuntime::new();
    let layouts = runtime.compute_layout(&tree, bounds);
    focus_by_pointer_down(&mut runtime, &tree, &layouts, enabled_button_id);

    let outcome = runtime.dispatch_input(
        &tree,
        &layouts,
        &UiInputEvent::Keyboard(KeyboardEvent {
            key: Key::Tab,
            state: KeyState::Pressed,
            modifiers: Modifiers::default(),
        }),
    );

    assert_eq!(outcome.dispatch.target, Some(numeric_id));
    assert_eq!(
        runtime.state().focused_target,
        Some(FocusTargetId(numeric_id.0)),
    );
}

#[test]
fn focused_text_controls_capture_viewport_shortcuts_but_viewports_do_not() {
    let theme = ThemeTokens::default();
    let text_style = TextStyle::default();
    let text_input_id = WidgetId(2);
    let viewport_id = WidgetId(3);
    let tree = UiTree::new(UiNode::with_children(
        WidgetId(1),
        UiNodeKind::Stack(StackNode::vertical(theme.spacing.sm)),
        vec![
            UiNode::new(
                text_input_id,
                UiNodeKind::TextInput(TextInputNode::new("", "Search", text_style, theme.clone())),
            ),
            UiNode::new(
                viewport_id,
                UiNodeKind::ViewportSurfaceEmbed(ViewportSurfaceEmbedNode::new(
                    1,
                    ViewportSurfaceEmbedSlotId::new(1),
                )),
            ),
        ],
    ));
    let mut runtime = UiRuntime::new();

    runtime.set_focused_widget(Some(text_input_id));
    assert!(
        runtime.focused_widget_captures_viewport_shortcuts(&tree),
        "focused text input should block viewport-local shortcut handling",
    );

    runtime.set_focused_widget(Some(viewport_id));
    assert!(
        !runtime.focused_widget_captures_viewport_shortcuts(&tree),
        "focused viewport embed should leave viewport-local shortcuts active",
    );
}
