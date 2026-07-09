//! File: domain/ui/ui_runtime/src/runtime/ui_runtime/tests/scroll_wheel.rs
//! Purpose: Retained UI runtime scroll wheel routing behavior tests.

use super::*;

#[test]
fn horizontal_scroll_uses_vertical_wheel_input_when_it_overflows() {
    let theme = ThemeTokens::default();
    let text_style = TextStyle::default();
    let scroll_id = WidgetId(171);
    let row_id = WidgetId(172);
    let mut row_children = Vec::new();
    for index in 0..8 {
        row_children.push(UiNode::new(
            WidgetId(180 + index),
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
            UiNodeKind::Scroll(
                ScrollNode::horizontal(theme.clone())
                    .with_input_policies(ScrollInputPolicies::default()),
            ),
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
    let scroll_point = center_of(&layouts, scroll_id);
    let _ = runtime.dispatch_input(
        &tree,
        &layouts,
        &UiInputEvent::Pointer(PointerEvent {
            kind: PointerEventKind::Scroll,
            position: scroll_point,
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
            .scroll_offset_for_axis(scroll_id, Axis::Horizontal)
            > 0.001,
        "vertical wheel should scroll a horizontal-only overflow region",
    );
}

#[test]
fn horizontal_scroll_uses_shift_wheel_input() {
    let theme = ThemeTokens::default();
    let text_style = TextStyle::default();
    let scroll_id = WidgetId(173);
    let row_id = WidgetId(174);
    let mut row_children = Vec::new();
    for index in 0..8 {
        row_children.push(UiNode::new(
            WidgetId(190 + index),
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
            UiNodeKind::Scroll(ScrollNode::horizontal(theme.clone())),
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
    let scroll_point = center_of(&layouts, scroll_id);
    let _ = runtime.dispatch_input(
        &tree,
        &layouts,
        &UiInputEvent::Pointer(PointerEvent {
            kind: PointerEventKind::Scroll,
            position: scroll_point,
            delta: UiVector::new(0.0, -8.0),
            button: None,
            modifiers: Modifiers {
                shift: true,
                ..Modifiers::default()
            },
            click_count: 0,
            ..Default::default()
        }),
    );
    assert!(
        runtime
            .state()
            .scroll_offset_for_axis(scroll_id, Axis::Horizontal)
            > 0.001,
        "shift-wheel should scroll horizontally",
    );
}

#[test]
fn wheel_scroll_routes_to_vertical_owner_when_nested_horizontal_cannot_scroll() {
    let theme = ThemeTokens::default();
    let text_style = TextStyle::default();
    let vertical_scroll_id = WidgetId(81);
    let horizontal_scroll_id = WidgetId(82);
    let row_id = WidgetId(83);
    let mut row_children = Vec::new();
    for index in 0..24 {
        row_children.push(UiNode::new(
            WidgetId(90 + index),
            UiNodeKind::Button(ButtonNode::new(
                format!("R{index}"),
                text_style.clone(),
                theme.clone(),
            )),
        ));
    }
    let tree = UiTree::new(UiNode::with_children(
        WidgetId(1),
        UiNodeKind::Panel(PanelNode::new(theme.clone())),
        vec![UiNode::with_children(
            vertical_scroll_id,
            UiNodeKind::Scroll(ScrollNode::vertical(theme.clone())),
            vec![UiNode::with_children(
                horizontal_scroll_id,
                UiNodeKind::Scroll(ScrollNode::horizontal(theme.clone())),
                vec![UiNode::with_children(
                    row_id,
                    UiNodeKind::Stack(StackNode::vertical(2.0)),
                    row_children,
                )],
            )],
        )],
    ));
    let bounds = UiRect::new(0.0, 0.0, 220.0, 120.0);
    let mut runtime = UiRuntime::new();
    let layouts = runtime.compute_layout(&tree, bounds);
    let visible_viewport = layouts
        .get(&vertical_scroll_id)
        .expect("vertical scroll layout should exist")
        .content_bounds;
    let pointer = UiPoint::new(
        visible_viewport.x + visible_viewport.width * 0.5,
        visible_viewport.y + visible_viewport.height * 0.5,
    );
    let vertical_max = runtime
        .max_scroll_offset_for_layout(&tree, &layouts, vertical_scroll_id)
        .expect("vertical max offset should be computed");
    assert!(
        vertical_max > 0.0,
        "vertical scroll should overflow in nested-scroll test setup",
    );

    let _ = runtime.dispatch_input(
        &tree,
        &layouts,
        &UiInputEvent::Pointer(PointerEvent {
            kind: PointerEventKind::Scroll,
            position: pointer,
            delta: UiVector::new(0.0, -8.0),
            button: None,
            modifiers: Modifiers::default(),
            click_count: 0,
            ..Default::default()
        }),
    );

    let vertical_offset = runtime.state().scroll_offset(vertical_scroll_id);
    let horizontal_offset = runtime
        .state()
        .scroll_offset_for_axis(horizontal_scroll_id, Axis::Horizontal);
    assert!(
        vertical_offset > 0.0,
        "vertical ancestor should consume wheel when nested horizontal scroll has no horizontal delta (vertical={vertical_offset}, horizontal={horizontal_offset})",
    );
}

#[test]
fn wheel_scroll_at_boundary_is_owned_without_mutating_offset() {
    let scroll_id = WidgetId(91);
    let child_id = WidgetId(92);
    let tree = vertical_overflow_scroll_tree(scroll_id, child_id);
    let bounds = UiRect::new(0.0, 0.0, 220.0, 120.0);
    let mut runtime = UiRuntime::new();
    let layouts = runtime.compute_layout(&tree, bounds);
    let max_offset = runtime
        .max_scroll_offset_for_layout(&tree, &layouts, scroll_id)
        .expect("scroll max offset should exist");
    runtime.set_scroll_offset(scroll_id, max_offset);
    let layouts = runtime.compute_layout(&tree, bounds);
    let pointer = center_of(&layouts, scroll_id);

    let outcome = runtime.dispatch_input(
        &tree,
        &layouts,
        &UiInputEvent::Pointer(PointerEvent {
            kind: PointerEventKind::Scroll,
            position: pointer,
            delta: UiVector::new(0.0, -8.0),
            button: None,
            modifiers: Modifiers::default(),
            click_count: 0,
            ..Default::default()
        }),
    );

    assert_eq!(outcome.dispatch.target, Some(scroll_id));
    assert_eq!(
        outcome.dispatch.response.propagation,
        ui_input::EventPropagation::Stop,
        "boundary wheel input remains owned by the nearest scroll owner",
    );
    assert_eq!(runtime.scroll_offset(scroll_id), max_offset);
    assert!(
        outcome
            .interactions
            .items
            .contains(&UiInteraction::ScrollInputOwned {
                owner: scroll_id,
                axis: Axis::Vertical,
                changed: false,
                at_boundary: true,
            }),
        "boundary ownership should be reported separately from content mutation",
    );
}
