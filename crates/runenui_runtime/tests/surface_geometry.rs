#![allow(refining_impl_trait)]

use runenui_core::{
    Color, EdgeInsets, Element, ElementId, FontFamilyName, GenericFontFamily, LayoutBound,
    LayoutDimension, LayoutStyle, LogicalLength, NoHostProtocol, SemanticRole, StyleEnvironment,
    StyleTokens, UiApp, View, button, children, color_token, column, row, text,
};
use runenui_runtime::{
    AppRuntime, LayoutConstraints, LogicalPoint, LogicalSize, PumpBudget, SurfaceBuildContext,
    SurfacePublication, render_debug_surface_frame, render_debug_surface_style_report,
};

const CANTARELL: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../runenui_text/tests/fixtures/Cantarell-Regular.ttf"
));

fn length(value: f32) -> LogicalLength {
    LogicalLength::new(value).unwrap_or_else(|_| unreachable!())
}

fn size(width: f32, height: f32) -> LogicalSize {
    LogicalSize::try_new(width, height).unwrap_or_else(|_| unreachable!())
}

fn register_controlled_text<App: UiApp>(runtime: &mut AppRuntime<App>) {
    assert!(
        runtime
            .register_text_font_bytes(CANTARELL.to_vec())
            .unwrap_or_else(|_| unreachable!("controlled Cantarell fixture is registerable"))
            > 0
    );
    let family = FontFamilyName::new("Cantarell")
        .unwrap_or_else(|_| unreachable!("controlled family name is canonical"));
    assert!(
        runtime
            .set_text_generic_family_mapping(GenericFontFamily::SansSerif, &[family])
            .unwrap_or_else(|_| unreachable!("controlled generic mapping is valid"))
    );
}

fn publish<App: UiApp>(
    runtime: &mut AppRuntime<App>,
    context: &SurfaceBuildContext<'_>,
) -> SurfacePublication {
    runtime
        .publish_surface(context)
        .unwrap_or_else(|_| unreachable!("surface geometry publication is admitted"))
}

struct CompositeApp;

impl UiApp for CompositeApp {
    type State = ();
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root((): &Self::State) -> Element<Self::Action> {
        column(children![
            text("Title").id("title"),
            row(children![
                button("A").id("button-a").on_activate(|| ()),
                button("B").disabled()
            ])
            .gap(8_u16),
        ])
        .gap(4_u16)
        .into_element()
    }

    fn update((): &mut Self::State, (): Self::Action) {}
}

