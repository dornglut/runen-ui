#![allow(refining_impl_trait)]
#![allow(
    clippy::float_cmp,
    reason = "M9 timing-boundary proofs require exact accepted endpoint identity"
)]

use core::num::NonZeroU64;
use std::time::Duration;

use runenui_core::{
    AnimationId, Element, ExplicitTimeline, MotionEasing, MotionKeyframe, MotionRepeat,
    MotionValue, NoHostProtocol, ReducedMotionStrategy, SceneOpacity, StyleEnvironment,
    TimelineSpec, UiApp, UnitInterval, View, text,
};
use runenui_runtime::{
    AppRuntime, LayoutConstraints, SurfaceBuildContext, SurfacePhase, SurfacePublication,
};

#[derive(Clone, Copy)]
struct TimingState {
    duration: Duration,
    delay: Duration,
    repeat: MotionRepeat,
}

struct TimingApp;

impl UiApp for TimingApp {
    type State = TimingState;
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> Element<Self::Action> {
        text("timing")
            .key("root")
            .timeline(opacity_timeline(*state))
            .into_element()
    }

    fn update(_: &mut Self::State, (): Self::Action) {}
}

fn opacity_timeline(state: TimingState) -> ExplicitTimeline {
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
        state.duration,
        state.delay,
        state.repeat,
        Some(ReducedMotionStrategy::PreserveEssential),
    )
    .unwrap_or_else(|_| unreachable!("bounded timing proof timeline is valid"));
    ExplicitTimeline::new(
        AnimationId::from_static("timing")
            .unwrap_or_else(|_| unreachable!("timing proof animation id is valid")),
        spec,
    )
}

fn publish(
    runtime: &mut AppRuntime<TimingApp>,
    environment: &StyleEnvironment,
) -> SurfacePublication {
    runtime
        .publish_surface(&SurfaceBuildContext::new(
            environment,
            LayoutConstraints::unbounded(),
        ))
        .unwrap_or_else(|_| unreachable!("timing proof publication is admitted"))
}

fn opacity(publication: &SurfacePublication) -> f32 {
    publication
        .frame()
        .root()
        .unwrap_or_else(|| unreachable!("timing proof has a root"))
        .computed_style()
        .opacity()
        .get()
}

fn advance(runtime: &AppRuntime<TimingApp>, millis: u64) {
    runtime
        .advance_time(Duration::from_millis(millis))
        .unwrap_or_else(|_| unreachable!("bounded timing proof advance is representable"));
}

#[test]
fn delay_holds_keyframe_zero_until_the_active_interval_begins() {
    let mut runtime = AppRuntime::<TimingApp>::mount(TimingState {
        duration: Duration::from_millis(100),
        delay: Duration::from_millis(50),
        repeat: MotionRepeat::ONCE,
    });
    let environment = StyleEnvironment::default();

    assert_eq!(opacity(&publish(&mut runtime, &environment)), 0.0);
    advance(&runtime, 49);
    assert_eq!(opacity(&publish(&mut runtime, &environment)), 0.0);
    advance(&runtime, 1);
    assert_eq!(opacity(&publish(&mut runtime, &environment)), 0.0);
    advance(&runtime, 50);
    assert!((opacity(&publish(&mut runtime, &environment)) - 0.5).abs() <= f32::EPSILON);
    advance(&runtime, 50);
    assert_eq!(opacity(&publish(&mut runtime, &environment)), 1.0);
}

#[test]
fn exact_non_final_repeat_boundary_restarts_at_keyframe_zero() {
    let repeat =
        MotionRepeat::finite(NonZeroU64::new(2).unwrap_or_else(|| unreachable!("two is non-zero")));
    let mut runtime = AppRuntime::<TimingApp>::mount(TimingState {
        duration: Duration::from_millis(100),
        delay: Duration::ZERO,
        repeat,
    });
    let environment = StyleEnvironment::default();

    assert_eq!(opacity(&publish(&mut runtime, &environment)), 0.0);
    advance(&runtime, 100);
    assert_eq!(
        opacity(&publish(&mut runtime, &environment)),
        0.0,
        "an exact non-final iteration boundary starts the next iteration at keyframe zero"
    );
    advance(&runtime, 50);
    assert!((opacity(&publish(&mut runtime, &environment)) - 0.5).abs() <= f32::EPSILON);
    advance(&runtime, 50);
    assert_eq!(
        opacity(&publish(&mut runtime, &environment)),
        1.0,
        "the exact final finite boundary commits keyframe one"
    );

    let retained = publish(&mut runtime, &environment);
    assert_eq!(opacity(&retained), 1.0);
    assert!(runtime.last_surface_phase_report().executed().is_empty());
}

#[test]
fn zero_duration_positive_delay_completes_atomically_at_the_deadline() {
    let mut runtime = AppRuntime::<TimingApp>::mount(TimingState {
        duration: Duration::ZERO,
        delay: Duration::from_millis(50),
        repeat: MotionRepeat::ONCE,
    });
    let environment = StyleEnvironment::default();

    assert_eq!(opacity(&publish(&mut runtime, &environment)), 0.0);
    advance(&runtime, 49);
    assert_eq!(opacity(&publish(&mut runtime, &environment)), 0.0);
    advance(&runtime, 1);
    assert_eq!(
        opacity(&publish(&mut runtime, &environment)),
        1.0,
        "the first candidate at the delayed zero-duration deadline commits the terminal sample"
    );

    let retained = publish(&mut runtime, &environment);
    assert_eq!(opacity(&retained), 1.0);
    assert!(runtime.last_surface_phase_report().executed().is_empty());
}

#[test]
fn forever_timeline_restarts_each_exact_iteration_boundary_without_completion() {
    let mut runtime = AppRuntime::<TimingApp>::mount(TimingState {
        duration: Duration::from_millis(100),
        delay: Duration::ZERO,
        repeat: MotionRepeat::Forever,
    });
    let environment = StyleEnvironment::default();

    assert_eq!(opacity(&publish(&mut runtime, &environment)), 0.0);
    advance(&runtime, 100);
    assert_eq!(opacity(&publish(&mut runtime, &environment)), 0.0);
    advance(&runtime, 150);
    assert!((opacity(&publish(&mut runtime, &environment)) - 0.5).abs() <= f32::EPSILON);
    assert_eq!(
        runtime.last_surface_phase_report().executed(),
        &[SurfacePhase::Paint]
    );
}
