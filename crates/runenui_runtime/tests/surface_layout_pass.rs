use runenui_core::prelude::{
    EdgeInsets, Length, SpacingToken, StyleTokens, UnresolvedStyleToken, button, column, row, text,
};
use runenui_runtime::prelude::{
    AxisConstraints, LayoutConstraints, LogicalRect, LogicalSize, MeasurementProvider,
    RuntimeNodeId, SurfaceBuildContext, SurfaceFrame, SurfaceLayoutNode, SurfaceNode,
    SurfaceNodeKind, SurfacePublication, TextMeasurement, TextMeasurementKind,
    TextMeasurementRequest, publish_surface,
};
use std::cell::RefCell;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Action {
    Press,
}

fn surface_frame<Action>(root: &runenui_core::Element<Action>, size: LogicalSize) -> SurfaceFrame {
    let tokens = StyleTokens::new();
    let context = SurfaceBuildContext::tight(&tokens, size);
    let (frame, _style_report, _layout_report) = publish_surface(root, &context).into_parts();
    frame
}

fn surface_frame_with_provider<Action>(
    root: &runenui_core::Element<Action>,
    root_constraints: LayoutConstraints,
    provider: &dyn MeasurementProvider,
) -> SurfaceFrame {
    let (frame, _style_report, _layout_report) =
        surface_publication_with_provider(root, root_constraints, provider).into_parts();
    frame
}

fn surface_publication_with_provider<Action>(
    root: &runenui_core::Element<Action>,
    root_constraints: LayoutConstraints,
    provider: &dyn MeasurementProvider,
) -> SurfacePublication {
    let tokens = StyleTokens::new();
    let context =
        SurfaceBuildContext::new(&tokens, root_constraints).with_measurement_provider(provider);
    publish_surface(root, &context)
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

        TextMeasurement::new(size)
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

        TextMeasurement::new(self.size)
    }
}

#[derive(Debug)]
struct RecordingKindSizedProvider {
    requests: RefCell<Vec<RecordedTextRequest>>,
    text: LogicalSize,
    button_label: LogicalSize,
}

impl RecordingKindSizedProvider {
    const fn new(text: LogicalSize, button_label: LogicalSize) -> Self {
        Self {
            requests: RefCell::new(Vec::new()),
            text,
            button_label,
        }
    }

    fn requests(&self) -> Vec<RecordedTextRequest> {
        self.requests.borrow().clone()
    }
}

