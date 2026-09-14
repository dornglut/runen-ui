#![allow(refining_impl_trait)]

use std::time::Duration;

use runenui_core::{
    AnimationId, Element, ExplicitTimeline, MotionEasing, MotionKeyframe, MotionRepeat, MotionValue,
    NoHostProtocol, ReducedMotionStrategy, SceneOpacity, StyleEnvironment, TimelineSpec, UiApp,
    UnitInterval, View, text,
};
use runenui_runtime::{
    AppRuntime, LayoutConstraints, SurfaceBuildContext, SurfacePhase, SurfacePublication,
};

struct TimelineApp;

impl UiApp for TimelineApp {
    type State = ();
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root((): &Self::State) -> Element<Self::Action> {
        text("motion")
            .key("root")
            .timeline(opacity_timeline("fade", Duration::from_millis(100)))
            .into_element()
    }

    fn update((): &mut Self::State, (): Self::Action) {}
}

fn opacity_timeline(id: &'static str, duration: Duration) -> ExplicitTimeline {
    let spec = TimelineSpec::new(
        vec![
            MotionKeyframe::new(
                UnitInterval::ZERO,
                MotionValue::Opacity(SceneOpacity::TRANSPARENT),
            ),
            MotionKeyframe::new(
                UnitInterval::ONE,
                MotionValue::Opacity(SceneOpacity::OPAQUE),
            ),
        ],
        vec![MotionEasing::Linear],
        duration,
        Duration::ZERO,
        MotionRepeat::ONCE,
        Some(ReducedMotionStrategy::PreserveEssential),
    )
    .unwrap_or_else(|_| unreachable!("bounded test timeline is valid"));
    ExplicitTimeline::new(
        AnimationId::from_static(id)
            .unwrap_or_else(|_| unreachable!("test animation identifier is valid")),
        spec,
    )
}

fn publish(
    runtime: &mut AppRuntime<TimelineApp>,
    environment: &StyleEnvironment,
) -> SurfacePublication {
    runtime
        .publish_surface(&SurfaceBuildContext::new(
            environment,
            LayoutConstraints::unbounded(),
        ))
        .unwrap_or_else(|_| unreachable!("motion proof publication is admitted"))
}

fn root_opacity(publication: &SurfacePublication) -> f32 {
    publication
        .frame()
        .root()
        .unwrap_or_else(|| unreachable!("motion proof has a root node"))
        .computed_style()
        .opacity()
        .get()
}

#[test]
fn explicit_timeline_uses_public_manual_time_and_reuses_unaffected_stages() {
    let mut runtime = AppRuntime::<TimelineApp>::mount(());
    let environment = StyleEnvironment::default();

    let initial = publish(&mut runtime, &environment);
    assert_eq!(root_opacity(&initial), 0.0);

    runtime
        .advance_time(Duration::from_millis(50))
        .unwrap_or_else(|_| unreachable!("bounded test advance is representable"));
    let middle = publish(&mut runtime, &environment);
    assert!((root_opacity(&middle) - 0.5).abs() <= f32::EPSILON);
    assert_eq!(
        runtime.last_surface_phase_report().executed(),
        &[SurfacePhase::Paint]
    );

    runtime
        .advance_time(Duration::from_millis(50))
        .unwrap_or_else(|_| unreachable!("bounded test advance is representable"));
    let completed = publish(&mut runtime, &environment);
    assert_eq!(root_opacity(&completed), 1.0);
    assert_eq!(
        runtime.last_surface_phase_report().executed(),
        &[SurfacePhase::Paint]
    );

    let retained = publish(&mut runtime, &environment);
    assert_eq!(root_opacity(&retained), 1.0);
    assert!(runtime.last_surface_phase_report().executed().is_empty());
}
