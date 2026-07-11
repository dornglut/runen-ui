use runenui_core::{
    Color, ColorToken, ComputedStyle, EdgeInsets, Element, ElementId, Length, Radius, RadiusToken,
    SpacingToken, StyleFieldProvenance, StyleProvenance, StyleTokens, UnresolvedStyleToken, button,
    column, text,
};
use runenui_runtime::{
    LogicalSize, RuntimeNodeId, SurfaceBuildContext, SurfaceNode, SurfaceStyleNode,
    publish_surface, render_debug_surface_style_report,
};

#[derive(Clone, Debug, Eq, PartialEq)]
enum Action {
    Save,
}

#[test]
fn surface_style_report_exposes_resolution_and_provenance_by_runtime_node() {
    let padding = EdgeInsets::all(Length::px(8.0));
    let radius = Radius::all(Length::px(4.0));
    let root = column((
        text("Title")
            .id("title")
            .foreground(ColorToken::new("color.text.primary")),
        button("Save")
            .id("save")
            .foreground(Color::WHITE)
            .background(ColorToken::new("color.action.primary"))
            .padding(SpacingToken::new("space.2"))
            .radius(RadiusToken::new("radius.control"))
            .on_press(Action::Save),
    ));
    let tokens = StyleTokens::new()
        .with_color("color.text.primary", Color::WHITE)
        .with_color("color.action.primary", Color::BLACK)
        .with_spacing("space.2", padding)
        .with_radius("radius.control", radius);

    let context = SurfaceBuildContext::new(&tokens);
    let publication = publish_surface(&root, LogicalSize::new(320.0, 200.0), &context);
    let frame = publication.frame();
    let report = publication.style_report();
    let title = report.node(RuntimeNodeId::from_index(1));
    let save = report.node(RuntimeNodeId::from_index(2));

    assert_eq!(report.nodes().len(), frame.nodes().len());
    assert_eq!(
        frame
            .node(RuntimeNodeId::from_index(1))
            .map(SurfaceNode::computed_style),
        title.map(SurfaceStyleNode::computed_style)
    );
    assert_eq!(
        frame
            .node(RuntimeNodeId::from_index(2))
            .map(SurfaceNode::computed_style),
        save.map(SurfaceStyleNode::computed_style)
    );
    assert_eq!(
        title
            .and_then(SurfaceStyleNode::authored_id)
            .map(ElementId::as_str),
        Some("title")
    );
    assert_eq!(
        title.map(SurfaceStyleNode::computed_style),
        Some(ComputedStyle::EMPTY.with_foreground(Color::WHITE))
    );
    assert_eq!(
        title
            .map(SurfaceStyleNode::provenance)
            .map(StyleProvenance::foreground),
        Some(&StyleFieldProvenance::ResolvedToken(ColorToken::new(
            "color.text.primary"
        )))
    );
    assert_eq!(title.map(SurfaceStyleNode::is_fully_resolved), Some(true));

    assert_eq!(
        save.and_then(SurfaceStyleNode::authored_id)
            .map(ElementId::as_str),
        Some("save")
    );
    assert_eq!(
        save.map(SurfaceStyleNode::computed_style),
        Some(
            ComputedStyle::EMPTY
                .with_foreground(Color::WHITE)
                .with_background(Color::BLACK)
                .with_padding(padding)
                .with_radius(radius),
        )
    );
    assert_eq!(
        save.map(SurfaceStyleNode::provenance)
            .map(StyleProvenance::foreground),
        Some(&StyleFieldProvenance::Literal)
    );
    assert_eq!(
        save.map(SurfaceStyleNode::provenance)
            .map(StyleProvenance::background),
        Some(&StyleFieldProvenance::ResolvedToken(ColorToken::new(
            "color.action.primary"
        )))
    );
    assert_eq!(
        save.map(SurfaceStyleNode::provenance)
            .map(StyleProvenance::padding),
        Some(&StyleFieldProvenance::ResolvedToken(SpacingToken::new(
            "space.2"
        )))
    );
    assert_eq!(
        save.map(SurfaceStyleNode::provenance)
            .map(StyleProvenance::radius),
        Some(&StyleFieldProvenance::ResolvedToken(RadiusToken::new(
            "radius.control"
        )))
    );
    assert_eq!(save.map(SurfaceStyleNode::is_fully_resolved), Some(true));
}