impl MeasurementProvider for RecordingKindSizedProvider {
    fn measure_text(&self, request: &TextMeasurementRequest<'_>) -> TextMeasurement {
        self.requests.borrow_mut().push(RecordedTextRequest {
            content: request.content().to_owned(),
            constraints: request.constraints(),
            node_id: request.node_id(),
            kind: request.kind(),
        });
        let size = match request.kind() {
            TextMeasurementKind::Text => self.text,
            TextMeasurementKind::ButtonLabel => self.button_label,
        };

        TextMeasurement::new(size)
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

    let (frame, _style_report, _layout_report) = publish_surface(&ui, &context).into_parts();
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

    let (frame, _style_report, _layout_report) = publish_surface(&ui, &context).into_parts();
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
fn nested_publication_measures_each_text_and_button_label_exactly_once() {
    let provider = RecordingProvider::new(LogicalSize::new(10.0, 10.0));
    let ui = column((
        text::<Action>("Root"),
        row((button::<Action>("A"), text::<Action>("Nested"))),
        button::<Action>("B"),
    ));

    let _frame = surface_frame_with_provider(
        &ui,
        LayoutConstraints::tight(LogicalSize::new(100.0, 100.0)),
        &provider,
    );
    let requests = provider.requests();
    let column_child_constraints =
        LayoutConstraints::new(AxisConstraints::loose(100.0), AxisConstraints::unbounded());

    assert_eq!(requests.len(), 4);
    assert_eq!(
        requests,
        vec![
            RecordedTextRequest {
                content: "Root".to_owned(),
                constraints: column_child_constraints,
                node_id: Some(RuntimeNodeId::from_index(1)),
                kind: TextMeasurementKind::Text,
            },
            RecordedTextRequest {
                content: "A".to_owned(),
                constraints: LayoutConstraints::unbounded(),
                node_id: Some(RuntimeNodeId::from_index(3)),
                kind: TextMeasurementKind::ButtonLabel,
            },
            RecordedTextRequest {
                content: "Nested".to_owned(),
                constraints: LayoutConstraints::unbounded(),
                node_id: Some(RuntimeNodeId::from_index(4)),
                kind: TextMeasurementKind::Text,
            },
            RecordedTextRequest {
                content: "B".to_owned(),
                constraints: column_child_constraints,
                node_id: Some(RuntimeNodeId::from_index(5)),
                kind: TextMeasurementKind::ButtonLabel,
            },
        ]
    );
}

#[test]
fn frame_style_and_layout_products_are_runtime_node_aligned() {
    let provider = RecordingProvider::new(LogicalSize::new(18.0, 12.0));
    let ui = column((
        text::<Action>("Title"),
        row((button::<Action>("A"), text::<Action>("Detail"))).gap(3.0),
    ))
    .gap(5.0);
    let publication = surface_publication_with_provider(
        &ui,
        LayoutConstraints::tight(LogicalSize::new(160.0, 90.0)),
        &provider,
    );
    let frame = publication.frame();
    let styles = publication.style_report();
    let layout = publication.layout_report();

    assert_eq!(frame.nodes().len(), styles.nodes().len());
    assert_eq!(frame.nodes().len(), layout.nodes().len());

    for ((frame_node, style_node), layout_node) in
        frame.nodes().iter().zip(styles.nodes()).zip(layout.nodes())
    {
        assert_eq!(frame_node.id(), style_node.id());
        assert_eq!(frame_node.id(), layout_node.id());
        assert_eq!(
            frame_node.bounds().size(),
            layout_node.constrained_outer_size()
        );
    }

    assert_eq!(
        layout.root().map(SurfaceLayoutNode::constrained_outer_size),
        Some(frame.size()),
    );
}

#[test]
fn column_propagates_loose_content_width_without_stretching_children() -> Result<(), &'static str> {
    let provider = RecordingKindSizedProvider::new(
        LogicalSize::new(20.0, 12.0),
        LogicalSize::new(200.0, 12.0),
    );
    let ui = column((text::<Action>("small"), button::<Action>("wide")))
        .padding(EdgeInsets::symmetric(Length::px(10.0), Length::px(0.0)));
    let publication = surface_publication_with_provider(
        &ui,
        LayoutConstraints::tight(LogicalSize::new(100.0, 100.0)),
        &provider,
    );
    let requests = provider.requests();
    let expected_constraints =
        LayoutConstraints::new(AxisConstraints::loose(80.0), AxisConstraints::unbounded());
    let text = publication
        .frame()
        .node(RuntimeNodeId::from_index(1))
        .ok_or("expected text frame node")?;
    let button = publication
        .frame()
        .node(RuntimeNodeId::from_index(2))
        .ok_or("expected button frame node")?;

    assert_eq!(requests.len(), 2);
    assert!(
        requests
            .iter()
            .all(|request| request.constraints == expected_constraints)
    );
    assert_eq!(text.bounds().size(), LogicalSize::new(20.0, 12.0));
    assert_eq!(button.bounds().size(), LogicalSize::new(80.0, 32.0));
    assert!(
        publication
            .layout_report()
            .node(RuntimeNodeId::from_index(2))
            .is_some_and(|node| node.overflow().width())
    );

    Ok(())
}

#[test]
fn row_propagates_loose_content_height_without_stretching_children() -> Result<(), &'static str> {
    let provider = RecordingKindSizedProvider::new(
        LogicalSize::new(10.0, 15.0),
        LogicalSize::new(10.0, 100.0),
    );
    let ui = row((text::<Action>("small"), button::<Action>("tall")))
        .padding(EdgeInsets::symmetric(Length::px(0.0), Length::px(10.0)));
    let publication = surface_publication_with_provider(
        &ui,
        LayoutConstraints::tight(LogicalSize::new(100.0, 60.0)),
        &provider,
    );
    let requests = provider.requests();
    let expected_constraints =
        LayoutConstraints::new(AxisConstraints::unbounded(), AxisConstraints::loose(40.0));
    let text = publication
        .frame()
        .node(RuntimeNodeId::from_index(1))
        .ok_or("expected text frame node")?;
    let button = publication
        .frame()
        .node(RuntimeNodeId::from_index(2))
        .ok_or("expected button frame node")?;

    assert_eq!(requests.len(), 2);
    assert!(
        requests
            .iter()
            .all(|request| request.constraints == expected_constraints)
    );
    assert_eq!(text.bounds().size(), LogicalSize::new(10.0, 15.0));
    assert_eq!(button.bounds().size(), LogicalSize::new(64.0, 40.0));
    assert!(
        publication
            .layout_report()
            .node(RuntimeNodeId::from_index(2))
            .is_some_and(|node| node.overflow().height())
    );

