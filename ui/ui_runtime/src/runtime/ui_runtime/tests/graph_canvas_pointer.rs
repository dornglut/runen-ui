//! File: domain/ui/ui_runtime/src/runtime/ui_runtime/tests/graph_canvas_pointer.rs
//! Purpose: Retained UI runtime graph-canvas pointer behavior tests.

use super::*;

#[test]
fn graph_canvas_pointer_capture() {
    let graph_id = WidgetId(700);
    let tree = graph_canvas_tree(graph_id);
    let bounds = UiRect::new(0.0, 0.0, 260.0, 180.0);
    let mut runtime = UiRuntime::new();
    let layouts = runtime.compute_layout(&tree, bounds);

    let down = UiPoint::new(32.0, 32.0);
    let down_outcome = runtime.dispatch_input(
        &tree,
        &layouts,
        &UiInputEvent::Pointer(PointerEvent {
            kind: PointerEventKind::Down,
            position: down,
            delta: UiVector::ZERO,
            button: Some(PointerButton::Primary),
            modifiers: Modifiers::default(),
            click_count: 1,
            ..Default::default()
        }),
    );

    assert_eq!(down_outcome.dispatch.target, Some(graph_id));
    assert_eq!(runtime.state().captured_widget, Some(graph_id));
    assert!(
        down_outcome
            .interactions
            .items
            .iter()
            .any(|interaction| matches!(
                interaction,
                UiInteraction::GraphCanvasAction {
                    target,
                    action: ui_graph_editor::GraphCanvasAction::BeginNodeDrag {
                        node,
                        ..
                    },
                } if *target == graph_id && *node == ui_graph_editor::GraphNodeKey(7)
            )),
        "node-body pointer down must form a graph node drag intent",
    );

    let move_outcome = runtime.dispatch_input(
        &tree,
        &layouts,
        &UiInputEvent::Pointer(PointerEvent {
            kind: PointerEventKind::Move,
            position: UiPoint::new(380.0, 260.0),
            delta: UiVector::new(348.0, 228.0),
            button: None,
            modifiers: Modifiers::default(),
            click_count: 0,
            ..Default::default()
        }),
    );

    assert_eq!(move_outcome.dispatch.target, Some(graph_id));
    assert_eq!(
        move_outcome.dispatch.response.capture,
        PointerCapture::CaptureSelf,
    );
    assert_eq!(runtime.state().captured_widget, Some(graph_id));
    assert!(
        move_outcome
            .interactions
            .items
            .contains(&UiInteraction::GraphCanvasAction {
                target: graph_id,
                action: ui_graph_editor::GraphCanvasAction::UpdateNodeDrag {
                    node: ui_graph_editor::GraphNodeKey(7),
                    delta: ui_graph_editor::GraphVector::new(348, 228),
                },
            }),
        "captured drag must keep routing to the graph canvas outside its bounds",
    );

    let up_outcome = runtime.dispatch_input(
        &tree,
        &layouts,
        &UiInputEvent::Pointer(PointerEvent {
            kind: PointerEventKind::Up,
            position: UiPoint::new(380.0, 260.0),
            delta: UiVector::ZERO,
            button: Some(PointerButton::Primary),
            modifiers: Modifiers::default(),
            click_count: 1,
            ..Default::default()
        }),
    );

    assert_eq!(up_outcome.dispatch.target, Some(graph_id));
    assert_eq!(
        up_outcome.dispatch.response.capture,
        PointerCapture::Release
    );
    assert_eq!(runtime.state().captured_widget, None);
}

#[test]
fn graph_canvas_state_cleans_up_when_node_unmounts() {
    let graph_id = WidgetId(705);
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
    let _ = runtime.dispatch_input(
        &tree,
        &layouts,
        &UiInputEvent::Pointer(PointerEvent {
            kind: PointerEventKind::Scroll,
            position: UiPoint::new(40.0, 40.0),
            delta: UiVector::new(0.0, -4.0),
            button: None,
            modifiers: Modifiers::default(),
            click_count: 0,
            ..Default::default()
        }),
    );
    assert!(
        runtime
            .state()
            .graph_canvas_gestures
            .contains_key(&graph_id)
    );
    assert!(
        runtime
            .state()
            .graph_canvas_viewports
            .contains_key(&graph_id)
    );

    let unmounted_tree = UiTree::new(UiNode::new(
        WidgetId(1),
        UiNodeKind::Panel(PanelNode::new(ThemeTokens::default())),
    ));
    runtime.retain_state_for_tree(&unmounted_tree);

    assert!(
        !runtime
            .state()
            .graph_canvas_gestures
            .contains_key(&graph_id)
    );
    assert!(
        !runtime
            .state()
            .graph_canvas_viewports
            .contains_key(&graph_id)
    );
    assert_eq!(runtime.state().captured_widget, None);
    assert_eq!(runtime.state().pressed_widget, None);
    assert_eq!(runtime.state().focused_target, None);
}

