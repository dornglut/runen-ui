//! File: domain/ui/ui_runtime/src/runtime/ui_runtime/tests/console_scroll_policy.rs
//! Purpose: Retained UI runtime console scroll policy behavior tests.

use super::*;

#[test]
fn two_axis_console_policy_uses_wheel_vertical_and_middle_drag_horizontal() {
    let scroll_id = WidgetId(731);
    let child_id = WidgetId(732);
    let tree = two_axis_overflow_scroll_tree(
        scroll_id,
        child_id,
        ScrollInputPolicies::new(
            ScrollInputPolicy::MiddleDragOnly,
            ScrollInputPolicy::WheelOnly,
        ),
    );
    let bounds = UiRect::new(0.0, 0.0, 220.0, 120.0);
    let mut runtime = UiRuntime::new();
    let layouts = runtime.compute_layout(&tree, bounds);
    let start = center_of(&layouts, scroll_id);

    let _ = runtime.dispatch_input(
        &tree,
        &layouts,
        &UiInputEvent::Pointer(PointerEvent {
            kind: PointerEventKind::Scroll,
            position: start,
            delta: UiVector::new(0.0, -8.0),
            button: None,
            modifiers: Modifiers::default(),
            click_count: 0,
            ..Default::default()
        }),
    );
    assert!(
        runtime
            .state()
            .scroll_offset_for_axis(scroll_id, Axis::Vertical)
            > 0.0,
        "wheel should scroll the vertical axis",
    );
    assert_eq!(
        runtime
            .state()
            .scroll_offset_for_axis(scroll_id, Axis::Horizontal),
        0.0,
        "vertical wheel should not move the horizontal axis",
    );
    assert_eq!(
        runtime.state().scrollbar_opacity(scroll_id, Axis::Vertical),
        1.0,
        "vertical wheel should reveal the vertical scrollbar",
    );
    assert_eq!(
        runtime
            .state()
            .scrollbar_opacity(scroll_id, Axis::Horizontal),
        0.0,
        "vertical wheel should not reveal the horizontal scrollbar",
    );

    let _ = runtime.dispatch_input(
        &tree,
        &layouts,
        &UiInputEvent::Pointer(PointerEvent {
            kind: PointerEventKind::Scroll,
            position: start,
            delta: UiVector::new(-8.0, 0.0),
            button: None,
            modifiers: Modifiers::default(),
            click_count: 0,
            ..Default::default()
        }),
    );
    assert_eq!(
        runtime
            .state()
            .scroll_offset_for_axis(scroll_id, Axis::Horizontal),
        0.0,
        "horizontal wheel should be blocked by console policy",
    );

    runtime.set_scroll_offset_for_axis(scroll_id, Axis::Vertical, 0.0);
    let layouts = runtime.compute_layout(&tree, bounds);
    let vertical_end = UiPoint::new(start.x, start.y - 40.0);
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
    let _ = runtime.dispatch_input(
        &tree,
        &layouts,
        &UiInputEvent::Pointer(PointerEvent {
            kind: PointerEventKind::Move,
            position: vertical_end,
            delta: vertical_end - start,
            button: None,
            modifiers: Modifiers::default(),
            click_count: 0,
            ..Default::default()
        }),
    );
    let _ = runtime.dispatch_input(
        &tree,
        &layouts,
        &UiInputEvent::Pointer(PointerEvent {
            kind: PointerEventKind::Up,
            position: vertical_end,
            delta: UiVector::ZERO,
            button: Some(PointerButton::Middle),
            modifiers: Modifiers::default(),
            click_count: 1,
            ..Default::default()
        }),
    );
    assert_eq!(
        runtime
            .state()
            .scroll_offset_for_axis(scroll_id, Axis::Vertical),
        0.0,
        "middle-drag should not move the vertical axis under console policy",
    );

    let layouts = runtime.compute_layout(&tree, bounds);
    let horizontal_end = UiPoint::new(start.x - 40.0, start.y);
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
    let _ = runtime.dispatch_input(
        &tree,
        &layouts,
        &UiInputEvent::Pointer(PointerEvent {
            kind: PointerEventKind::Move,
            position: horizontal_end,
            delta: horizontal_end - start,
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
        "middle-drag should pan the horizontal axis under console policy",
    );
}

#[test]
fn two_axis_console_policy_reveals_only_changed_scrollbar_axis() {
    let scroll_id = WidgetId(741);
    let child_id = WidgetId(742);
    let tree = two_axis_overflow_scroll_tree(
        scroll_id,
        child_id,
        ScrollInputPolicies::new(
            ScrollInputPolicy::MiddleDragOnly,
            ScrollInputPolicy::WheelOnly,
        ),
    );
    let bounds = UiRect::new(0.0, 0.0, 220.0, 120.0);

    let mut vertical_wheel_runtime = UiRuntime::new();
    let layouts = vertical_wheel_runtime.compute_layout(&tree, bounds);
    let start = center_of(&layouts, scroll_id);
    let _ = vertical_wheel_runtime.dispatch_input(
        &tree,
        &layouts,
        &UiInputEvent::Pointer(PointerEvent {
            kind: PointerEventKind::Scroll,
            position: start,
            delta: UiVector::new(0.0, -8.0),
            button: None,
            modifiers: Modifiers::default(),
            click_count: 0,
            ..Default::default()
        }),
    );
    assert_eq!(
        vertical_wheel_runtime
            .state()
            .scrollbar_opacity(scroll_id, Axis::Vertical),
        1.0,
        "vertical wheel should reveal the vertical scrollbar",
    );
    assert_eq!(
        vertical_wheel_runtime
            .state()
            .scrollbar_opacity(scroll_id, Axis::Horizontal),
        0.0,
        "vertical wheel should leave the horizontal scrollbar hidden",
    );

    let mut horizontal_wheel_runtime = UiRuntime::new();
    let layouts = horizontal_wheel_runtime.compute_layout(&tree, bounds);
    let _ = horizontal_wheel_runtime.dispatch_input(
        &tree,
        &layouts,
        &UiInputEvent::Pointer(PointerEvent {
            kind: PointerEventKind::Scroll,
            position: start,
            delta: UiVector::new(-8.0, 0.0),
            button: None,
            modifiers: Modifiers::default(),
            click_count: 0,
            ..Default::default()
        }),
    );
    assert_eq!(
        horizontal_wheel_runtime
            .state()
            .scrollbar_opacity(scroll_id, Axis::Horizontal),
        0.0,
        "blocked horizontal wheel should not reveal the horizontal scrollbar",
    );
    assert_eq!(
        horizontal_wheel_runtime
            .state()
            .scrollbar_opacity(scroll_id, Axis::Vertical),
        0.0,
        "blocked horizontal wheel should not reveal the vertical scrollbar",
    );

    let mut vertical_middle_drag_runtime = UiRuntime::new();
    let layouts = vertical_middle_drag_runtime.compute_layout(&tree, bounds);
    let vertical_end = UiPoint::new(start.x, start.y - 40.0);
    let _ = vertical_middle_drag_runtime.dispatch_input(
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
    let _ = vertical_middle_drag_runtime.dispatch_input(
        &tree,
        &layouts,
        &UiInputEvent::Pointer(PointerEvent {
            kind: PointerEventKind::Move,
            position: vertical_end,
            delta: vertical_end - start,
            button: None,
            modifiers: Modifiers::default(),
            click_count: 0,
            ..Default::default()
        }),
    );
    assert_eq!(
        vertical_middle_drag_runtime
            .state()
            .scrollbar_opacity(scroll_id, Axis::Vertical),
        0.0,
        "ignored vertical middle-drag should not reveal the vertical scrollbar",
    );
    assert_eq!(
        vertical_middle_drag_runtime
            .state()
            .scrollbar_opacity(scroll_id, Axis::Horizontal),
        0.0,
        "ignored vertical middle-drag should not reveal the horizontal scrollbar",
    );

    let mut horizontal_middle_drag_runtime = UiRuntime::new();
    let layouts = horizontal_middle_drag_runtime.compute_layout(&tree, bounds);
    let horizontal_end = UiPoint::new(start.x - 40.0, start.y);
    let _ = horizontal_middle_drag_runtime.dispatch_input(
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
    let _ = horizontal_middle_drag_runtime.dispatch_input(
        &tree,
        &layouts,
        &UiInputEvent::Pointer(PointerEvent {
            kind: PointerEventKind::Move,
            position: horizontal_end,
            delta: horizontal_end - start,
            button: None,
            modifiers: Modifiers::default(),
            click_count: 0,
            ..Default::default()
        }),
    );
    assert_eq!(
        horizontal_middle_drag_runtime
            .state()
            .scrollbar_opacity(scroll_id, Axis::Horizontal),
        1.0,
        "horizontal middle-drag should reveal the horizontal scrollbar",
    );
    assert_eq!(
        horizontal_middle_drag_runtime
            .state()
            .scrollbar_opacity(scroll_id, Axis::Vertical),
        0.0,
        "horizontal middle-drag should leave the vertical scrollbar hidden",
    );
}