    Ok(())
}

#[test]
fn nested_columns_propagate_finite_cross_axis_after_each_padding_box() -> Result<(), &'static str> {
    let provider = RecordingProvider::new(LogicalSize::new(20.0, 10.0));
    let ui = column((column((text::<Action>("deep"),))
        .padding(EdgeInsets::symmetric(Length::px(5.0), Length::px(0.0))),))
    .padding(EdgeInsets::symmetric(Length::px(10.0), Length::px(0.0)));
    let publication = surface_publication_with_provider(
        &ui,
        LayoutConstraints::tight(LogicalSize::new(120.0, 100.0)),
        &provider,
    );
    let requests = provider.requests();
    let nested = publication
        .layout_report()
        .node(RuntimeNodeId::from_index(1))
        .ok_or("expected nested column layout node")?;

    assert_eq!(requests.len(), 1);
    assert_eq!(
        requests[0].constraints,
        LayoutConstraints::new(AxisConstraints::loose(90.0), AxisConstraints::unbounded(),)
    );
    assert_f32_eq(nested.outer_constraints().horizontal().min(), 0.0);
    assert_eq!(
        nested.outer_constraints().horizontal().max().finite(),
        Some(100.0)
    );
    assert_eq!(
        nested.content_constraints().horizontal().max().finite(),
        Some(90.0)
    );

    Ok(())
}

#[test]
fn column_main_axis_overflow_is_diagnostic_and_placement_remains_intrinsic()
-> Result<(), &'static str> {
    let provider = RecordingProvider::new(LogicalSize::new(20.0, 30.0));
    let ui = column((text::<Action>("A"), text::<Action>("B"))).gap(5.0);
    let publication = surface_publication_with_provider(
        &ui,
        LayoutConstraints::loose(LogicalSize::new(100.0, 50.0)),
        &provider,
    );
    let root = publication
        .layout_report()
        .root()
        .ok_or("expected root layout node")?;
    let second = publication
        .frame()
        .node(RuntimeNodeId::from_index(2))
        .ok_or("expected second text frame node")?;

    assert_eq!(publication.frame().size(), LogicalSize::new(20.0, 50.0));
    assert_eq!(root.desired_content_size(), LogicalSize::new(20.0, 65.0));
    assert!(!root.overflow().width());
    assert!(root.overflow().height());
    assert_eq!(
        second.bounds(),
        LogicalRect::from_xywh(0.0, 35.0, 20.0, 30.0)
    );

    Ok(())
}

#[test]
fn row_main_axis_overflow_is_diagnostic_and_final_width_is_constrained() -> Result<(), &'static str>
{
    let provider = RecordingProvider::new(LogicalSize::new(40.0, 20.0));
    let ui = row((text::<Action>("A"), text::<Action>("B"))).gap(5.0);
    let publication = surface_publication_with_provider(
        &ui,
        LayoutConstraints::loose(LogicalSize::new(70.0, 100.0)),
        &provider,
    );
    let root = publication
        .layout_report()
        .root()
        .ok_or("expected root layout node")?;

    assert_eq!(publication.frame().size(), LogicalSize::new(70.0, 20.0));
    assert_eq!(root.desired_content_size(), LogicalSize::new(85.0, 20.0));
    assert!(root.overflow().width());
    assert!(!root.overflow().height());

    Ok(())
}

#[test]
fn provider_cross_axis_overflow_is_clamped_and_reported_with_finite_sizes()
-> Result<(), &'static str> {
    let provider = RecordingProvider::new(LogicalSize::new(200.0, 20.0));
    let ui = column((text::<Action>("wide"),));
    let publication = surface_publication_with_provider(
        &ui,
        LayoutConstraints::loose(LogicalSize::new(50.0, 100.0)),
        &provider,
    );
    let text_layout = publication
        .layout_report()
        .node(RuntimeNodeId::from_index(1))
        .ok_or("expected text layout node")?;
    let text_frame = publication
        .frame()
        .node(RuntimeNodeId::from_index(1))
        .ok_or("expected text frame node")?;

    assert_eq!(
        text_layout.desired_content_size(),
        LogicalSize::new(200.0, 20.0)
    );
    assert_eq!(
        text_layout.constrained_outer_size(),
        LogicalSize::new(50.0, 20.0)
    );
    assert!(text_layout.overflow().width());
    assert_eq!(
        text_frame.bounds().size(),
        text_layout.constrained_outer_size()
    );

    for node in publication.layout_report().nodes() {
        for size in [
            node.desired_content_size(),
            node.desired_outer_size(),
            node.constrained_outer_size(),
        ] {
            assert!(size.width().is_finite() && size.width() >= 0.0);
            assert!(size.height().is_finite() && size.height() >= 0.0);
        }
    }

    Ok(())
}

