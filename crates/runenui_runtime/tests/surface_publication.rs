use runenui_core::{
    Color, EdgeInsets, Element, LogicalLength, StyleTokens, View, button, children, color_token,
    column, row, text,
};
use runenui_runtime::{
    DeterministicMeasurementProvider, LayoutConstraints, LogicalPoint, LogicalSize,
    MeasurementProvider, SurfaceBuildContext, SurfaceNode, TextMeasurement, TextMeasurementKind,
    TextMeasurementRequest, publish_surface, render_debug_surface_frame,
    render_debug_surface_style_report,
};

fn length(value: f32) -> LogicalLength {
    LogicalLength::new(value).unwrap_or_default()
}
fn size(width: f32, height: f32) -> LogicalSize {
    LogicalSize::new(length(width), length(height))
}

fn assert_float_bits(actual: f32, expected: f32) {
    assert_eq!(actual.to_bits(), expected.to_bits());
}

#[test]
fn row_column_measurement_arrangement_hit_and_debug_regress() -> Result<(), &'static str> {
    let root: Element<()> = column(children![
        text("Title").id("title"),
        row(children![button("A"), button("B").disabled()]).gap(8_u16),
    ])
    .gap(4_u16)
    .into_element();
    let tokens = StyleTokens::new();
    let provider = DeterministicMeasurementProvider::new(length(10.0), length(20.0));
    let context = SurfaceBuildContext::new(&tokens, LayoutConstraints::loose(size(300.0, 200.0)))
        .with_measurement_provider(&provider);
    let publication = publish_surface(&root, &context);
    assert_eq!(publication.frame().nodes().len(), 5);
    assert_eq!(publication.layout_report().nodes().len(), 5);
    assert_eq!(publication.style_report().nodes().len(), 5);
    assert!(
        !publication
            .layout_report()
            .root()
            .ok_or("root")?
            .overflow()
            .any()
    );
    let hit = publication
        .frame()
        .hit_test(LogicalPoint::new(1.0, 25.0).map_err(|_| "point")?)
        .ok_or("hit")?;
    assert_ne!(hit.id(), publication.frame().root().ok_or("root")?.id());
    assert!(
        render_debug_surface_frame(publication.frame())
            .contains("semantic=button \"A\" enabled=true actionable=false")
    );
    Ok(())
}

#[test]
fn resolved_padding_and_token_provenance_share_one_publication()
-> Result<(), Box<dyn std::error::Error>> {
    let padding = EdgeInsets::all(length(6.0));
    let color = color_token!("color.text");
    let mut tokens = StyleTokens::new();
    tokens.define_color(color.clone(), Color::WHITE)?;
    let root: Element<()> = text("X").foreground(color).padding(padding).into_element();
    let publication = publish_surface(
        &root,
        &SurfaceBuildContext::new(&tokens, LayoutConstraints::unbounded()),
    );
    let node = publication.frame().root().ok_or("frame root")?;
    assert_eq!(node.computed_style().foreground(), Some(Color::WHITE));
    assert!((node.bounds().width() - 20.0).abs() <= f32::EPSILON);
    assert_eq!(
        publication
            .style_report()
            .root_style()?
            .computed_style()
            .padding(),
        Some(padding)
    );
    assert!(
        render_debug_surface_style_report(publication.style_report()).contains("ResolvedToken")
    );
    Ok(())
}

trait RootStyle {
    fn root_style(&self) -> Result<&runenui_runtime::SurfaceStyleNode, &'static str>;
}
impl RootStyle for runenui_runtime::SurfaceStyleReport {
    fn root_style(&self) -> Result<&runenui_runtime::SurfaceStyleNode, &'static str> {
        self.nodes().first().ok_or("style root")
    }
}

#[test]
fn invalid_dynamic_sizes_and_overflow_are_explicit() {
    assert!(LogicalSize::try_new(f32::NAN, 10.0).is_err());
    assert!(LogicalSize::try_new(-1.0, 10.0).is_err());
    let root: Element<()> = button("Too small").into_element();
    let tokens = StyleTokens::new();
    let publication = publish_surface(
        &root,
        &SurfaceBuildContext::new(&tokens, LayoutConstraints::loose(size(2.0, 2.0))),
    );
    assert!(
        publication
            .layout_report()
            .root()
            .is_some_and(|node| node.overflow().any())
    );
    assert!(matches!(
        TextMeasurementKind::Text,
        TextMeasurementKind::Text
    ));
}

