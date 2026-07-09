//! File: domain/ui/ui_runtime/src/runtime/ui_runtime/tests/middle_pan.rs
//! Purpose: Retained UI runtime middle-pan behavior tests.

use super::*;

#[test]
fn vertical_scroll_ignores_middle_drag_input() {
    let theme = ThemeTokens::default();
    let text_style = TextStyle::default();
    let scroll_id = WidgetId(191);
    let mut rows = Vec::new();
    for index in 0..24 {
        rows.push(UiNode::new(
            WidgetId(200 + index),
            UiNodeKind::Button(ButtonNode::new(
                format!("Row {index}"),
                text_style.clone(),
                theme.clone(),
            )),
        ));
    }
    let tree = UiTree::new(UiNode::with_children(
        WidgetId(1),
        UiNodeKind::Panel(PanelNode::new(theme.clone())),
        vec![UiNode::with_children(
            scroll_id,
            UiNodeKind::Scroll(ScrollNode::vertical(theme.clone())),
            vec![UiNode::with_children(
                WidgetId(199),
                UiNodeKind::Stack(StackNode::vertical(2.0)),
                rows,
            )],
        )],
    ));
    let bounds = UiRect::new(0.0, 0.0, 220.0, 120.0);
    let mut runtime = UiRuntime::new();
    let layouts = runtime.compute_layout(&tree, bounds);
    let start = center_of(&layouts, scroll_id);
    let end = UiPoint::new(start.x, start.y - 40.0);

    let _ = runtime.dispatch_input(
        &tree,
        &layouts,
        &UiInputEvent::Pointer(PointerEvent {
            kind: PointerEventKind::Down,
            position: start,
            delta: UiVector::ZERO,
            button: Some(PointerButton::Middle),
            modifiers: Modifiers::default(),
            click_count: 1,
            ..Default::default()
        }),
    );
    let layouts = runtime.compute_layout(&tree, bounds);
    let _ = runtime.dispatch_input(
        &tree,
        &layouts,
        &UiInputEvent::Pointer(PointerEvent {
            kind: PointerEventKind::Move,
            position: end,
            delta: end - start,
            button: None,
            modifiers: Modifiers::default(),
            click_count: 0,
            ..Default::default()
        }),
    );
    let layouts = runtime.compute_layout(&tree, bounds);
    let _ = runtime.dispatch_input(
        &tree,
        &layouts,
        &UiInputEvent::Pointer(PointerEvent {
            kind: PointerEventKind::Up,
            position: end,
            delta: UiVector::ZERO,
            button: Some(PointerButton::Middle),
            modifiers: Modifiers::default(),
            click_count: 1,
            ..Default::default()
        }),
    );

    assert!(
        runtime.state().scroll_offset(scroll_id) <= 0.001,
        "vertical scroll should ignore middle-drag input by default",
    );
}