#[test]
fn surface_style_report_preserves_missing_token_provenance_and_diagnostics() {
    let root = column((button("Save")
        .id("save")
        .background(ColorToken::new("color.action.primary"))
        .radius(RadiusToken::new("radius.control"))
        .on_press(Action::Save),));
    let tokens = StyleTokens::new().with_color("color.action.primary", Color::BLACK);

    let context = SurfaceBuildContext::new(&tokens);
    let publication = publish_surface(&root, LogicalSize::new(320.0, 200.0), &context);
    let frame = publication.frame();
    let report = publication.style_report();
    let save = report.node(RuntimeNodeId::from_index(1));
    let expected_unresolved = [UnresolvedStyleToken::Radius(RadiusToken::new(
        "radius.control",
    ))];

    assert_eq!(
        frame
            .node(RuntimeNodeId::from_index(1))
            .map(SurfaceNode::computed_style),
        save.map(SurfaceStyleNode::computed_style)
    );
    assert_eq!(
        save.map(SurfaceStyleNode::computed_style),
        Some(ComputedStyle::EMPTY.with_background(Color::BLACK))
    );
    assert_eq!(
        save.map(SurfaceStyleNode::provenance)
            .map(StyleProvenance::background),
        Some(&StyleFieldProvenance::ResolvedToken(ColorToken::new(
            "color.action.primary"
        )))
    );
    assert_eq!(
        save.map(SurfaceStyleNode::provenance)
            .map(StyleProvenance::radius),
        Some(&StyleFieldProvenance::MissingToken(RadiusToken::new(
            "radius.control"
        )))
    );
    assert_eq!(
        save.map(SurfaceStyleNode::unresolved_tokens),
        Some(expected_unresolved.as_slice())
    );
    assert_eq!(save.map(SurfaceStyleNode::is_fully_resolved), Some(false));
}

#[test]
fn debug_surface_style_report_is_deterministic_text() {
    let root: Element<Action> = column((text("Title")
        .id("title")
        .foreground(ColorToken::new("color.text.primary")),));
    let tokens = StyleTokens::new().with_color("color.text.primary", Color::WHITE);

    let context = SurfaceBuildContext::new(&tokens);
    let publication = publish_surface(&root, LogicalSize::new(320.0, 200.0), &context);
    let frame = publication.frame();
    let report = publication.style_report();
    let output = render_debug_surface_style_report(report);
    let expected = concat!(
        "surface styles nodes=2\n",
        "style id=0 authored=- computed=ComputedStyle { foreground: None, background: None, padding: None, radius: None } ",
        "provenance=StyleProvenance { foreground: Absent, background: Absent, padding: Absent, radius: Absent } unresolved=[]\n",
        "style id=1 authored=title computed=ComputedStyle { foreground: Some(Color { red: 255, green: 255, blue: 255, alpha: 255 }), background: None, padding: None, radius: None } ",
        "provenance=StyleProvenance { foreground: ResolvedToken(ColorToken(TokenId(\"color.text.primary\"))), background: Absent, padding: Absent, radius: Absent } unresolved=[]\n",
    );

    assert_eq!(
        frame
            .nodes()
            .iter()
            .map(SurfaceNode::id)
            .collect::<Vec<_>>(),
        report
            .nodes()
            .iter()
            .map(SurfaceStyleNode::id)
            .collect::<Vec<_>>()
    );
    assert_eq!(output, expected);
    assert_eq!(render_debug_surface_style_report(report), expected);
}