struct BoundaryMeasurementProvider;

impl MeasurementProvider for BoundaryMeasurementProvider {
    fn measure_text(&self, request: &TextMeasurementRequest<'_>) -> TextMeasurement {
        let measured = match request.content() {
            "huge-width" => size(f32::MAX, 1.0),
            "huge-height" => size(1.0, f32::MAX),
            _ => size(1.0, 1.0),
        };
        TextMeasurement::new(measured)
    }
}

#[test]
fn derived_geometry_saturates_and_never_publishes_non_finite_values()
-> Result<(), Box<dyn std::error::Error>> {
    let tokens = StyleTokens::new();
    let provider = BoundaryMeasurementProvider;
    let context = SurfaceBuildContext::new(&tokens, LayoutConstraints::unbounded())
        .with_measurement_provider(&provider);

    let horizontal: Element<()> = row(children![
        text("small"),
        text("huge-width"),
        text("after-one"),
        text("after-two"),
    ])
    .into_element();
    let horizontal = publish_surface(&horizontal, &context);
    let nodes = horizontal.frame().nodes();
    assert_float_bits(nodes[2].bounds().x(), 1.0);
    assert_float_bits(nodes[2].bounds().width(), f32::MAX);
    assert_float_bits(nodes[2].bounds().max_x(), f32::MAX);
    assert_float_bits(nodes[3].bounds().x(), f32::MAX);
    assert_float_bits(nodes[3].bounds().width(), 1.0);
    assert_float_bits(nodes[3].bounds().max_x(), f32::MAX);
    assert_float_bits(nodes[4].bounds().x(), f32::MAX);

    let before_max = f32::from_bits(f32::MAX.to_bits() - 1);
    assert!(
        nodes[2]
            .bounds()
            .contains(LogicalPoint::new(before_max, 0.5)?)
    );
    assert!(
        !nodes[2]
            .bounds()
            .contains(LogicalPoint::new(f32::MAX, 0.5)?)
    );
    assert_eq!(
        horizontal
            .frame()
            .hit_test(LogicalPoint::new(before_max, 0.5)?)
            .map(SurfaceNode::id),
        Some(nodes[2].id())
    );

    let vertical: Element<()> = column(children![
        text("small"),
        text("huge-height"),
        text("after-one"),
        text("after-two"),
    ])
    .into_element();
    let vertical = publish_surface(&vertical, &context);
    let vertical_nodes = vertical.frame().nodes();
    assert_float_bits(vertical_nodes[2].bounds().y(), 1.0);
    assert_float_bits(vertical_nodes[2].bounds().height(), f32::MAX);
    assert_float_bits(vertical_nodes[2].bounds().max_y(), f32::MAX);
    assert_float_bits(vertical_nodes[3].bounds().y(), f32::MAX);
    assert_float_bits(vertical_nodes[3].bounds().height(), 1.0);
    assert_float_bits(vertical_nodes[3].bounds().max_y(), f32::MAX);

    let padded: Element<()> = text("small")
        .padding(EdgeInsets::all(LogicalLength::MAX))
        .into_element();
    let padded = publish_surface(&padded, &context);
    let padded_root = padded.frame().root().ok_or("padded root")?;
    assert_float_bits(padded_root.bounds().width(), f32::MAX);
    assert_float_bits(padded_root.bounds().height(), f32::MAX);

    for publication in [&horizontal, &vertical, &padded] {
        assert!(publication.frame().size().width().is_finite());
        assert!(publication.frame().size().height().is_finite());
        for node in publication.frame().nodes() {
            let bounds = node.bounds();
            assert!(bounds.x().is_finite());
            assert!(bounds.y().is_finite());
            assert!(bounds.width().is_finite());
            assert!(bounds.height().is_finite());
            assert!(bounds.max_x().is_finite());
            assert!(bounds.max_y().is_finite());
        }
        for node in publication.layout_report().nodes() {
            let size = node.constrained_outer_size();
            assert!(size.width().is_finite());
            assert!(size.height().is_finite());
        }
    }
    Ok(())
}
