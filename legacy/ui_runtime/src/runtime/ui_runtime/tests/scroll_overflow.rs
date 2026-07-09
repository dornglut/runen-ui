//! File: domain/ui/ui_runtime/src/runtime/ui_runtime/tests/scroll_overflow.rs
//! Purpose: Retained UI runtime scroll overflow geometry behavior tests.

use super::*;

#[test]
fn horizontal_scroll_clamps_offset_on_narrow_bounds_with_middle_drag() {
    let theme = ThemeTokens::default();
    let text_style = TextStyle::default();
    let scroll_id = WidgetId(41);
    let row_id = WidgetId(42);
    let mut row_children = Vec::new();
    for index in 0..8 {
        row_children.push(UiNode::new(
            WidgetId(50 + index),
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
            scroll_id,
            UiNodeKind::Scroll(ScrollNode::horizontal(theme)),
            vec![UiNode::with_children(
                row_id,
                UiNodeKind::Stack(StackNode::horizontal(4.0)),
                row_children,
            )],
        )],
    ));
    let bounds = UiRect::new(0.0, 0.0, 220.0, 96.0);
    let mut runtime = UiRuntime::new();
    let layouts = runtime.compute_layout(&tree, bounds);
    let scroll_layout = layouts
        .get(&scroll_id)
        .expect("horizontal scroll layout should exist");
    assert_eq!(
        scroll_layout.content_bounds, scroll_layout.bounds,
        "overlay scrollbars should not reserve layout gutter",
    );

    let max_offset = runtime
        .max_scroll_offset_for_layout_axis(&tree, &layouts, scroll_id, Axis::Horizontal)
        .expect("horizontal max offset should be computed");
    assert!(max_offset > 0.0, "row should overflow narrow bounds");

    let scroll_point = UiPoint::new(
        scroll_layout.content_bounds.x + scroll_layout.content_bounds.width * 0.5,
        scroll_layout.content_bounds.y + scroll_layout.content_bounds.height * 0.5,
    );
    let start = scroll_point;
    let end = UiPoint::new(start.x - 48.0, start.y);
    let layouts = runtime.compute_layout(&tree, bounds);
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
    for _ in 0..32 {
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
    }
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

    let offset = runtime
        .state()
        .scroll_offset_for_axis(scroll_id, Axis::Horizontal);
    assert!(offset > 0.0, "horizontal scroll should advance offset");
    assert!(
        offset <= max_offset + 0.001,
        "horizontal scroll offset should clamp to measured content range",
    );
}

#[test]
fn two_axis_scroll_applies_independent_offsets() {
    let scroll_id = WidgetId(701);
    let child_id = WidgetId(702);
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
    let initial_layouts = runtime.compute_layout(&tree, bounds);
    let max_x = runtime
        .max_scroll_offset_for_layout_axis(&tree, &initial_layouts, scroll_id, Axis::Horizontal)
        .expect("horizontal max offset should exist");
    let max_y = runtime
        .max_scroll_offset_for_layout_axis(&tree, &initial_layouts, scroll_id, Axis::Vertical)
        .expect("vertical max offset should exist");
    assert!(
        max_x > 80.0,
        "two-axis fixture should overflow horizontally"
    );
    assert!(max_y > 60.0, "two-axis fixture should overflow vertically");

    runtime.set_scroll_offset_for_axis(scroll_id, Axis::Horizontal, 80.0);
    runtime.set_scroll_offset_for_axis(scroll_id, Axis::Vertical, 60.0);
    let layouts = runtime.compute_layout(&tree, bounds);
    let scroll_layout = layouts.get(&scroll_id).expect("scroll layout should exist");
    let child_layout = layouts.get(&child_id).expect("child layout should exist");

    assert!(
        (child_layout.bounds.x - (scroll_layout.content_bounds.x - 80.0)).abs() <= 0.001,
        "horizontal offset should translate content independently",
    );
    assert!(
        (child_layout.bounds.y - (scroll_layout.content_bounds.y - 60.0)).abs() <= 0.001,
        "vertical offset should translate content independently",
    );
}

