use runenui_core::prelude::{
    EdgeInsets, Length, SpacingToken, StyleTokens, UnresolvedStyleToken, button, column, row, text,
};
use runenui_runtime::prelude::{
    LogicalRect, LogicalSize, RuntimeNodeId, SurfaceBuildContext, SurfaceFrame,
    SurfaceLayoutMetrics, SurfaceNode, SurfaceNodeKind, publish_surface,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Action {
    Press,
}

fn surface_frame<Action>(root: &runenui_core::Element<Action>, size: LogicalSize) -> SurfaceFrame {
    let tokens = StyleTokens::new();
    let context = SurfaceBuildContext::new(&tokens);
    publish_surface(root, size, &context).into_parts().0
}

fn surface_frame_with_metrics<Action>(
    root: &runenui_core::Element<Action>,
    size: LogicalSize,
    metrics: SurfaceLayoutMetrics,
) -> SurfaceFrame {
    let tokens = StyleTokens::new();
    let context = SurfaceBuildContext::new(&tokens).with_layout_metrics(metrics);
    publish_surface(root, size, &context).into_parts().0
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

    let frame = surface_frame(&ui, LogicalSize::new(200.0, 100.0));
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

    let frame = surface_frame(&ui, LogicalSize::new(120.0, 40.0));
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

    let frame = surface_frame(&ui, LogicalSize::new(300.0, 200.0));
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

    let frame = surface_frame(&ui, LogicalSize::new(100.0, 40.0));
    let disabled = surface_node(&frame, RuntimeNodeId::from_index(1))?;

    assert_eq!(disabled.kind(), &SurfaceNodeKind::button("Disabled", false));

    Ok(())
}

#[test]
fn explicit_metrics_control_intrinsic_text_and_button_sizes() -> Result<(), &'static str> {
    let metrics = SurfaceLayoutMetrics::new(10.0, 18.0, 9.0, 22.0, 30.0);
    let ui = column((text::<Action>("ABC"), button::<Action>("ABCD"))).gap(2.0);

    let frame = surface_frame_with_metrics(&ui, LogicalSize::new(100.0, 100.0), metrics);
    let text = surface_node(&frame, RuntimeNodeId::from_index(1))?;
    let button = surface_node(&frame, RuntimeNodeId::from_index(2))?;

    assert_rect_eq(text.bounds(), LogicalRect::from_xywh(0.0, 0.0, 30.0, 18.0));
    assert_rect_eq(
        button.bounds(),
        LogicalRect::from_xywh(0.0, 20.0, 36.0, 22.0),
    );

    Ok(())
}

#[test]
fn empty_container_produces_only_root_surface_node() -> Result<(), &'static str> {
    let ui = column::<Action>(Vec::new()).id("empty.root");

    let frame = surface_frame(&ui, LogicalSize::new(640.0, 480.0));

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

#[test]
fn root_and_text_padding_affect_content_origin_and_outer_size() -> Result<(), &'static str> {
    let root_padding = EdgeInsets::new(
        Length::px(1.0),
        Length::px(2.0),
        Length::px(3.0),
        Length::px(4.0),
    );
    let text_padding = EdgeInsets::all(Length::px(2.0));
    let ui = column((text::<Action>("A").padding(text_padding),)).padding(root_padding);

    let frame = surface_frame(&ui, LogicalSize::new(200.0, 100.0));
    let root = root_node(&frame)?;
    let label = surface_node(&frame, RuntimeNodeId::from_index(1))?;

    assert_rect_eq(
        root.bounds(),
        LogicalRect::from_xywh(0.0, 0.0, 200.0, 100.0),
    );
    assert_rect_eq(label.bounds(), LogicalRect::from_xywh(4.0, 1.0, 12.0, 24.0));

    Ok(())
}

