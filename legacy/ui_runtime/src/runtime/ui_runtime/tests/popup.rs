//! File: domain/ui/ui_runtime/src/runtime/ui_runtime/tests/popup.rs
//! Purpose: Retained UI runtime popup dismissal behavior tests.

use super::*;

#[test]
fn outside_pointer_down_requests_popup_dismiss_and_returns_focus_to_anchor() {
    let theme = ThemeTokens::default();
    let text_style = TextStyle::default();
    let anchor_id = WidgetId(31);
    let popup_id = WidgetId(32);
    let item_id = WidgetId(33);
    let tree = UiTree::new(UiNode::with_children(
        WidgetId(1),
        UiNodeKind::Panel(PanelNode::new(theme.clone())),
        vec![
            UiNode::new(
                anchor_id,
                UiNodeKind::Button(ButtonNode::new("Menu", text_style.clone(), theme.clone())),
            ),
            UiNode::with_children(
                popup_id,
                UiNodeKind::Popup(PopupNode::anchored_bottom_start(anchor_id, theme.clone())),
                vec![UiNode::new(
                    item_id,
                    UiNodeKind::Button(ButtonNode::new("Open", text_style, theme)),
                )],
            ),
        ],
    ));
    let bounds = UiRect::new(0.0, 0.0, 320.0, 220.0);
    let mut runtime = UiRuntime::new();
    runtime.set_focused_widget(Some(item_id));
    let layouts = runtime.compute_layout(&tree, bounds);

    let outcome = runtime.dispatch_input(
        &tree,
        &layouts,
        &UiInputEvent::Pointer(PointerEvent {
            kind: PointerEventKind::Down,
            position: UiPoint::new(
                bounds.x + bounds.width - 4.0,
                bounds.y + bounds.height - 4.0,
            ),
            delta: UiVector::ZERO,
            button: Some(PointerButton::Primary),
            modifiers: Modifiers::default(),
            click_count: 1,
            ..Default::default()
        }),
    );

    assert_eq!(outcome.dispatch.target, Some(popup_id));
    assert_eq!(
        runtime.state().focused_target,
        Some(FocusTargetId(anchor_id.0)),
    );
    assert!(
        outcome
            .interactions
            .items
            .contains(&UiInteraction::PopupDismissRequested {
                popup: popup_id,
                focus_return: Some(anchor_id),
            }),
        "outside pointer down should request popup stack dismissal",
    );
    assert!(
        outcome
            .interactions
            .items
            .contains(&UiInteraction::FocusChanged(FocusChange::Set(
                FocusTargetId(anchor_id.0),
            ))),
        "dismissal should report focus return to the popup anchor",
    );
}

#[test]
fn non_dismissable_popup_does_not_join_popup_dismiss_stack() {
    let theme = ThemeTokens::default();
    let text_style = TextStyle::default();
    let anchor_id = WidgetId(41);
    let popup_id = WidgetId(42);
    let item_id = WidgetId(43);
    let tree = UiTree::new(UiNode::with_children(
        WidgetId(1),
        UiNodeKind::Panel(PanelNode::new(theme.clone())),
        vec![
            UiNode::new(
                anchor_id,
                UiNodeKind::Button(ButtonNode::new("Anchor", text_style.clone(), theme.clone())),
            ),
            UiNode::with_children(
                popup_id,
                UiNodeKind::Popup(
                    PopupNode::anchored_bottom_start(anchor_id, theme.clone())
                        .with_dismiss_policy(PopupDismissPolicy::None),
                ),
                vec![UiNode::new(
                    item_id,
                    UiNodeKind::Button(ButtonNode::new("Overlay item", text_style, theme)),
                )],
            ),
        ],
    ));
    let bounds = UiRect::new(0.0, 0.0, 320.0, 220.0);
    let mut runtime = UiRuntime::new();
    runtime.set_focused_widget(Some(item_id));
    let layouts = runtime.compute_layout(&tree, bounds);

    let outcome = runtime.dispatch_input(
        &tree,
        &layouts,
        &UiInputEvent::Pointer(PointerEvent {
            kind: PointerEventKind::Down,
            position: UiPoint::new(
                bounds.x + bounds.width - 4.0,
                bounds.y + bounds.height - 4.0,
            ),
            delta: UiVector::ZERO,
            button: Some(PointerButton::Primary),
            modifiers: Modifiers::default(),
            click_count: 1,
            ..Default::default()
        }),
    );

    assert_ne!(outcome.dispatch.target, Some(popup_id));
    assert!(
        !outcome
            .interactions
            .items
            .contains(&UiInteraction::PopupDismissRequested {
                popup: popup_id,
                focus_return: Some(anchor_id),
            }),
        "non-dismissable anchored overlays must not consume outside pointer down",
    );
}

#[test]
fn escape_requests_top_popup_dismiss_and_returns_focus_to_anchor() {
    let theme = ThemeTokens::default();
    let text_style = TextStyle::default();
    let low_anchor_id = WidgetId(34);
    let high_anchor_id = WidgetId(35);
    let low_popup_id = WidgetId(36);
    let high_popup_id = WidgetId(37);
    let high_item_id = WidgetId(38);
    let mut high_popup = PopupNode::anchored_bottom_start(high_anchor_id, theme.clone());
    high_popup.layer_order = 3;
    let tree = UiTree::new(UiNode::with_children(
        WidgetId(1),
        UiNodeKind::Panel(PanelNode::new(theme.clone())),
        vec![
            UiNode::new(
                low_anchor_id,
                UiNodeKind::Button(ButtonNode::new("Low", text_style.clone(), theme.clone())),
            ),
            UiNode::new(
                high_anchor_id,
                UiNodeKind::Button(ButtonNode::new("High", text_style.clone(), theme.clone())),
            ),
            UiNode::with_children(
                low_popup_id,
                UiNodeKind::Popup(PopupNode::anchored_bottom_start(
                    low_anchor_id,
                    theme.clone(),
                )),
                vec![UiNode::new(
                    WidgetId(39),
                    UiNodeKind::Button(ButtonNode::new(
                        "Low item",
                        text_style.clone(),
                        theme.clone(),
                    )),
                )],
            ),
            UiNode::with_children(
                high_popup_id,
                UiNodeKind::Popup(high_popup),
                vec![UiNode::new(
                    high_item_id,
                    UiNodeKind::Button(ButtonNode::new("High item", text_style, theme)),
                )],
            ),
        ],
    ));
    let bounds = UiRect::new(0.0, 0.0, 360.0, 240.0);
    let mut runtime = UiRuntime::new();
    runtime.set_focused_widget(Some(high_item_id));
    let layouts = runtime.compute_layout(&tree, bounds);

    let outcome = runtime.dispatch_input(
        &tree,
        &layouts,
        &UiInputEvent::Keyboard(KeyboardEvent {
            key: Key::Escape,
            state: KeyState::Pressed,
            modifiers: Modifiers::default(),
        }),
    );

    assert_eq!(outcome.dispatch.target, Some(high_popup_id));
    assert_eq!(
        runtime.state().focused_target,
        Some(FocusTargetId(high_anchor_id.0)),
    );
    assert!(
        outcome
            .interactions
            .items
            .contains(&UiInteraction::PopupDismissRequested {
                popup: high_popup_id,
                focus_return: Some(high_anchor_id),
            }),
        "escape should dismiss the topmost popup scope",
    );
    assert!(
        !outcome
            .interactions
            .items
            .contains(&UiInteraction::PopupDismissRequested {
                popup: low_popup_id,
                focus_return: Some(low_anchor_id),
            })
    );
}
