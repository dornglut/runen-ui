use runenui_core::{
    Color, EdgeInsets, Element, LogicalLength, StyleTokens, View, button, children, color_token,
    column, row, text,
};
use runenui_runtime::{
    AppRuntime, DeterministicMeasurementProvider, LayoutConstraints, LogicalPoint, LogicalSize,
    MeasurementProvider, SurfaceBuildContext, TextMeasurement, TextMeasurementRequest, UiApp,
    render_debug_surface_frame, render_debug_surface_style_report,
};

fn length(value: f32) -> LogicalLength {
    LogicalLength::new(value).unwrap_or_else(|_| unreachable!())
}

fn size(width: f32, height: f32) -> LogicalSize {
    LogicalSize::try_new(width, height).unwrap_or_else(|_| unreachable!())
}

struct CompositeApp;

impl UiApp for CompositeApp {
    type State = ();
    type Action = ();

    fn root((): &Self::State) -> Element<Self::Action> {
        column(children![
            text("Title").id("title"),
            row(children![button("A"), button("B").disabled()]).gap(8_u16),
        ])
        .gap(4_u16)
        .into_element()
    }

    fn update((): &mut Self::State, (): Self::Action) {}
}

#[test]
fn built_in_row_column_measure_arrange_hit_and_debug_through_mounted_publication() {
    let mut runtime = AppRuntime::<CompositeApp>::mount(());
    let tokens = StyleTokens::new();
    let provider = DeterministicMeasurementProvider::new(length(10.0), length(20.0));
    let publication = runtime.publish_surface(
        &SurfaceBuildContext::new(&tokens, LayoutConstraints::loose(size(300.0, 200.0)))
            .with_measurement_provider(&provider),
    );

    assert_eq!(publication.frame().nodes().len(), 5);
    assert_eq!(publication.layout_report().nodes().len(), 5);
    assert_eq!(publication.style_report().nodes().len(), 5);
    assert!(
        !publication
            .layout_report()
            .root()
            .unwrap_or_else(|| unreachable!())
            .overflow()
            .any()
    );
    let hit = publication
        .frame()
        .hit_test(LogicalPoint::new(1.0, 25.0).unwrap_or_else(|_| unreachable!()))
        .unwrap_or_else(|| unreachable!());
    assert_ne!(
        hit.id(),
        publication
            .frame()
            .root()
            .unwrap_or_else(|| unreachable!())
            .id()
    );
    assert!(
        render_debug_surface_frame(publication.frame())
            .contains("semantic=button \"A\" enabled=true actionable=false")
    );
}

struct StyledApp;

impl UiApp for StyledApp {
    type State = ();
    type Action = ();

    fn root((): &Self::State) -> Element<Self::Action> {
        text("X")
            .foreground(color_token!("color.text"))
            .padding(EdgeInsets::all(length(6.0)))
            .into_element()
    }

    fn update((): &mut Self::State, (): Self::Action) {}
}

#[test]
fn resolved_padding_and_token_provenance_align_in_one_mounted_publication() {
    let mut tokens = StyleTokens::new();
    tokens
        .define_color(color_token!("color.text"), Color::WHITE)
        .unwrap_or_else(|_| unreachable!());
    let mut runtime = AppRuntime::<StyledApp>::mount(());
    let publication = runtime.publish_surface(&SurfaceBuildContext::new(
        &tokens,
        LayoutConstraints::unbounded(),
    ));
    let root = publication.frame().root().unwrap_or_else(|| unreachable!());
    assert_eq!(root.computed_style().foreground(), Some(Color::WHITE));
    assert!((root.bounds().width() - 20.0).abs() <= f32::EPSILON);
    assert_eq!(
        publication.style_report().nodes()[0]
            .computed_style()
            .padding(),
        Some(EdgeInsets::all(length(6.0)))
    );
    assert!(
        render_debug_surface_style_report(publication.style_report()).contains("ResolvedToken")
    );
}

struct BoundaryMeasurementProvider;

impl MeasurementProvider for BoundaryMeasurementProvider {
    fn cache_identity(&self) -> u64 {
        0x424f_554e_4441_5259
    }

    fn cache_revision(&self) -> u64 {
        1
    }

    fn measure_text(&self, request: &TextMeasurementRequest<'_>) -> TextMeasurement {
        TextMeasurement::new(match request.content() {
            "huge-width" => size(f32::MAX, 1.0),
            "huge-height" => size(1.0, f32::MAX),
            _ => size(1.0, 1.0),
        })
    }
}

#[derive(Clone, Copy, Debug)]
enum BoundaryCase {
    Horizontal,
    Vertical,
    Padded,
}

struct BoundaryApp;

impl UiApp for BoundaryApp {
    type State = BoundaryCase;
    type Action = ();

    fn root(state: &Self::State) -> Element<Self::Action> {
        match state {
            BoundaryCase::Horizontal => row(children![
                text("small"),
                text("huge-width"),
                text("after-one"),
                text("after-two"),
            ])
            .into_element(),
            BoundaryCase::Vertical => column(children![
                text("small"),
                text("huge-height"),
                text("after-one"),
                text("after-two"),
            ])
            .into_element(),
            BoundaryCase::Padded => text("small")
                .padding(EdgeInsets::all(LogicalLength::MAX))
                .into_element(),
        }
    }

    fn update(_: &mut Self::State, (): Self::Action) {}
}

#[test]
fn derived_geometry_saturates_without_non_finite_bounds_or_maxima() {
    let tokens = StyleTokens::new();
    let provider = BoundaryMeasurementProvider;
    for case in [
        BoundaryCase::Horizontal,
        BoundaryCase::Vertical,
        BoundaryCase::Padded,
    ] {
        let mut runtime = AppRuntime::<BoundaryApp>::mount(case);
        let publication = runtime.publish_surface(
            &SurfaceBuildContext::new(&tokens, LayoutConstraints::unbounded())
                .with_measurement_provider(&provider),
        );
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
            assert!(node.constrained_outer_size().width().is_finite());
            assert!(node.constrained_outer_size().height().is_finite());
        }
    }
}

#[test]
fn invalid_dynamic_sizes_and_tight_constraint_overflow_are_explicit() {
    assert!(LogicalSize::try_new(f32::NAN, 10.0).is_err());
    assert!(LogicalSize::try_new(-1.0, 10.0).is_err());
    let mut runtime = AppRuntime::<CompositeApp>::mount(());
    let tokens = StyleTokens::new();
    let publication = runtime.publish_surface(&SurfaceBuildContext::new(
        &tokens,
        LayoutConstraints::loose(size(2.0, 2.0)),
    ));
    assert!(
        publication
            .layout_report()
            .root()
            .is_some_and(|node| node.overflow().any())
    );
}
