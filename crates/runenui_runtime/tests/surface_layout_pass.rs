use runenui_core::prelude::{
    EdgeInsets, Length, SpacingToken, StyleTokens, UnresolvedStyleToken, button, column, row, text,
};
use runenui_runtime::prelude::{
    LayoutConstraints, LogicalRect, LogicalSize, MeasurementProvider, RuntimeNodeId,
    SurfaceBuildContext, SurfaceFrame, SurfaceNode, SurfaceNodeKind, TextMeasurement,
    TextMeasurementKind, TextMeasurementRequest, publish_surface,
};
use std::cell::RefCell;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Action {
    Press,
}

fn surface_frame<Action>(root: &runenui_core::Element<Action>, size: LogicalSize) -> SurfaceFrame {
    let tokens = StyleTokens::new();
    let context = SurfaceBuildContext::tight(&tokens, size);
    publish_surface(root, &context).into_parts().0
}

fn surface_frame_with_provider<Action>(
    root: &runenui_core::Element<Action>,
    root_constraints: LayoutConstraints,
    provider: &dyn MeasurementProvider,
) -> SurfaceFrame {
    let tokens = StyleTokens::new();
    let context =
        SurfaceBuildContext::new(&tokens, root_constraints).with_measurement_provider(provider);
    publish_surface(root, &context).into_parts().0
}

#[derive(Clone, Copy, Debug)]
struct KindSizedProvider {
    text: LogicalSize,
    button_label: LogicalSize,
}

impl KindSizedProvider {
    const fn new(text: LogicalSize, button_label: LogicalSize) -> Self {
        Self { text, button_label }
    }
}

impl MeasurementProvider for KindSizedProvider {
    fn measure_text(&self, request: &TextMeasurementRequest<'_>) -> TextMeasurement {
        let size = match request.kind() {
            TextMeasurementKind::Text => self.text,
            TextMeasurementKind::ButtonLabel => self.button_label,
        };

        TextMeasurement::new(request.constraints().constrain(size))
    }
}

#[derive(Clone, Debug, PartialEq)]
struct RecordedTextRequest {
    content: String,
    constraints: LayoutConstraints,
    node_id: Option<RuntimeNodeId>,
    kind: TextMeasurementKind,
}

#[derive(Debug)]
struct RecordingProvider {
    requests: RefCell<Vec<RecordedTextRequest>>,
    size: LogicalSize,
}

impl RecordingProvider {
    const fn new(size: LogicalSize) -> Self {
        Self {
            requests: RefCell::new(Vec::new()),
            size,
        }
    }

    fn requests(&self) -> Vec<RecordedTextRequest> {
        self.requests.borrow().clone()
    }
}

impl MeasurementProvider for RecordingProvider {
    fn measure_text(&self, request: &TextMeasurementRequest<'_>) -> TextMeasurement {
        self.requests.borrow_mut().push(RecordedTextRequest {
            content: request.content().to_owned(),
            constraints: request.constraints(),
            node_id: request.node_id(),
            kind: request.kind(),
        });

        TextMeasurement::new(request.constraints().constrain(self.size))
    }
}

#[derive(Clone, Copy, Debug)]
struct InvalidProvider;

impl MeasurementProvider for InvalidProvider {
    fn measure_text(&self, request: &TextMeasurementRequest<'_>) -> TextMeasurement {
        let size = match request.kind() {
            TextMeasurementKind::Text => LogicalSize::new(f32::NAN, f32::INFINITY),
            TextMeasurementKind::ButtonLabel => LogicalSize::new(-20.0, f32::NEG_INFINITY),
        };

        TextMeasurement::new(size)
    }
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
fn tight_constraints_preserve_fixed_size_root_behavior() -> Result<(), &'static str> {
    let ui = column((text::<Action>("A"), button::<Action>("B"))).gap(8.0);

    let frame = surface_frame(&ui, LogicalSize::new(200.0, 100.0));
    let root = root_node(&frame)?;

    assert_eq!(frame.size(), LogicalSize::new(200.0, 100.0));
    assert_rect_eq(
        root.bounds(),
        LogicalRect::from_xywh(0.0, 0.0, 200.0, 100.0),
    );

    Ok(())
}

#[test]
fn loose_constraints_allow_intrinsic_root_shrink_to_fit() -> Result<(), &'static str> {
    let ui = column((text::<Action>("A"), button::<Action>("B"))).gap(8.0);
    let tokens = StyleTokens::new();
    let context = SurfaceBuildContext::new(
        &tokens,
        LayoutConstraints::loose(LogicalSize::new(200.0, 100.0)),
    );

    let frame = publish_surface(&ui, &context).into_parts().0;
    let root = root_node(&frame)?;

    assert_eq!(frame.size(), LogicalSize::new(64.0, 60.0));
    assert_rect_eq(root.bounds(), LogicalRect::from_xywh(0.0, 0.0, 64.0, 60.0));

    Ok(())
}