#[test]
fn graph_canvas_wheel_zoom_does_not_leak_to_scroll_or_viewport() {
    let scroll_id = WidgetId(710);
    let graph_id = WidgetId(711);
    let tree = scroll_wrapped_compact_graph_canvas_tree(scroll_id, graph_id);
    let bounds = UiRect::new(0.0, 0.0, 260.0, 140.0);
    let mut runtime = UiRuntime::new();
    let layouts = runtime.compute_layout(&tree, bounds);
    let graph_point = center_of(&layouts, graph_id);

    let outcome = runtime.dispatch_input(
        &tree,
        &layouts,
        &UiInputEvent::Pointer(PointerEvent {
            kind: PointerEventKind::Scroll,
            position: graph_point,
            delta: UiVector::new(0.0, -4.0),
            button: None,
            modifiers: Modifiers::default(),
            click_count: 0,
            ..Default::default()
        }),
    );

    assert_eq!(outcome.dispatch.target, Some(graph_id));
    assert_eq!(
        outcome.dispatch.response.propagation,
        EventPropagation::Stop,
    );
    assert_eq!(
        runtime.scroll_offset(scroll_id),
        0.0,
        "graph wheel zoom must not scroll an ancestor viewport",
    );
    assert!(
        runtime
            .state()
            .graph_canvas_viewports
            .get(&graph_id)
            .is_some_and(|viewport| viewport.zoom_milli != 1000),
        "wheel input over the graph must update graph zoom state",
    );
    assert!(
        outcome
            .interactions
            .items
            .iter()
            .any(|interaction| matches!(
                interaction,
                UiInteraction::GraphCanvasAction {
                    target,
                    action: ui_graph_editor::GraphCanvasAction::Zoom { .. },
                } if *target == graph_id
            )),
        "wheel input over the graph must emit a graph zoom intent",
    );
    assert!(
        !outcome
            .interactions
            .items
            .iter()
            .any(|interaction| matches!(interaction, UiInteraction::ScrollInputOwned { .. })),
        "graph wheel zoom must not leak as scroll ownership",
    );

    let scroll_only_point = UiPoint::new(graph_point.x, graph_point.y + 72.0);
    let scroll_outcome = runtime.dispatch_input(
        &tree,
        &layouts,
        &UiInputEvent::Pointer(PointerEvent {
            kind: PointerEventKind::Scroll,
            position: scroll_only_point,
            delta: UiVector::new(0.0, -4.0),
            button: None,
            modifiers: Modifiers::default(),
            click_count: 0,
            ..Default::default()
        }),
    );

    assert_eq!(scroll_outcome.dispatch.target, Some(scroll_id));
    assert!(
        runtime.scroll_offset(scroll_id) > 0.0,
        "wheel outside the graph but inside the scroll owner must still scroll",
    );
    assert!(
        scroll_outcome
            .interactions
            .items
            .iter()
            .any(|interaction| matches!(
                interaction,
                UiInteraction::ScrollInputOwned { owner, .. } if *owner == scroll_id
            )),
        "scroll owner must receive wheel ownership outside the graph canvas",
    );
    assert!(
        !scroll_outcome
            .interactions
            .items
            .iter()
            .any(|interaction| matches!(
                interaction,
                UiInteraction::GraphCanvasAction {
                    action: ui_graph_editor::GraphCanvasAction::Zoom { .. },
                    ..
                }
            )),
        "wheel outside the graph must not emit graph zoom",
    );
}

#[test]
fn graph_canvas_pointer_capture_does_not_leak_to_viewport() {
    let graph_id = WidgetId(712);
    let viewport_id = WidgetId(713);
    let tree = graph_and_viewport_tree(graph_id, viewport_id);
    let bounds = UiRect::new(0.0, 0.0, 260.0, 340.0);
    let mut runtime = UiRuntime::new();
    let layouts = runtime.compute_layout(&tree, bounds);
    let viewport_point = center_of(&layouts, viewport_id);

    let outcome = runtime.dispatch_input(
        &tree,
        &layouts,
        &UiInputEvent::Pointer(PointerEvent {
            kind: PointerEventKind::Down,
            position: viewport_point,
            delta: UiVector::ZERO,
            button: Some(PointerButton::Primary),
            modifiers: Modifiers::default(),
            click_count: 1,
            ..Default::default()
        }),
    );

    assert_eq!(outcome.dispatch.target, Some(viewport_id));
    assert_eq!(
        runtime.state().captured_widget,
        Some(viewport_id),
        "primary drag outside the graph must use the hit widget's normal capture",
    );
    assert!(
        outcome
            .interactions
            .items
            .iter()
            .all(|interaction| !matches!(interaction, UiInteraction::GraphCanvasAction { .. })),
        "viewport pointer down must not create graph canvas intents",
    );
}
