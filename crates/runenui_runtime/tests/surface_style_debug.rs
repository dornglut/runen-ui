use runenui_core::{
    Color, ColorToken, ComputedStyle, EdgeInsets, Element, ElementId, Length, Radius, RadiusToken,
    SpacingToken, StyleTokens, UnresolvedStyleToken, button, column, text,
};
use runenui_runtime::{
    LogicalSize, RuntimeNodeId, SurfaceStyleNode, layout_surface,
    render_debug_surface_style_report, resolve_surface_style_report,
};

#[derive(Clone, Debug, Eq, PartialEq)]
enum Action {
    Save,
}

#[test]
fn surface_style_report_exposes_resolved_style_by_runtime_node() {
    let padding = EdgeInsets::all(Length::px(8.0));
    let radius = Radius::all(Length::px(4.0));
    let root = column((
        text("Title")
            .id("title")
            .foreground(ColorToken::new("color.text.primary")),
        button("Save")
            .id("save")
            .background(ColorToken::new("color.action.primary"))
            .padding(SpacingToken::new("space.2"))
            .radius(RadiusToken::new("radius.control"))
            .on_press(Action::Save),
    ));
    let frame = layout_surface(&root, LogicalSize::new(320.0, 200.0));
    let tokens = StyleTokens::new()
        .with_color("color.text.primary", Color::WHITE)
        .with_color("color.action.primary", Color::BLACK)
        .with_spacing("space.2", padding)
        .with_radius("radius.control", radius);

    let report = resolve_surface_style_report(&root, &frame, &tokens);
    let title = report.node(RuntimeNodeId::from_index(1));
    let save = report.node(RuntimeNodeId::from_index(2));

    assert_eq!(report.nodes().len(), frame.nodes().len());
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
                .with_background(Color::BLACK)
                .with_padding(padding)
                .with_radius(radius),
        )
    );
    assert_eq!(save.map(SurfaceStyleNode::is_fully_resolved), Some(true));
}

#[test]
fn surface_style_report_preserves_missing_token_diagnostics() {
    let root = column((button("Save")
        .id("save")
        .background(ColorToken::new("color.action.primary"))
        .radius(RadiusToken::new("radius.control"))
        .on_press(Action::Save),));
    let frame = layout_surface(&root, LogicalSize::new(320.0, 200.0));
    let tokens = StyleTokens::new().with_color("color.action.primary", Color::BLACK);

    let report = resolve_surface_style_report(&root, &frame, &tokens);
    let save = report.node(RuntimeNodeId::from_index(1));
    let expected_unresolved = [UnresolvedStyleToken::Radius(RadiusToken::new(
        "radius.control",
    ))];

    assert_eq!(
        save.map(SurfaceStyleNode::computed_style),
        Some(ComputedStyle::EMPTY.with_background(Color::BLACK))
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
    let frame = layout_surface(&root, LogicalSize::new(320.0, 200.0));
    let tokens = StyleTokens::new().with_color("color.text.primary", Color::WHITE);

    let report = resolve_surface_style_report(&root, &frame, &tokens);
    let output = render_debug_surface_style_report(&report);

    assert!(output.contains("surface styles nodes=2"));
    assert!(output.contains("style id=1 authored=title"));
    assert!(output.contains("computed=ComputedStyle"));
    assert!(output.contains("unresolved=[]"));
}