#[test]
fn unbounded_constraints_preserve_intrinsic_root_size() -> Result<(), &'static str> {
    let ui = row((text::<Action>("AB"), button::<Action>("OK"))).gap(4.0);
    let tokens = StyleTokens::new();
    let context = SurfaceBuildContext::new(&tokens, LayoutConstraints::unbounded());

    let frame = publish_surface(&ui, &context).into_parts().0;
    let root = root_node(&frame)?;

    assert_eq!(frame.size(), LogicalSize::new(84.0, 32.0));
    assert_rect_eq(root.bounds(), LogicalRect::from_xywh(0.0, 0.0, 84.0, 32.0));

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
fn custom_provider_changes_standalone_text_geometry() -> Result<(), &'static str> {
    let provider =
        KindSizedProvider::new(LogicalSize::new(42.0, 16.0), LogicalSize::new(8.0, 20.0));
    let ui = column((text::<Action>("ABC"),));

    let frame = surface_frame_with_provider(
        &ui,
        LayoutConstraints::tight(LogicalSize::new(100.0, 100.0)),
        &provider,
    );
    let text = surface_node(&frame, RuntimeNodeId::from_index(1))?;

    assert_rect_eq(text.bounds(), LogicalRect::from_xywh(0.0, 0.0, 42.0, 16.0));

    Ok(())
}

#[test]
fn custom_provider_changes_button_label_geometry() -> Result<(), &'static str> {
    let provider =
        KindSizedProvider::new(LogicalSize::new(8.0, 20.0), LogicalSize::new(72.0, 18.0));
    let ui = column((button::<Action>("ABCD"),));

    let frame = surface_frame_with_provider(
        &ui,
        LayoutConstraints::tight(LogicalSize::new(100.0, 100.0)),
        &provider,
    );
    let button = surface_node(&frame, RuntimeNodeId::from_index(1))?;

    assert_rect_eq(
        button.bounds(),
        LogicalRect::from_xywh(0.0, 0.0, 72.0, 32.0),
    );

    Ok(())
}

#[test]
fn text_and_button_requests_include_runtime_id_kind_and_constraints() {
    let provider = RecordingProvider::new(LogicalSize::new(10.0, 10.0));
    let ui = column((text::<Action>("ABC"), button::<Action>("ABCD"))).gap(2.0);

    let _frame = surface_frame_with_provider(
        &ui,
        LayoutConstraints::tight(LogicalSize::new(100.0, 100.0)),
        &provider,
    );
    let requests = provider.requests();

    assert!(requests.iter().any(|request| {
        request.content == "ABC"
            && request.node_id == Some(RuntimeNodeId::from_index(1))
            && request.kind == TextMeasurementKind::Text
            && request.constraints == LayoutConstraints::unbounded()
    }));
    assert!(requests.iter().any(|request| {
        request.content == "ABCD"
            && request.node_id == Some(RuntimeNodeId::from_index(2))
            && request.kind == TextMeasurementKind::ButtonLabel
            && request.constraints == LayoutConstraints::unbounded()
    }));
}

#[test]
fn button_size_composes_label_measurement_padding_and_minimum_policy() -> Result<(), &'static str> {
    let provider =
        KindSizedProvider::new(LogicalSize::new(8.0, 20.0), LogicalSize::new(70.0, 20.0));
    let ui =
        column((button::<Action>("ABCD")
            .padding(EdgeInsets::symmetric(Length::px(5.0), Length::px(7.0))),));

    let frame = surface_frame_with_provider(
        &ui,
        LayoutConstraints::tight(LogicalSize::new(120.0, 120.0)),
        &provider,
    );
    let button = surface_node(&frame, RuntimeNodeId::from_index(1))?;

    assert_rect_eq(
        button.bounds(),
        LogicalRect::from_xywh(0.0, 0.0, 80.0, 34.0),
    );

    Ok(())
}

#[test]
fn provider_output_is_sanitized_before_frame_geometry() {
    let ui = row((text::<Action>("Bad"), button::<Action>("Bad"))).gap(4.0);

    let frame = surface_frame_with_provider(&ui, LayoutConstraints::unbounded(), &InvalidProvider);

    for node in frame.nodes() {
        let bounds = node.bounds();
        assert!(bounds.x().is_finite() && bounds.x() >= 0.0);
        assert!(bounds.y().is_finite() && bounds.y() >= 0.0);
        assert!(bounds.width().is_finite() && bounds.width() >= 0.0);
        assert!(bounds.height().is_finite() && bounds.height() >= 0.0);
    }
}

#[test]
fn default_provider_preserves_deterministic_intrinsic_text_and_button_sizes()
-> Result<(), &'static str> {
    let ui = column((text::<Action>("ABC"), button::<Action>("ABCD"))).gap(2.0);

    let frame = surface_frame(&ui, LogicalSize::new(100.0, 100.0));
    let text = surface_node(&frame, RuntimeNodeId::from_index(1))?;
    let button = surface_node(&frame, RuntimeNodeId::from_index(2))?;

    assert_rect_eq(text.bounds(), LogicalRect::from_xywh(0.0, 0.0, 24.0, 20.0));
    assert_rect_eq(
        button.bounds(),
        LogicalRect::from_xywh(0.0, 22.0, 64.0, 32.0),
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
    let context = SurfaceBuildContext::tight(&tokens, LogicalSize::new(200.0, 100.0));

    let literal_frame = publish_surface(&literal, &context).into_parts().0;
    let token_frame = publish_surface(&token, &context).into_parts().0;

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
    let context = SurfaceBuildContext::tight(&tokens, LogicalSize::new(200.0, 100.0));
    let publication = publish_surface(&ui, &context);
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
