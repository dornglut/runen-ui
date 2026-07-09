//! File: domain/ui/ui_runtime/src/runtime/ui_runtime/tests.rs
//! Purpose: Retained UI runtime behavior tests.

use super::UiRuntime;
use crate::output::build_ui_frame::{scrollbar_geometry, scrollbar_geometry_for_axis};
use crate::{
    ButtonNode, ComputedLayoutMap, GraphCanvasNode, ImageNode, NumericInputNode, PanelNode,
    PopupDismissPolicy, PopupNode, ScrollInputPolicies, ScrollInputPolicy, ScrollNode, SpacerNode,
    StackNode, TabsNode, TextInputNode, ToggleNode, UiInputOutcome, UiInteraction, UiInvalidation,
    UiNode, UiNodeKind, UiTree, ViewportSurfaceEmbedNode, WidgetId,
};
use ui_input::{
    EventPropagation, FocusChange, FocusTargetId, Key, KeyState, KeyboardEvent, Modifiers,
    PointerButton, PointerCapture, PointerEvent, PointerEventKind, TextInputEvent, UiInputEvent,
};
use ui_math::{Axis, UiPoint, UiRect, UiSize, UiVector};
use ui_render_data::ViewportSurfaceEmbedSlotId;
use ui_text::TextStyle;
use ui_theme::ThemeTokens;

fn sample_tree() -> (UiTree, UiRect, WidgetId, WidgetId) {
    let theme = ThemeTokens::default();
    let text_style = TextStyle::default();
    let button_a = WidgetId(2);
    let button_b = WidgetId(3);
    let stack_id = WidgetId(10);
    let root_id = WidgetId(1);
    let tree = UiTree::new(UiNode::with_children(
        root_id,
        UiNodeKind::Panel(PanelNode::new(theme.clone())),
        vec![UiNode::with_children(
            stack_id,
            UiNodeKind::Stack(StackNode::vertical(theme.spacing.sm)),
            vec![
                UiNode::new(
                    button_a,
                    UiNodeKind::Button(ButtonNode::new("One", text_style.clone(), theme.clone())),
                ),
                UiNode::new(
                    button_b,
                    UiNodeKind::Button(ButtonNode::new("Two", text_style, theme)),
                ),
            ],
        )],
    ));
    (
        tree,
        UiRect::new(0.0, 0.0, 640.0, 360.0),
        button_a,
        button_b,
    )
}

fn center_of(layouts: &ComputedLayoutMap, id: WidgetId) -> UiPoint {
    let bounds = layouts.get(&id).expect("layout entry should exist").bounds;
    UiPoint::new(
        bounds.x + bounds.width * 0.5,
        bounds.y + bounds.height * 0.5,
    )
}

fn graph_canvas_view_model() -> ui_graph_editor::GraphCanvasViewModel {
    let node = ui_graph_editor::GraphNodeKey(7);
    let port = ui_graph_editor::GraphPortKey(9);
    ui_graph_editor::GraphCanvasViewModel {
        canvas_id: ui_graph_editor::GraphCanvasId(3),
        viewport: ui_graph_editor::GraphViewport::default(),
        nodes: vec![ui_graph_editor::GraphNodeView::new(
            node,
            "Node",
            ui_graph_editor::GraphRect::new(20, 20, 80, 44),
        )],
        ports: vec![ui_graph_editor::GraphPortView::new(
            port,
            node,
            "out",
            ui_graph_editor::GraphPortDirection::Output,
            ui_graph_editor::GraphRect::new(88, 32, 10, 10),
        )],
        edges: Vec::new(),
        overlays: Vec::new(),
        selection: ui_graph_editor::GraphSelection::default(),
        hit_test_scene: ui_graph_editor::GraphHitTestScene {
            canvas_rect: ui_graph_editor::GraphRect::new(0, 0, 240, 160),
            nodes: vec![ui_graph_editor::GraphNodeBounds {
                node,
                rect: ui_graph_editor::GraphRect::new(20, 20, 80, 44),
            }],
            ports: vec![ui_graph_editor::GraphPortBounds {
                port,
                node,
                rect: ui_graph_editor::GraphRect::new(88, 32, 10, 10),
            }],
            edges: Vec::new(),
            selections: Vec::new(),
        },
    }
}