#[test]
fn padding_pressure_collapses_content_constraints_and_reports_overflow() -> Result<(), &'static str>
{
    let provider = RecordingProvider::new(LogicalSize::new(10.0, 10.0));
    let ui = column::<Action>(Vec::new()).padding(EdgeInsets::all(Length::px(10.0)));
    let publication = surface_publication_with_provider(
        &ui,
        LayoutConstraints::loose(LogicalSize::new(15.0, 15.0)),
        &provider,
    );
    let root = publication
        .layout_report()
        .root()
        .ok_or("expected root layout node")?;

    assert_eq!(
        root.content_constraints(),
        LayoutConstraints::tight(LogicalSize::new(0.0, 0.0))
    );
    assert_eq!(root.desired_content_size(), LogicalSize::new(0.0, 0.0));
    assert_eq!(root.desired_outer_size(), LogicalSize::new(20.0, 20.0));
    assert_eq!(root.constrained_outer_size(), LogicalSize::new(15.0, 15.0));
    assert!(root.overflow().width());
    assert!(root.overflow().height());
    assert_eq!(publication.frame().size(), LogicalSize::new(15.0, 15.0));

    Ok(())
}

#[test]
fn button_minimum_pressure_is_visible_before_finite_outer_clamping() -> Result<(), &'static str> {
    let provider = KindSizedProvider::new(LogicalSize::new(1.0, 1.0), LogicalSize::new(1.0, 1.0));
    let ui = button::<Action>("A");
    let publication = surface_publication_with_provider(
        &ui,
        LayoutConstraints::loose(LogicalSize::new(40.0, 20.0)),
        &provider,
    );
    let root = publication
        .layout_report()
        .root()
        .ok_or("expected root button layout node")?;

    assert_eq!(root.desired_outer_size(), LogicalSize::new(64.0, 32.0));
    assert_eq!(root.constrained_outer_size(), LogicalSize::new(40.0, 20.0));
    assert!(root.overflow().width());
    assert!(root.overflow().height());

    Ok(())
}

#[test]
fn fitting_loose_and_unbounded_publications_report_no_overflow() {
    let fitting =
        KindSizedProvider::new(LogicalSize::new(20.0, 10.0), LogicalSize::new(20.0, 10.0));
    let large = KindSizedProvider::new(
        LogicalSize::new(500.0, 400.0),
        LogicalSize::new(500.0, 400.0),
    );
    let ui = text::<Action>("Text");
    let loose = surface_publication_with_provider(
        &ui,
        LayoutConstraints::loose(LogicalSize::new(100.0, 100.0)),
        &fitting,
    );
    let unbounded = surface_publication_with_provider(&ui, LayoutConstraints::unbounded(), &large);

    assert!(
        loose
            .layout_report()
            .root()
            .is_some_and(|node| !node.overflow().any())
    );
    assert!(
        unbounded
            .layout_report()
            .root()
            .is_some_and(|node| !node.overflow().any())
    );
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

    let publication =
        surface_publication_with_provider(&ui, LayoutConstraints::unbounded(), &InvalidProvider);

    for node in publication.frame().nodes() {
        let bounds = node.bounds();
        assert!(bounds.x().is_finite() && bounds.x() >= 0.0);
        assert!(bounds.y().is_finite() && bounds.y() >= 0.0);
        assert!(bounds.width().is_finite() && bounds.width() >= 0.0);
        assert!(bounds.height().is_finite() && bounds.height() >= 0.0);
    }
    for node in publication.layout_report().nodes() {
        for size in [
            node.desired_content_size(),
            node.desired_outer_size(),
            node.constrained_outer_size(),
        ] {
            assert!(size.width().is_finite() && size.width() >= 0.0);
            assert!(size.height().is_finite() && size.height() >= 0.0);
        }
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

    let (literal_frame, _literal_styles, _literal_layout) =
        publish_surface(&literal, &context).into_parts();
    let (token_frame, _token_styles, _token_layout) =
        publish_surface(&token, &context).into_parts();

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
