use runenui_core::{
    Color, EdgeInsets, Element, IntoElement, LogicalLength, StyleTokens, button, children,
    color_token, column, row, text,
};
use runenui_runtime::{
    DeterministicMeasurementProvider, LayoutConstraints, LogicalPoint, LogicalSize,
    SurfaceBuildContext, TextMeasurementKind, publish_surface, render_debug_surface_frame,
    render_debug_surface_style_report,
};

fn length(value: f32) -> LogicalLength {
    LogicalLength::new(value).unwrap_or_default()
}
fn size(width: f32, height: f32) -> LogicalSize {
    LogicalSize::new(length(width), length(height))
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
        render_debug_surface_frame(publication.frame()).contains("kind=button \"A\" enabled=true")
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