fn graph_canvas_node(theme: ThemeTokens) -> GraphCanvasNode {
    GraphCanvasNode::new(graph_canvas_view_model(), theme).with_min_size(UiSize::new(240.0, 160.0))
}

fn graph_canvas_tree(graph_id: WidgetId) -> UiTree {
    let theme = ThemeTokens::default();
    UiTree::new(UiNode::with_children(
        WidgetId(1),
        UiNodeKind::Panel(PanelNode::new(theme.clone())),
        vec![UiNode::new(
            graph_id,
            UiNodeKind::GraphCanvas(graph_canvas_node(theme)),
        )],
    ))
}

fn scroll_wrapped_compact_graph_canvas_tree(scroll_id: WidgetId, graph_id: WidgetId) -> UiTree {
    let theme = ThemeTokens::default();
    let mut graph_canvas = graph_canvas_node(theme.clone());
    graph_canvas.min_size = UiSize::new(240.0, 72.0);
    UiTree::new(UiNode::with_children(
        WidgetId(1),
        UiNodeKind::Panel(PanelNode::new(theme.clone())),
        vec![UiNode::with_children(
            scroll_id,
            UiNodeKind::Scroll(ScrollNode::vertical(theme.clone())),
            vec![UiNode::with_children(
                WidgetId(902),
                UiNodeKind::Stack(StackNode::vertical(0.0)),
                vec![
                    UiNode::new(graph_id, UiNodeKind::GraphCanvas(graph_canvas)),
                    UiNode::new(
                        WidgetId(903),
                        UiNodeKind::Spacer(SpacerNode::new(UiSize::new(240.0, 480.0))),
                    ),
                ],
            )],
        )],
    ))
}

fn graph_and_viewport_tree(graph_id: WidgetId, viewport_id: WidgetId) -> UiTree {
    let theme = ThemeTokens::default();
    UiTree::new(UiNode::with_children(
        WidgetId(1),
        UiNodeKind::Panel(PanelNode::new(theme.clone())),
        vec![UiNode::with_children(
            WidgetId(904),
            UiNodeKind::Stack(StackNode::vertical(0.0)),
            vec![
                UiNode::new(graph_id, UiNodeKind::GraphCanvas(graph_canvas_node(theme))),
                UiNode::new(
                    viewport_id,
                    UiNodeKind::ViewportSurfaceEmbed(ViewportSurfaceEmbedNode {
                        viewport_id: 5,
                        slot: ViewportSurfaceEmbedSlotId(5),
                        min_size: UiSize::new(240.0, 140.0),
                    }),
                ),
            ],
        )],
    ))
}

fn vertical_overflow_scroll_tree(scroll_id: WidgetId, child_id: WidgetId) -> UiTree {
    let theme = ThemeTokens::default();
    let text_style = TextStyle::default();
    let rows = (0..24)
        .map(|index| {
            UiNode::new(
                WidgetId(10_000 + index),
                UiNodeKind::Button(ButtonNode::new(
                    format!("Row {index}"),
                    text_style.clone(),
                    theme.clone(),
                )),
            )
        })
        .collect::<Vec<_>>();
    UiTree::new(UiNode::with_children(
        WidgetId(1),
        UiNodeKind::Panel(PanelNode::new(theme.clone())),
        vec![UiNode::with_children(
            scroll_id,
            UiNodeKind::Scroll(ScrollNode::vertical(theme.clone())),
            vec![UiNode::with_children(
                child_id,
                UiNodeKind::Stack(StackNode::vertical(2.0)),
                rows,
            )],
        )],
    ))
}

