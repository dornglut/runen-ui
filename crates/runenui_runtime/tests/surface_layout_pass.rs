use runenui_core::prelude::{button, column, row, text};
use runenui_runtime::prelude::{
    LogicalRect, LogicalSize, RuntimeNodeId, SurfaceFrame, SurfaceLayoutMetrics, SurfaceNode,
    SurfaceNodeKind, layout_surface, layout_surface_with_metrics,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Action {
    Press,
}

fn assert_f32_eq(actual: f32, expected: f32) {
    assert!(
        (actual - expected).abs() <= f32::EPSILON,
        "expected {expected}, got {actual}",
    );
}

fn assert_rect_eq(actual: LogicalRect, expected: LogicalRect) {
    assert_f32_eq(actual.x(), expected.x());
    assert_f32_eq(actual.y(), expected.y());
    assert_f32_eq(actual.width(), expected.width());
    assert_f32_eq(actual.height(), expected.height());
}

fn root_node(frame: &SurfaceFrame) -> Result<&SurfaceNode, &'static str> {
    frame.root().ok_or("expected root surface node")
}

fn surface_node(frame: &SurfaceFrame, id: RuntimeNodeId) -> Result<&SurfaceNode, &'static str> {
    frame.node(id).ok_or("expected surface node")
}

#[test]
fn vertical_column_lays_out_children_by_gap_and_intrinsic_size() -> Result<(), &'static str> {
    let ui = column((
        text::<Action>("Counter").id("counter.title"),
        button("+").id("counter.increment").on_press(Action::Press),
    ))
    .id("counter.root")
    .gap(8.0);

    let frame = layout_surface(&ui, LogicalSize::new(200.0, 100.0));
    let root = root_node(&frame)?;
    let title = surface_node(&frame, RuntimeNodeId::from_index(1))?;
    let increment = surface_node(&frame, RuntimeNodeId::from_index(2))?;

    assert_eq!(frame.nodes().len(), 3);
    assert_rect_eq(
        root.bounds(),
        LogicalRect::from_xywh(0.0, 0.0, 200.0, 100.0),
    );
    assert_rect_eq(title.bounds(), LogicalRect::from_xywh(0.0, 0.0, 56.0, 20.0));
    assert_rect_eq(
        increment.bounds(),
        LogicalRect::from_xywh(0.0, 28.0, 64.0, 32.0),
    );
    assert_eq!(title.parent(), Some(RuntimeNodeId::ROOT));
    assert_eq!(increment.parent(), Some(RuntimeNodeId::ROOT));
    assert_eq!(
        title.authored_id().map(runenui_core::ElementId::as_str),
        Some("counter.title")
    );
    assert_eq!(increment.kind(), &SurfaceNodeKind::button("+", true));

    Ok(())
}

#[test]
fn horizontal_row_lays_out_children_on_x_axis() -> Result<(), &'static str> {
    let ui = row((text::<Action>("A"), button::<Action>("OK")))
        .id("row.root")
        .gap(4.0);

    let frame = layout_surface(&ui, LogicalSize::new(120.0, 40.0));
    let label = surface_node(&frame, RuntimeNodeId::from_index(1))?;
    let button = surface_node(&frame, RuntimeNodeId::from_index(2))?;

    assert_rect_eq(label.bounds(), LogicalRect::from_xywh(0.0, 0.0, 8.0, 20.0));
    assert_rect_eq(
        button.bounds(),
        LogicalRect::from_xywh(12.0, 0.0, 64.0, 32.0),
    );

    Ok(())
}

#[test]
fn nested_containers_keep_preorder_runtime_ids_and_parent_ids() -> Result<(), &'static str> {
    let ui = column((
        row((button::<Action>("A"), button::<Action>("B")))
            .id("button.row")
            .gap(3.0),
        text::<Action>("End").id("end"),
    ))
    .id("root")
    .gap(5.0);

    let frame = layout_surface(&ui, LogicalSize::new(300.0, 200.0));
    let row_node = surface_node(&frame, RuntimeNodeId::from_index(1))?;
    let first_button = surface_node(&frame, RuntimeNodeId::from_index(2))?;
    let second_button = surface_node(&frame, RuntimeNodeId::from_index(3))?;
    let end = surface_node(&frame, RuntimeNodeId::from_index(4))?;

    assert_eq!(row_node.parent(), Some(RuntimeNodeId::ROOT));
    assert_eq!(first_button.parent(), Some(RuntimeNodeId::from_index(1)));
    assert_eq!(second_button.parent(), Some(RuntimeNodeId::from_index(1)));
    assert_eq!(end.parent(), Some(RuntimeNodeId::ROOT));
    assert_rect_eq(
        row_node.bounds(),
        LogicalRect::from_xywh(0.0, 0.0, 131.0, 32.0),
    );
    assert_rect_eq(
        first_button.bounds(),
        LogicalRect::from_xywh(0.0, 0.0, 64.0, 32.0),
    );
    assert_rect_eq(
        second_button.bounds(),
        LogicalRect::from_xywh(67.0, 0.0, 64.0, 32.0),
    );
    assert_rect_eq(end.bounds(), LogicalRect::from_xywh(0.0, 37.0, 24.0, 20.0));

    Ok(())
}

#[test]
fn disabled_button_surface_kind_preserves_enabled_state() -> Result<(), &'static str> {
    let ui = column(button::<Action>("Disabled").disabled());

    let frame = layout_surface(&ui, LogicalSize::new(100.0, 40.0));
    let disabled = surface_node(&frame, RuntimeNodeId::from_index(1))?;

    assert_eq!(disabled.kind(), &SurfaceNodeKind::button("Disabled", false));

    Ok(())
}

#[test]
fn explicit_metrics_control_intrinsic_text_and_button_sizes() -> Result<(), &'static str> {
    let metrics = SurfaceLayoutMetrics::new(10.0, 18.0, 9.0, 5.0, 22.0, 30.0);
    let ui = column((text::<Action>("ABC"), button::<Action>("ABCD"))).gap(2.0);

    let frame = layout_surface_with_metrics(&ui, LogicalSize::new(100.0, 100.0), metrics);
    let text = surface_node(&frame, RuntimeNodeId::from_index(1))?;
    let button = surface_node(&frame, RuntimeNodeId::from_index(2))?;

    assert_rect_eq(text.bounds(), LogicalRect::from_xywh(0.0, 0.0, 30.0, 18.0));
    assert_rect_eq(
        button.bounds(),
        LogicalRect::from_xywh(0.0, 20.0, 46.0, 22.0),
    );

    Ok(())
}

#[test]
fn empty_container_produces_only_root_surface_node() -> Result<(), &'static str> {
    let ui = column::<Action>(Vec::new()).id("empty.root");

    let frame = layout_surface(&ui, LogicalSize::new(640.0, 480.0));

    assert_eq!(frame.nodes().len(), 1);
    let root = root_node(&frame)?;
    assert_rect_eq(
        root.bounds(),
        LogicalRect::from_xywh(0.0, 0.0, 640.0, 480.0),
    );
    assert_eq!(root.kind(), &SurfaceNodeKind::container());
    assert_eq!(
        root.authored_id().map(runenui_core::ElementId::as_str),
        Some("empty.root")
    );

    Ok(())
}