#[test]
fn button_padding_expands_desired_size_before_minimum_constraints() -> Result<(), &'static str> {
    let ui = column((button::<Action>("12345678")
        .padding(EdgeInsets::symmetric(Length::px(10.0), Length::px(6.0))),));

    let frame = surface_frame(&ui, LogicalSize::new(200.0, 100.0));
    let button = surface_node(&frame, RuntimeNodeId::from_index(1))?;

    assert_rect_eq(
        button.bounds(),
        LogicalRect::from_xywh(0.0, 0.0, 84.0, 32.0),
    );

    Ok(())
}

#[test]
fn container_padding_expands_outer_size_and_offsets_children() -> Result<(), &'static str> {
    let ui = column((row((text::<Action>("A"), text::<Action>("B")))
        .gap(2.0)
        .padding(EdgeInsets::symmetric(Length::px(3.0), Length::px(4.0))),));

    let frame = surface_frame(&ui, LogicalSize::new(200.0, 100.0));
    let row = surface_node(&frame, RuntimeNodeId::from_index(1))?;
    let first = surface_node(&frame, RuntimeNodeId::from_index(2))?;
    let second = surface_node(&frame, RuntimeNodeId::from_index(3))?;

    assert_rect_eq(row.bounds(), LogicalRect::from_xywh(0.0, 0.0, 24.0, 28.0));
    assert_rect_eq(first.bounds(), LogicalRect::from_xywh(3.0, 4.0, 8.0, 20.0));
    assert_rect_eq(
        second.bounds(),
        LogicalRect::from_xywh(13.0, 4.0, 8.0, 20.0),
    );

    Ok(())
}

#[test]
fn token_resolved_padding_matches_literal_geometry() {
    let padding = EdgeInsets::new(
        Length::px(2.0),
        Length::px(4.0),
        Length::px(6.0),
        Length::px(8.0),
    );
    let literal = column((text::<Action>("Token").padding(padding),));
    let token = column((text::<Action>("Token").padding(SpacingToken::new("space.test")),));
    let tokens = StyleTokens::new().with_spacing("space.test", padding);
    let context = SurfaceBuildContext::new(&tokens);

    let literal_frame = publish_surface(&literal, LogicalSize::new(200.0, 100.0), &context)
        .into_parts()
        .0;
    let token_frame = publish_surface(&token, LogicalSize::new(200.0, 100.0), &context)
        .into_parts()
        .0;

    assert_eq!(
        literal_frame
            .nodes()
            .iter()
            .map(SurfaceNode::bounds)
            .collect::<Vec<_>>(),
        token_frame
            .nodes()
            .iter()
            .map(SurfaceNode::bounds)
            .collect::<Vec<_>>(),
    );
}

#[test]
fn missing_padding_token_uses_zero_insets_and_preserves_diagnostics() -> Result<(), &'static str> {
    let missing = SpacingToken::new("space.missing");
    let ui = column((button::<Action>("A").padding(missing.clone()),));
    let tokens = StyleTokens::new();
    let context = SurfaceBuildContext::new(&tokens);
    let publication = publish_surface(&ui, LogicalSize::new(200.0, 100.0), &context);
    let button = publication
        .frame()
        .node(RuntimeNodeId::from_index(1))
        .ok_or("expected button surface node")?;
    let style = publication
        .style_report()
        .node(RuntimeNodeId::from_index(1))
        .ok_or("expected button style node")?;

    assert_rect_eq(
        button.bounds(),
        LogicalRect::from_xywh(0.0, 0.0, 64.0, 32.0),
    );
    assert_eq!(
        style.unresolved_tokens(),
        &[UnresolvedStyleToken::Padding(missing)],
    );

    Ok(())
}

#[test]
fn hit_testing_uses_padded_outer_bounds() {
    let ui = column((text::<Action>("A").padding(EdgeInsets::all(Length::px(10.0))),));
    let frame = surface_frame(&ui, LogicalSize::new(200.0, 100.0));

    assert_eq!(
        frame.hit_test_id(runenui_runtime::LogicalPoint::new(25.0, 35.0)),
        Some(RuntimeNodeId::from_index(1)),
    );
}
