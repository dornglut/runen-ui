//! File: domain/ui/ui_runtime/src/runtime/ui_runtime/tests/scrollbar.rs
//! Purpose: Retained UI runtime scrollbar behavior tests.

use super::*;

#[test]
fn vertical_scrollbar_thumb_drag_updates_scroll_offset() {
    let scroll_id = WidgetId(301);
    let tree = vertical_overflow_scroll_tree(scroll_id, WidgetId(302));
    let bounds = UiRect::new(0.0, 0.0, 220.0, 120.0);
    let mut runtime = UiRuntime::new();
    let layouts = runtime.compute_layout(&tree, bounds);
    let start = scrollbar_thumb_center(&tree, &layouts, scroll_id);
    let end = UiPoint::new(start.x, start.y + 36.0);

    let _ = runtime.dispatch_input(
        &tree,
        &layouts,
        &UiInputEvent::Pointer(PointerEvent {
            kind: PointerEventKind::Down,
            position: start,
            delta: UiVector::ZERO,
            button: Some(PointerButton::Primary),
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

    assert!(
        runtime.state().scroll_offset(scroll_id) > 0.0,
        "vertical thumb drag should advance scroll offset",
    );
}

#[test]
fn horizontal_scrollbar_thumb_drag_updates_scroll_offset() {
    let scroll_id = WidgetId(311);
    let tree = horizontal_overflow_scroll_tree(scroll_id, WidgetId(312));
    let bounds = UiRect::new(0.0, 0.0, 220.0, 96.0);
    let mut runtime = UiRuntime::new();
    let layouts = runtime.compute_layout(&tree, bounds);
    let start = scrollbar_thumb_center(&tree, &layouts, scroll_id);
    let end = UiPoint::new(start.x + 42.0, start.y);

    let _ = runtime.dispatch_input(
        &tree,
        &layouts,
        &UiInputEvent::Pointer(PointerEvent {
            kind: PointerEventKind::Down,
            position: start,
            delta: UiVector::ZERO,
            button: Some(PointerButton::Primary),
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

    assert!(
        runtime
            .state()
            .scroll_offset_for_axis(scroll_id, Axis::Horizontal)
            > 0.0,
        "horizontal thumb drag should advance scroll offset",
    );
}

#[test]
fn scrollbar_thumb_drag_clamps_to_max_offset() {
    let scroll_id = WidgetId(321);
    let tree = vertical_overflow_scroll_tree(scroll_id, WidgetId(322));
    let bounds = UiRect::new(0.0, 0.0, 220.0, 120.0);
    let mut runtime = UiRuntime::new();
    let layouts = runtime.compute_layout(&tree, bounds);
    let start = scrollbar_thumb_center(&tree, &layouts, scroll_id);
    let end = UiPoint::new(start.x, start.y + 10_000.0);
    let max_offset = runtime
        .max_scroll_offset_for_layout(&tree, &layouts, scroll_id)
        .expect("max scroll should be computed");

    let _ = runtime.dispatch_input(
        &tree,
        &layouts,
        &UiInputEvent::Pointer(PointerEvent {
            kind: PointerEventKind::Down,
            position: start,
            delta: UiVector::ZERO,
            button: Some(PointerButton::Primary),
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

    assert!(
        (runtime.state().scroll_offset(scroll_id) - max_offset).abs() <= 0.001,
        "thumb drag should clamp to max scroll offset",
    );
}

#[test]
fn scrollbar_thumb_drag_releases_capture_on_pointer_up() {
    let scroll_id = WidgetId(331);
    let tree = vertical_overflow_scroll_tree(scroll_id, WidgetId(332));
    let bounds = UiRect::new(0.0, 0.0, 220.0, 120.0);
    let mut runtime = UiRuntime::new();
    let layouts = runtime.compute_layout(&tree, bounds);
    let start = scrollbar_thumb_center(&tree, &layouts, scroll_id);

    let _ = runtime.dispatch_input(
        &tree,
        &layouts,
        &UiInputEvent::Pointer(PointerEvent {
            kind: PointerEventKind::Down,
            position: start,
            delta: UiVector::ZERO,
            button: Some(PointerButton::Primary),
            modifiers: Modifiers::default(),
            click_count: 1,
            ..Default::default()
        }),
    );
    assert!(runtime.state().scrollbar_thumb_drag.is_some());

    let layouts = runtime.compute_layout(&tree, bounds);
    let _ = runtime.dispatch_input(
        &tree,
        &layouts,
        &UiInputEvent::Pointer(PointerEvent {
            kind: PointerEventKind::Up,
            position: start,
            delta: UiVector::ZERO,
            button: Some(PointerButton::Primary),
            modifiers: Modifiers::default(),
            click_count: 1,
            ..Default::default()
        }),
    );

    assert!(runtime.state().scrollbar_thumb_drag.is_none());
    assert_eq!(runtime.state().captured_widget, None);
    assert_eq!(runtime.state().pressed_widget, None);
}

#[test]
fn scrollbar_track_without_overflow_does_not_capture() {
    let theme = ThemeTokens::default();
    let text_style = TextStyle::default();
    let scroll_id = WidgetId(341);
    let tree = UiTree::new(UiNode::with_children(
        WidgetId(1),
        UiNodeKind::Panel(PanelNode::new(theme.clone())),
        vec![UiNode::with_children(
            scroll_id,
            UiNodeKind::Scroll(ScrollNode::vertical(theme.clone())),
            vec![UiNode::new(
                WidgetId(342),
                UiNodeKind::Button(ButtonNode::new("One", text_style, theme)),
            )],
        )],
    ));
    let bounds = UiRect::new(0.0, 0.0, 220.0, 120.0);
    let mut runtime = UiRuntime::new();
    let layouts = runtime.compute_layout(&tree, bounds);
    let scroll_bounds = layouts.get(&scroll_id).expect("scroll layout").bounds;
    let point = UiPoint::new(
        scroll_bounds.x + scroll_bounds.width - 2.0,
        scroll_bounds.y + 8.0,
    );

    let _ = runtime.dispatch_input(
        &tree,
        &layouts,
        &UiInputEvent::Pointer(PointerEvent {
            kind: PointerEventKind::Down,
            position: point,
            delta: UiVector::ZERO,
            button: Some(PointerButton::Primary),
            modifiers: Modifiers::default(),
            click_count: 1,
            ..Default::default()
        }),
    );

    assert!(
        runtime.state().scrollbar_thumb_drag.is_none(),
        "non-overflowing scroll should not start a scrollbar-thumb capture",
    );
}