#[test]
fn built_in_row_column_measure_arrange_hit_and_debug_through_mounted_publication() {
    let mut runtime = AppRuntime::<CompositeApp>::mount(());
    register_controlled_text(&mut runtime);
    let style_environment = StyleEnvironment::default();
    let publication = publish(
        &mut runtime,
        &SurfaceBuildContext::new(
            &style_environment,
            LayoutConstraints::loose(size(300.0, 200.0)),
        ),
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

    let shaped_runs: Vec<_> = publication
        .paint_scene()
        .items()
        .iter()
        .filter_map(|item| item.primitive().as_shaped_text_run())
        .collect();
    assert!(!shaped_runs.is_empty());
    for run in shaped_runs {
        let shaped = publication
            .paint_scene()
            .shaped_text_resource(run.resource_ref())
            .unwrap_or_else(|| {
                unreachable!("published text run retains its exact logical resource")
            });
        assert_eq!(shaped.resource_ref(), run.resource_ref());
        assert!(!shaped.glyphs().is_empty());
    }

    let button_a_id = ElementId::new("button-a")
        .unwrap_or_else(|_| unreachable!("test authored identifier is canonical"));
    let button_a = publication
        .frame()
        .nodes()
        .iter()
        .find(|node| node.authored_id() == Some(&button_a_id))
        .unwrap_or_else(|| unreachable!("button A is present in the surface frame"));
    let bounds = button_a.bounds();
    let hit_point = LogicalPoint::new(
        bounds.width().mul_add(0.5, bounds.x()),
        bounds.height().mul_add(0.5, bounds.y()),
    )
    .unwrap_or_else(|_| unreachable!("button midpoint is finite"));
    assert_eq!(
        publication.hit_test_scene().target_at(hit_point),
        Some(button_a.id())
    );

    let semantic_button_a = publication
        .semantic_publication()
        .snapshot()
        .nodes()
        .iter()
        .find(|node| node.name() == Some("A"))
        .unwrap_or_else(|| unreachable!("button A is present in semantic publication"));
    assert_eq!(semantic_button_a.role(), SemanticRole::Button);

    let debug = render_debug_surface_frame(publication.frame());
    assert!(!debug.contains("semantics="));
}

struct StyledApp;

impl UiApp for StyledApp {
    type State = ();
    type Action = ();
    type HostProtocol = NoHostProtocol;

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
    let style_environment = StyleEnvironment::from_tokens(tokens);
    let mut runtime = AppRuntime::<StyledApp>::mount(());
    register_controlled_text(&mut runtime);
    let publication = publish(
        &mut runtime,
        &SurfaceBuildContext::new(&style_environment, LayoutConstraints::unbounded()),
    );
    let root = publication.frame().root().unwrap_or_else(|| unreachable!());
    assert_eq!(root.computed_style().foreground(), Some(Color::WHITE));
    assert!(root.bounds().width() > 12.0);
    assert!(root.bounds().height() > 12.0);
    assert_eq!(
        publication.style_report().nodes()[0]
            .computed_style()
            .padding(),
        Some(EdgeInsets::all(length(6.0)))
    );
    let text_run = publication
        .paint_scene()
        .items()
        .iter()
        .find_map(|item| item.primitive().as_shaped_text_run())
        .unwrap_or_else(|| unreachable!("logical text artifact contributes one paint run"));
    assert_eq!(text_run.foreground(), Color::WHITE);
    assert!(text_run.origin().x() >= 6.0);
    assert!(text_run.origin().y() >= 6.0);
    assert!(
        publication
            .paint_scene()
            .shaped_text_resource(text_run.resource_ref())
            .is_some()
    );
    assert!(
        render_debug_surface_style_report(publication.style_report()).contains("ResolvedToken")
    );
}

struct RetainedTextApp;

impl UiApp for RetainedTextApp {
    type State = &'static str;
    type Action = &'static str;
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> Element<Self::Action> {
        text(*state).into_element()
    }

    fn update(state: &mut Self::State, action: Self::Action) {
        *state = action;
    }
}

#[test]
fn retained_paint_publication_keeps_old_shaped_binding_after_text_changes() {
    let mut runtime = AppRuntime::<RetainedTextApp>::mount("first");
    register_controlled_text(&mut runtime);
    let environment = StyleEnvironment::default();
    let context = SurfaceBuildContext::new(&environment, LayoutConstraints::unbounded());
    let first = publish(&mut runtime, &context);
    let old_ref = first
        .paint_scene()
        .items()
        .iter()
        .find_map(|item| item.primitive().as_shaped_text_run())
        .unwrap_or_else(|| unreachable!("initial text contributes a shaped run"))
        .resource_ref()
        .clone();

    runtime
        .submit_action("second")
        .unwrap_or_else(|_| unreachable!("test action queue has capacity"));
    assert!(
        runtime
            .pump(PumpBudget::new(4, usize::MAX, usize::MAX, usize::MAX))
            .processed_envelopes()
            >= 2
    );
    assert_eq!(runtime.state(), &"second");
    let second = publish(&mut runtime, &context);
    assert!(
        second
            .paint_scene()
            .shaped_text_resource(&old_ref)
            .is_none()
    );

    drop(second);
    drop(runtime);
    let retained = first
        .paint_scene()
        .shaped_text_resource(&old_ref)
        .unwrap_or_else(|| unreachable!("retained publication owns the old logical binding"));
    assert_eq!(retained.resource_ref(), &old_ref);
    assert!(!retained.glyphs().is_empty());
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
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> Element<Self::Action> {
        let huge = EdgeInsets::all(LogicalLength::MAX);
        match state {
            BoundaryCase::Horizontal => row(children![
                text("huge").padding(huge),
                text("after-one"),
                text("after-two"),
            ])
            .into_element(),
            BoundaryCase::Vertical => column(children![
                text("huge").padding(huge),
                text("after-one"),
                text("after-two"),
            ])
            .into_element(),
            BoundaryCase::Padded => text("small").padding(huge).into_element(),
        }
    }

    fn update(_: &mut Self::State, (): Self::Action) {}
}

#[test]
fn derived_geometry_saturates_without_non_finite_bounds_or_maxima() {
    let style_environment = StyleEnvironment::default();
    for case in [
        BoundaryCase::Horizontal,
        BoundaryCase::Vertical,
        BoundaryCase::Padded,
    ] {
        let mut runtime = AppRuntime::<BoundaryApp>::mount(case);
        register_controlled_text(&mut runtime);
        let publication = publish(
            &mut runtime,
            &SurfaceBuildContext::new(&style_environment, LayoutConstraints::unbounded()),
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
    register_controlled_text(&mut runtime);
    let style_environment = StyleEnvironment::default();
    let publication = publish(
        &mut runtime,
        &SurfaceBuildContext::new(&style_environment, LayoutConstraints::loose(size(2.0, 2.0))),
    );
    assert!(
        publication
            .layout_report()
            .root()
            .is_some_and(|node| node.overflow().any())
    );
}

struct AuthoredRootBoundsApp;

impl UiApp for AuthoredRootBoundsApp {
    type State = ();
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root((): &()) -> Element<Self::Action> {
        text("root")
            .with_layout(
                LayoutStyle::default()
                    .with_width(LayoutDimension::length(length(200.0)))
                    .with_min_width(LayoutBound::length(length(100.0)))
                    .with_max_width(LayoutBound::length(length(120.0))),
            )
            .into_element()
    }

    fn update((): &mut (), (): ()) {}
}

#[test]
fn root_constraints_preserve_authored_root_bounds() {
    let mut runtime = AppRuntime::<AuthoredRootBoundsApp>::mount(());
    register_controlled_text(&mut runtime);
    let environment = StyleEnvironment::default();
    let publication = publish(
        &mut runtime,
        &SurfaceBuildContext::new(&environment, LayoutConstraints::loose(size(150.0, 100.0))),
    );
    let root = publication
        .frame()
        .root()
        .unwrap_or_else(|| unreachable!("authored root is published"));
    assert!((root.bounds().width() - 120.0).abs() <= f32::EPSILON);
}
