use runenui_core::{
    Brush, Color, GradientStop, GradientStops, IntoEffects, LinearGradient, LogicalPoint,
    NoHostProtocol, StyleEnvironment, UiApp, UnitInterval, View, text,
};
use runenui_runtime::{AppRuntime, LayoutConstraints, SurfaceBuildContext};

struct BackgroundBrushApp;

impl UiApp for BackgroundBrushApp {
    type State = ();
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root((): &Self::State) -> impl View<Self::Action> {
        text("background brush")
            .background(background_brush())
            .key("root")
    }

    fn update(
        (): &mut Self::State,
        (): Self::Action,
    ) -> impl IntoEffects<Self::Action, Self::HostProtocol> {
    }
}

fn background_brush() -> Brush {
    let stops = GradientStops::new(vec![
        GradientStop::new(UnitInterval::ZERO, Color::BLACK),
        GradientStop::new(UnitInterval::ONE, Color::WHITE),
    ])
    .unwrap_or_else(|_| unreachable!("controlled gradient stops are valid"));
    let start = LogicalPoint::new(0.0, 0.0)
        .unwrap_or_else(|_| unreachable!("controlled gradient start is finite"));
    let end = LogicalPoint::new(32.0, 0.0)
        .unwrap_or_else(|_| unreachable!("controlled gradient end is finite"));
    Brush::Linear(
        LinearGradient::new(start, end, stops)
            .unwrap_or_else(|_| unreachable!("controlled gradient is nondegenerate")),
    )
}

#[test]
fn non_solid_background_survives_style_resolution_and_paint_publication() {
    let expected = background_brush();
    let mut runtime = AppRuntime::<BackgroundBrushApp>::mount(());
    let environment = StyleEnvironment::default();
    let publication = runtime
        .publish_surface(&SurfaceBuildContext::new(
            &environment,
            LayoutConstraints::unbounded(),
        ))
        .unwrap_or_else(|_| unreachable!("controlled brush-background publication is admitted"));

    let style = publication
        .style_report()
        .nodes()
        .first()
        .unwrap_or_else(|| unreachable!("published root has a style record"));
    assert_eq!(style.computed_style().background(), Some(&expected));

    let published_brush = publication
        .paint_publication()
        .scene()
        .items()
        .iter()
        .find_map(|item| item.primitive().brush())
        .unwrap_or_else(|| unreachable!("background contributes one generic fill brush"));
    assert_eq!(published_brush, &expected);
}