fn horizontal_overflow_scroll_tree(scroll_id: WidgetId, child_id: WidgetId) -> UiTree {
    let theme = ThemeTokens::default();
    let text_style = TextStyle::default();
    let columns = (0..16)
        .map(|index| {
            UiNode::new(
                WidgetId(11_000 + index),
                UiNodeKind::Button(ButtonNode::new(
                    format!("Button {index}"),
                    text_style.clone(),
                    theme.clone(),
                )),
            )
        })
        .collect::<Vec<_>>();
    UiTree::new(UiNode::with_children(
        WidgetId(1),
        UiNodeKind::Panel(PanelNode::new(theme.clone())),
        vec![UiNode::with_children(
            scroll_id,
            UiNodeKind::Scroll(
                ScrollNode::horizontal(theme.clone())
                    .with_input_policies(ScrollInputPolicies::default()),
            ),
            vec![UiNode::with_children(
                child_id,
                UiNodeKind::Stack(StackNode::horizontal(4.0)),
                columns,
            )],
        )],
    ))
}

fn two_axis_overflow_scroll_tree(
    scroll_id: WidgetId,
    child_id: WidgetId,
    input_policies: ScrollInputPolicies,
) -> UiTree {
    let theme = ThemeTokens::default();
    let text_style = TextStyle::default();
    let rows = (0..20)
        .map(|row| {
            let columns = (0..10)
                .map(|column| {
                    UiNode::new(
                        WidgetId(20_000 + row * 100 + column),
                        UiNodeKind::Button(ButtonNode::new(
                            format!("Cell {row}-{column}"),
                            text_style.clone(),
                            theme.clone(),
                        )),
                    )
                })
                .collect::<Vec<_>>();
            UiNode::with_children(
                WidgetId(21_000 + row),
                UiNodeKind::Stack(StackNode::horizontal(4.0)),
                columns,
            )
        })
        .collect::<Vec<_>>();
    UiTree::new(UiNode::with_children(
        WidgetId(1),
        UiNodeKind::Panel(PanelNode::new(theme.clone())),
        vec![UiNode::with_children(
            scroll_id,
            UiNodeKind::Scroll(ScrollNode::both(theme.clone()).with_input_policies(input_policies)),
            vec![UiNode::with_children(
                child_id,
                UiNodeKind::Stack(StackNode::vertical(2.0)),
                rows,
            )],
        )],
    ))
}

fn scrollbar_thumb_center(
    tree: &UiTree,
    layouts: &ComputedLayoutMap,
    scroll_id: WidgetId,
) -> UiPoint {
    let layout = layouts.get(&scroll_id).expect("scroll layout should exist");
    let geometry = scrollbar_geometry(
        tree,
        scroll_id,
        layouts,
        layout.bounds,
        layout.content_bounds,
    )
    .expect("scrollbar geometry should exist");
    UiPoint::new(
        geometry.thumb_rect.x + geometry.thumb_rect.width * 0.5,
        geometry.thumb_rect.y + geometry.thumb_rect.height * 0.5,
    )
}

fn focus_by_pointer_down(
    runtime: &mut UiRuntime,
    tree: &UiTree,
    layouts: &ComputedLayoutMap,
    widget_id: WidgetId,
) {
    let point = center_of(layouts, widget_id);
    let outcome = runtime.dispatch_input(
        tree,
        layouts,
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
    assert_eq!(outcome.dispatch.target, Some(widget_id));
    assert_eq!(
        runtime.state().focused_target,
        Some(FocusTargetId(widget_id.0)),
    );
}

fn click_widget(
    runtime: &mut UiRuntime,
    tree: &UiTree,
    layouts: &ComputedLayoutMap,
    widget_id: WidgetId,
) -> UiInputOutcome {
    let point = center_of(layouts, widget_id);
    let _ = runtime.dispatch_input(
        tree,
        layouts,
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
    runtime.dispatch_input(
        tree,
        layouts,
        &UiInputEvent::Pointer(PointerEvent {
            kind: PointerEventKind::Up,
            position: point,
            delta: UiVector::ZERO,
            button: None,
            modifiers: Modifiers::default(),
            click_count: 1,
            ..Default::default()
        }),
    )
}

mod console_scroll_policy;
mod controls;
mod graph_canvas_keyboard;
mod graph_canvas_pointer;
mod keyboard_focus;
mod middle_pan;
mod popup;
mod scroll_overflow;
mod scroll_wheel;
mod scrollbar;
