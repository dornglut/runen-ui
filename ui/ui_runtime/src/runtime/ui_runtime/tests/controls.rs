//! File: domain/ui/ui_runtime/src/runtime/ui_runtime/tests/controls.rs
//! Purpose: Retained UI runtime control activation behavior tests.

use super::*;

#[test]
fn disabled_button_click_does_not_activate_or_focus() {
    let theme = ThemeTokens::default();
    let text_style = TextStyle::default();
    let button_id = WidgetId(11);
    let mut button = ButtonNode::new("Disabled", text_style, theme);
    button.enabled = false;
    let tree = UiTree::new(UiNode::new(button_id, UiNodeKind::Button(button)));
    let bounds = UiRect::new(0.0, 0.0, 160.0, 64.0);
    let mut runtime = UiRuntime::new();
    let layouts = runtime.compute_layout(&tree, bounds);

    let outcome = click_widget(&mut runtime, &tree, &layouts, button_id);

    assert!(
        !outcome
            .interactions
            .items
            .contains(&UiInteraction::Activated(button_id)),
        "disabled button should not activate",
    );
    assert_eq!(runtime.state().focused_target, None);
}

#[test]
fn primitive_nodes_do_not_emit_pointer_interactions_or_focus() {
    let image_id = WidgetId(12);
    let tree = UiTree::new(UiNode::with_children(
        WidgetId(1),
        UiNodeKind::Stack(StackNode::vertical(0.0)),
        vec![
            UiNode::new(
                WidgetId(2),
                UiNodeKind::Spacer(SpacerNode::new(ui_math::UiSize::new(4.0, 4.0))),
            ),
            UiNode::new(
                image_id,
                UiNodeKind::Image(ImageNode::new(
                    ui_render_data::UiDrawKey::new(1, Some(2)),
                    UiRect::new(0.0, 0.0, 1.0, 1.0),
                    ui_render_data::UiPaint::WHITE,
                    ui_math::UiSize::new(32.0, 32.0),
                )),
            ),
        ],
    ));
    let bounds = UiRect::new(0.0, 0.0, 96.0, 96.0);
    let mut runtime = UiRuntime::new();
    let layouts = runtime.compute_layout(&tree, bounds);

    let outcome = click_widget(&mut runtime, &tree, &layouts, image_id);

    assert!(outcome.interactions.is_empty());
    assert_eq!(outcome.dispatch.target, None);
    assert_eq!(runtime.state().focused_target, None);
}

#[test]
fn toggle_click_emits_toggled_interaction() {
    let theme = ThemeTokens::default();
    let text_style = TextStyle::default();
    let toggle_id = WidgetId(11);
    let tree = UiTree::new(UiNode::with_children(
        WidgetId(1),
        UiNodeKind::Panel(PanelNode::new(theme.clone())),
        vec![UiNode::new(
            toggle_id,
            UiNodeKind::Toggle(ToggleNode::new("Snap", false, text_style, theme)),
        )],
    ));
    let bounds = UiRect::new(0.0, 0.0, 320.0, 120.0);
    let mut runtime = UiRuntime::new();
    let layouts = runtime.compute_layout(&tree, bounds);
    let outcome = click_widget(&mut runtime, &tree, &layouts, toggle_id);
    assert!(
        outcome
            .interactions
            .items
            .contains(&UiInteraction::Toggled {
                target: toggle_id,
                checked: true,
            }),
        "toggle interaction should be emitted on click release",
    );
}

#[test]
fn numeric_scroll_emits_stepped_value() {
    let theme = ThemeTokens::default();
    let text_style = TextStyle::default();
    let numeric_id = WidgetId(21);
    let tree = UiTree::new(UiNode::with_children(
        WidgetId(1),
        UiNodeKind::Panel(PanelNode::new(theme.clone())),
        vec![UiNode::new(
            numeric_id,
            UiNodeKind::NumericInput(NumericInputNode::new(
                1.0,
                0.5,
                Some(0.0),
                Some(5.0),
                1,
                text_style,
                theme,
            )),
        )],
    ));
    let bounds = UiRect::new(0.0, 0.0, 320.0, 120.0);
    let mut runtime = UiRuntime::new();
    let layouts = runtime.compute_layout(&tree, bounds);
    let point = center_of(&layouts, numeric_id);
    let outcome = runtime.dispatch_input(
        &tree,
        &layouts,
        &UiInputEvent::Pointer(PointerEvent {
            kind: PointerEventKind::Scroll,
            position: point,
            delta: UiVector::new(0.0, -1.0),
            button: None,
            modifiers: Modifiers::default(),
            click_count: 0,
            ..Default::default()
        }),
    );
    assert!(
        outcome
            .interactions
            .items
            .contains(&UiInteraction::NumericStepped {
                target: numeric_id,
                value: 1.5,
            }),
        "numeric scroll should emit stepped value interaction",
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
fn tabs_click_emits_selected_index() {
    let theme = ThemeTokens::default();
    let text_style = TextStyle::default();
    let tabs_id = WidgetId(31);
    let tree = UiTree::new(UiNode::with_children(
        WidgetId(1),
        UiNodeKind::Panel(PanelNode::new(theme.clone())),
        vec![UiNode::new(
            tabs_id,
            UiNodeKind::Tabs(TabsNode::new(["A", "B", "C"], 0, text_style, theme)),
        )],
    ));
    let bounds = UiRect::new(0.0, 0.0, 360.0, 120.0);
    let mut runtime = UiRuntime::new();
    let layouts = runtime.compute_layout(&tree, bounds);
    let tabs_bounds = layouts
        .get(&tabs_id)
        .expect("tabs layout should exist")
        .bounds;
    let point = UiPoint::new(
        tabs_bounds.x + tabs_bounds.width * 0.8,
        tabs_bounds.y + tabs_bounds.height * 0.5,
    );

    let _ = runtime.dispatch_input(
        &tree,
        &layouts,
        &UiInputEvent::Pointer(PointerEvent {
            kind: PointerEventKind::Down,
            position: point,
            delta: UiVector::ZERO,
            button: None,
            modifiers: Modifiers::default(),
            click_count: 1,
            ..Default::default()
        }),
    );
    let outcome = runtime.dispatch_input(
        &tree,
        &layouts,
        &UiInputEvent::Pointer(PointerEvent {
            kind: PointerEventKind::Up,
            position: point,
            delta: UiVector::ZERO,
            button: None,
            modifiers: Modifiers::default(),
            click_count: 1,
            ..Default::default()
        }),
    );

    assert!(
        outcome
            .interactions
            .items
            .contains(&UiInteraction::TabSelected {
                target: tabs_id,
                index: 2,
            }),
        "tab click should emit selected index interaction",
    );
}
