#![allow(refining_impl_trait)]

use std::time::Duration;

use runenui_core::{
    AnimationId, Color, ExplicitTimeline, LayoutDimension, LayoutStyle, LogicalLength,
    MotionEasing, MotionKeyframe, MotionRepeat, MotionValue, NoHostProtocol, PresentationOrigin,
    PresentationRotation, PresentationScale, PresentationTransform, PresentationTranslation,
    ReducedMotionStrategy, StyleEnvironment, TimelineSpec, UiApp, UnitInterval, View, button,
};
use runenui_runtime::{
    AppRuntime, LayoutConstraints, SurfaceBuildContext, SurfacePhase, SurfacePublication,
};

struct PresentationMotionApp;
struct LayoutMotionApp;

impl UiApp for PresentationMotionApp {
    type State = ();
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root((): &Self::State) -> impl View<Self::Action> {
        button("presentation")
            .on_activate(|| ())
            .background(Color::WHITE)
            .with_layout(
                LayoutStyle::default()
                    .with_width(LayoutDimension::length(LogicalLength::from(120_u16)))
                    .with_height(LayoutDimension::length(LogicalLength::from(40_u16))),
            )
            .timeline(presentation_timeline())
            .key("root")
    }

    fn update((): &mut Self::State, (): Self::Action) {}
}

impl UiApp for LayoutMotionApp {
    type State = ();
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root((): &Self::State) -> impl View<Self::Action> {
        button("layout")
            .on_activate(|| ())
            .background(Color::WHITE)
            .with_layout(
                LayoutStyle::default()
                    .with_width(LayoutDimension::length(LogicalLength::from(100_u16)))
                    .with_height(LayoutDimension::length(LogicalLength::from(40_u16))),
            )
            .timeline(width_timeline())
            .key("root")
    }

    fn update((): &mut Self::State, (): Self::Action) {}
}

fn timeline(id: &'static str, keyframes: Vec<MotionKeyframe>) -> ExplicitTimeline {
    let spec = TimelineSpec::new(
        keyframes,
        vec![MotionEasing::Linear],
        Duration::from_millis(100),
        Duration::ZERO,
        MotionRepeat::ONCE,
        Some(ReducedMotionStrategy::PreserveEssential),
    )
    .unwrap_or_else(|_| unreachable!("differential-effect proof timeline is valid"));
    ExplicitTimeline::new(
        AnimationId::from_static(id)
            .unwrap_or_else(|_| unreachable!("differential-effect animation id is valid")),
        spec,
    )
}

fn presentation(translation_x: f32) -> PresentationTransform {
    PresentationTransform::new(
        PresentationTranslation::new(translation_x, 0.0)
            .unwrap_or_else(|_| unreachable!("controlled presentation translation is finite")),
        PresentationScale::IDENTITY,
        PresentationRotation::ZERO,
        PresentationOrigin::new(UnitInterval::ZERO, UnitInterval::ZERO),
    )
}

fn presentation_timeline() -> ExplicitTimeline {
    timeline(
        "presentation-motion",
        vec![
            MotionKeyframe::new(
                UnitInterval::ZERO,
                MotionValue::Presentation(Some(presentation(0.0))),
            ),
            MotionKeyframe::new(
                UnitInterval::ONE,
                MotionValue::Presentation(Some(presentation(20.0))),
            ),
        ],
    )
}

fn width_timeline() -> ExplicitTimeline {
    timeline(
        "layout-motion",
        vec![
            MotionKeyframe::new(
                UnitInterval::ZERO,
                MotionValue::Width(LayoutDimension::length(LogicalLength::from(100_u16))),
            ),
            MotionKeyframe::new(
                UnitInterval::ONE,
                MotionValue::Width(LayoutDimension::length(LogicalLength::from(200_u16))),
            ),
        ],
    )
}

fn publish<App: UiApp>(runtime: &mut AppRuntime<App>) -> SurfacePublication {
    let environment = StyleEnvironment::default();
    runtime
        .publish_surface(&SurfaceBuildContext::new(
            &environment,
            LayoutConstraints::unbounded(),
        ))
        .unwrap_or_else(|_| unreachable!("differential-effect publication is admitted"))
}

fn advance<App: UiApp>(runtime: &AppRuntime<App>) {
    runtime
        .advance_time(Duration::from_millis(50))
        .unwrap_or_else(|_| unreachable!("bounded differential-effect advance is representable"));
}

#[test]
fn presentation_motion_republishes_geometry_dependents_without_relayout() {
    let mut runtime = AppRuntime::<PresentationMotionApp>::mount(());
    let initial = publish(&mut runtime);
    let initial_layout = initial
        .frame()
        .root()
        .unwrap_or_else(|| unreachable!("presentation proof has a root"))
        .bounds();

    advance(&runtime);
    let middle = publish(&mut runtime);
    let middle_layout = middle
        .frame()
        .root()
        .unwrap_or_else(|| unreachable!("presentation proof has a root"))
        .bounds();

    assert_eq!(middle_layout, initial_layout);
    assert_eq!(
        runtime.last_surface_phase_report().executed(),
        &[
            SurfacePhase::HitTesting,
            SurfacePhase::Paint,
            SurfacePhase::Semantics,
        ],
        "presentation motion must update presentation-derived products without entering layout"
    );

    let semantic = middle
        .semantic_publication()
        .snapshot()
        .nodes()
        .iter()
        .find(|node| node.name() == Some("presentation"))
        .unwrap_or_else(|| unreachable!("presentation button publishes semantics"));
    assert!((semantic.bounds().x() - 10.0).abs() <= f32::EPSILON);
}

#[test]
fn structural_width_motion_recomputes_layout_and_all_geometry_dependents() {
    let mut runtime = AppRuntime::<LayoutMotionApp>::mount(());
    let initial = publish(&mut runtime);
    let initial_width = initial
        .frame()
        .root()
        .unwrap_or_else(|| unreachable!("layout proof has a root"))
        .bounds()
        .width();
    assert!((initial_width - 100.0).abs() <= f32::EPSILON);

    advance(&runtime);
    let middle = publish(&mut runtime);
    let middle_width = middle
        .frame()
        .root()
        .unwrap_or_else(|| unreachable!("layout proof has a root"))
        .bounds()
        .width();
    assert!(
        (middle_width - 150.0).abs() <= f32::EPSILON,
        "sampled structural width must feed the accepted layout authority"
    );
    assert_eq!(
        runtime.last_surface_phase_report().executed(),
        &[
            SurfacePhase::Layout,
            SurfacePhase::HitTesting,
            SurfacePhase::Paint,
            SurfacePhase::Semantics,
        ],
        "layout motion must recompute layout and every dependent geometry product"
    );
}