#[test]
fn two_axis_vertical_scrollbar_stays_pinned_after_horizontal_scroll() {
    let scroll_id = WidgetId(711);
    let child_id = WidgetId(712);
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
    runtime.set_scroll_offset_for_axis(scroll_id, Axis::Horizontal, 96.0);
    let layouts = runtime.compute_layout(&tree, bounds);
    let scroll_layout = layouts.get(&scroll_id).expect("scroll layout should exist");
    let geometry = scrollbar_geometry_for_axis(
        &tree,
        scroll_id,
        &layouts,
        scroll_layout.bounds,
        scroll_layout.content_bounds,
        Axis::Vertical,
    )
    .expect("two-axis vertical scrollbar should have geometry");

    let expected_x =
        scroll_layout.bounds.x + scroll_layout.bounds.width - geometry.track_rect.width;
    assert!(
        (geometry.track_rect.x - expected_x).abs() <= 0.001,
        "vertical scrollbar should be pinned to the visible scroll viewport",
    );
}

#[test]
fn two_axis_scrollbar_tracks_do_not_overlap() {
    let scroll_id = WidgetId(721);
    let child_id = WidgetId(722);
    let tree = two_axis_overflow_scroll_tree(
        scroll_id,
        child_id,
        ScrollInputPolicies::new(
            ScrollInputPolicy::MiddleDragOnly,
            ScrollInputPolicy::WheelOnly,
        ),
    );
    let bounds = UiRect::new(0.0, 0.0, 220.0, 120.0);
    let runtime = UiRuntime::new();
    let layouts = runtime.compute_layout(&tree, bounds);
    let scroll_layout = layouts.get(&scroll_id).expect("scroll layout should exist");
    let vertical = scrollbar_geometry_for_axis(
        &tree,
        scroll_id,
        &layouts,
        scroll_layout.bounds,
        scroll_layout.content_bounds,
        Axis::Vertical,
    )
    .expect("vertical scrollbar should exist");
    let horizontal = scrollbar_geometry_for_axis(
        &tree,
        scroll_id,
        &layouts,
        scroll_layout.bounds,
        scroll_layout.content_bounds,
        Axis::Horizontal,
    )
    .expect("horizontal scrollbar should exist");

    assert!(
        vertical.track_rect.y + vertical.track_rect.height <= horizontal.track_rect.y + 0.001,
        "vertical track should stop above the horizontal track corner",
    );
    assert!(
        horizontal.track_rect.x + horizontal.track_rect.width <= vertical.track_rect.x + 0.001,
        "horizontal track should stop before the vertical track corner",
    );
}

#[test]
fn vertical_scroll_without_overflow_has_no_reserved_gutter() {
    let theme = ThemeTokens::default();
    let text_style = TextStyle::default();
    let scroll_id = WidgetId(61);
    let tree = UiTree::new(UiNode::with_children(
        WidgetId(1),
        UiNodeKind::Panel(PanelNode::new(theme.clone())),
        vec![UiNode::with_children(
            scroll_id,
            UiNodeKind::Scroll(ScrollNode::vertical(theme.clone())),
            vec![UiNode::new(
                WidgetId(62),
                UiNodeKind::Button(ButtonNode::new("One", text_style, theme)),
            )],
        )],
    ));
    let bounds = UiRect::new(0.0, 0.0, 320.0, 140.0);
    let runtime = UiRuntime::new();
    let layouts = runtime.compute_layout(&tree, bounds);
    let scroll_layout = layouts
        .get(&scroll_id)
        .expect("vertical scroll layout should exist");

    assert!(
        (scroll_layout.content_bounds.width - scroll_layout.bounds.width).abs() <= 0.001,
        "vertical scroll should not reserve gutter when content does not overflow",
    );
}

#[test]
fn horizontal_scroll_without_overflow_has_no_reserved_gutter() {
    let theme = ThemeTokens::default();
    let text_style = TextStyle::default();
    let scroll_id = WidgetId(71);
    let tree = UiTree::new(UiNode::with_children(
        WidgetId(1),
        UiNodeKind::Panel(PanelNode::new(theme.clone())),
        vec![UiNode::with_children(
            scroll_id,
            UiNodeKind::Scroll(ScrollNode::horizontal(theme.clone())),
            vec![UiNode::new(
                WidgetId(72),
                UiNodeKind::Button(ButtonNode::new("One", text_style, theme)),
            )],
        )],
    ));
    let bounds = UiRect::new(0.0, 0.0, 320.0, 140.0);
    let runtime = UiRuntime::new();
    let layouts = runtime.compute_layout(&tree, bounds);
    let scroll_layout = layouts
        .get(&scroll_id)
        .expect("horizontal scroll layout should exist");

    assert!(
        (scroll_layout.content_bounds.height - scroll_layout.bounds.height).abs() <= 0.001,
        "horizontal scroll should not reserve gutter when content does not overflow",
    );
}
