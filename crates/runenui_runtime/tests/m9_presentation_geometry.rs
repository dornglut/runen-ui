use runenui_core::{
    Color, IntoEffects, LayoutDimension, LayoutStyle, LogicalLength, LogicalPoint, NoHostProtocol,
    PresentationOrigin, PresentationRotation, PresentationScale, PresentationTransform,
    PresentationTranslation, StyleEnvironment, UiApp, UnitInterval, View, button,
};
use runenui_runtime::{AppRuntime, LayoutConstraints, SurfaceBuildContext};

struct PresentationGeometryApp;

impl UiApp for PresentationGeometryApp {
    type State = ();
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root((): &Self::State) -> impl View<Self::Action> {
        button("translated")
            .on_activate(|| ())
            .background(Color::WHITE)
            .with_layout(
                LayoutStyle::default()
                    .with_width(LayoutDimension::length(LogicalLength::from(120_u16)))
                    .with_height(LayoutDimension::length(LogicalLength::from(40_u16))),
            )
            .presentation(translation())
            .key("root")
    }

    fn update(
        (): &mut Self::State,
        (): Self::Action,
    ) -> impl IntoEffects<Self::Action, Self::HostProtocol> {
    }
}

fn translation() -> PresentationTransform {
    PresentationTransform::new(
        PresentationTranslation::new(10.0, 20.0)
            .unwrap_or_else(|_| unreachable!("controlled translation is finite")),
        PresentationScale::IDENTITY,
        PresentationRotation::ZERO,
        PresentationOrigin::new(UnitInterval::ZERO, UnitInterval::ZERO),
    )
}

fn approx_eq(left: f32, right: f32) {
    assert!((left - right).abs() <= 1.0e-4, "{left} != {right}");
}

#[test]
fn static_presentation_translation_correlates_paint_hit_and_semantic_geometry_without_layout_mutation()
 {
    let mut runtime = AppRuntime::<PresentationGeometryApp>::mount(());
    let environment = StyleEnvironment::default();
    let publication = runtime
        .publish_surface(&SurfaceBuildContext::new(
            &environment,
            LayoutConstraints::unbounded(),
        ))
        .unwrap_or_else(|_| unreachable!("controlled presentation publication is admitted"));

    let layout = publication
        .frame()
        .root()
        .unwrap_or_else(|| unreachable!("published app has one root"))
        .bounds();
    let expected_left = layout.x() + 10.0;
    let expected_top = layout.y() + 20.0;

    let paint = publication
        .paint_scene()
        .items()
        .iter()
        .find(|item| item.primitive().brush().is_some())
        .unwrap_or_else(|| unreachable!("button background contributes one paint item"));
    let hit = publication
        .hit_test_scene()
        .regions()
        .first()
        .unwrap_or_else(|| unreachable!("actionable button contributes one hit region"));

    assert_eq!(paint.local_to_surface(), hit.local_to_surface());
    let [m11, m12, m21, m22, tx, ty] = paint.local_to_surface().components();
    approx_eq(m11, 1.0);
    approx_eq(m12, 0.0);
    approx_eq(m21, 0.0);
    approx_eq(m22, 1.0);
    approx_eq(tx, expected_left);
    approx_eq(ty, expected_top);

    let semantic = publication
        .semantic_publication()
        .snapshot()
        .nodes()
        .iter()
        .find(|node| node.name() == Some("translated"))
        .unwrap_or_else(|| unreachable!("button publishes one semantic node"));
    let semantic_bounds = semantic.bounds();
    approx_eq(semantic_bounds.x(), expected_left);
    approx_eq(semantic_bounds.y(), expected_top);
    approx_eq(semantic_bounds.width(), layout.width());
    approx_eq(semantic_bounds.height(), layout.height());

    let hit_point = LogicalPoint::new(
        semantic_bounds.width().mul_add(0.5, semantic_bounds.x()),
        semantic_bounds.height().mul_add(0.5, semantic_bounds.y()),
    )
    .unwrap_or_else(|_| unreachable!("controlled hit point is finite"));
    assert_eq!(
        publication.hit_test_scene().target_at(hit_point),
        Some(
            publication
                .frame()
                .root()
                .unwrap_or_else(|| unreachable!())
                .id()
        )
    );

    // Presentation changes publication geometry only; the retained layout/debug
    // box remains the untransformed layout authority.
    approx_eq(layout.x(), 0.0);
    approx_eq(layout.y(), 0.0);
}