#[test]
fn middle_drag_pans_horizontal_scroll_offset() {
    let theme = ThemeTokens::default();
    let text_style = TextStyle::default();
    let horizontal_scroll_id = WidgetId(101);
    let row_id = WidgetId(102);
    let mut row_children = Vec::new();
    for index in 0..12 {
        row_children.push(UiNode::new(
            WidgetId(120 + index),
            UiNodeKind::Button(ButtonNode::new(
                format!("Button {index}"),
                text_style.clone(),
                theme.clone(),
            )),
        ));
    }
    let tree = UiTree::new(UiNode::with_children(
        WidgetId(1),
        UiNodeKind::Panel(PanelNode::new(theme.clone())),
        vec![UiNode::with_children(
            horizontal_scroll_id,
            UiNodeKind::Scroll(ScrollNode::horizontal(theme.clone())),
            vec![UiNode::with_children(
                row_id,
                UiNodeKind::Stack(StackNode::horizontal(4.0)),
                row_children,
            )],
        )],
    ));
    let bounds = UiRect::new(0.0, 0.0, 240.0, 96.0);
    let mut runtime = UiRuntime::new();
    let layouts = runtime.compute_layout(&tree, bounds);
    let start = center_of(&layouts, horizontal_scroll_id);
    let end = UiPoint::new(start.x - 40.0, start.y);

    let _ = runtime.dispatch_input(
        &tree,
        &layouts,
        &UiInputEvent::Pointer(PointerEvent {
            kind: PointerEventKind::Down,
            position: start,
            delta: UiVector::ZERO,
            button: Some(PointerButton::Middle),
            modifiers: Modifiers::default(),
            click_count: 1,
            ..Default::default()
        }),
    );
    let layouts = runtime.compute_layout(&tree, bounds);
    let _ = runtime.dispatch_input(
        &tree,
        &layouts,
        &UiInputEvent::Pointer(PointerEvent {
            kind: PointerEventKind::Move,
            position: end,
            delta: end - start,
            button: None,
            modifiers: Modifiers::default(),
            click_count: 0,
            ..Default::default()
        }),
    );
    let layouts = runtime.compute_layout(&tree, bounds);
    let _ = runtime.dispatch_input(
        &tree,
        &layouts,
        &UiInputEvent::Pointer(PointerEvent {
            kind: PointerEventKind::Up,
            position: end,
            delta: UiVector::ZERO,
            button: Some(PointerButton::Middle),
            modifiers: Modifiers::default(),
            click_count: 1,
            ..Default::default()
        }),
    );

    assert!(
        runtime
            .state()
            .scroll_offset_for_axis(horizontal_scroll_id, Axis::Horizontal)
            > 0.0,
        "middle-button drag should pan horizontal scroll offset",
    );
}

#[test]
fn middle_drag_without_starting_scroll_owner_does_not_switch_to_hovered_scroll() {
    let theme = ThemeTokens::default();
    let text_style = TextStyle::default();
    let anchor_id = WidgetId(201);
    let horizontal_scroll_id = WidgetId(202);
    let row_id = WidgetId(203);
    let row_children = (0..12)
        .map(|index| {
            UiNode::new(
                WidgetId(220 + index),
                UiNodeKind::Button(ButtonNode::new(
                    format!("Button {index}"),
                    text_style.clone(),
                    theme.clone(),
                )),
            )
        })
        .collect::<Vec<_>>();
    let tree = UiTree::new(UiNode::with_children(
        WidgetId(1),
        UiNodeKind::Stack(StackNode::horizontal(theme.spacing.sm)),
        vec![
            UiNode::new(
                anchor_id,
                UiNodeKind::Button(ButtonNode::new("Anchor", text_style.clone(), theme.clone())),
            ),
            UiNode::with_children(
                horizontal_scroll_id,
                UiNodeKind::Scroll(ScrollNode::horizontal(theme.clone())),
                vec![UiNode::with_children(
                    row_id,
                    UiNodeKind::Stack(StackNode::horizontal(4.0)),
                    row_children,
                )],
            ),
        ],
    ));
    let bounds = UiRect::new(0.0, 0.0, 360.0, 96.0);
    let mut runtime = UiRuntime::new();
    let layouts = runtime.compute_layout(&tree, bounds);
    let start = center_of(&layouts, anchor_id);
    let scroll_bounds = layouts
        .get(&horizontal_scroll_id)
        .expect("horizontal scroll layout should exist")
        .bounds;
    let end = UiPoint::new(scroll_bounds.x + scroll_bounds.width * 0.5, start.y);

    let _ = runtime.dispatch_input(
        &tree,
        &layouts,
        &UiInputEvent::Pointer(PointerEvent {
            kind: PointerEventKind::Down,
            position: start,
            delta: UiVector::ZERO,
            button: Some(PointerButton::Middle),
            modifiers: Modifiers::default(),
            click_count: 1,
            ..Default::default()
        }),
    );
    let layouts = runtime.compute_layout(&tree, bounds);
    let _ = runtime.dispatch_input(
        &tree,
        &layouts,
        &UiInputEvent::Pointer(PointerEvent {
            kind: PointerEventKind::Move,
            position: end,
            delta: end - start,
            button: None,
            modifiers: Modifiers::default(),
            click_count: 0,
            ..Default::default()
        }),
    );

    assert_eq!(
        runtime
            .state()
            .scroll_offset_for_axis(horizontal_scroll_id, Axis::Horizontal),
        0.0,
        "middle-drag that starts outside a scroll owner must not adopt another scroll area mid-drag",
    );
}
