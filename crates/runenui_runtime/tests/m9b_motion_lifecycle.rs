#![allow(refining_impl_trait)]

use std::time::Duration;

use runenui_core::{
    AnimationId, Element, ExplicitTimeline, MotionEasing, MotionKeyframe, MotionRepeat,
    MotionValue, NoHostProtocol, ReducedMotionStrategy, SceneOpacity, StyleEnvironment,
    TimelineSpec, UiApp, UnitInterval, View, text,
};
use runenui_runtime::{
    AppRuntime, LayoutConstraints, PumpBudget, SurfaceBuildContext, SurfacePublication,
};

#[derive(Clone, Copy)]
struct LifecycleState {
    present: bool,
    duration: Duration,
}

#[derive(Clone, Copy)]
enum LifecycleAction {
    SetPresent(bool),
    SetDuration(Duration),
}

struct LifecycleApp;

impl UiApp for LifecycleApp {
    type State = LifecycleState;
    type Action = LifecycleAction;
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> Element<Self::Action> {
        let root = text("lifecycle").key("root");
        if state.present {
            root.timeline(opacity_timeline(state.duration))
                .into_element()
        } else {
            root.into_element()
        }
    }

    fn update(state: &mut Self::State, action: Self::Action) {
        match action {
            LifecycleAction::SetPresent(present) => state.present = present,
            LifecycleAction::SetDuration(duration) => state.duration = duration,
        }
    }
}

fn opacity_timeline(duration: Duration) -> ExplicitTimeline {
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
    .unwrap_or_else(|_| unreachable!("bounded lifecycle proof timeline is valid"));
    ExplicitTimeline::new(
        AnimationId::from_static("fade")
            .unwrap_or_else(|_| unreachable!("lifecycle proof animation id is valid")),
        spec,
    )
}

fn publish(runtime: &mut AppRuntime<LifecycleApp>) -> SurfacePublication {
    let environment = StyleEnvironment::default();
    runtime
        .publish_surface(&SurfaceBuildContext::new(
            &environment,
            LayoutConstraints::unbounded(),
        ))
        .unwrap_or_else(|_| unreachable!("lifecycle proof publication is admitted"))
}

fn opacity(publication: &SurfacePublication) -> f32 {
    publication
        .frame()
        .root()
        .unwrap_or_else(|| unreachable!("lifecycle proof has a root"))
        .computed_style()
        .opacity()
        .get()
}

fn dispatch(runtime: &mut AppRuntime<LifecycleApp>, action: LifecycleAction) {
    runtime
        .submit_action(action)
        .unwrap_or_else(|_| unreachable!("bounded lifecycle action is admitted"));
    runtime.pump(PumpBudget::new(2, usize::MAX, usize::MAX, usize::MAX));
}

fn advance(runtime: &mut AppRuntime<LifecycleApp>, millis: u64) {
    runtime
        .advance_time(Duration::from_millis(millis))
        .unwrap_or_else(|_| unreachable!("bounded lifecycle advance is representable"));
}

#[test]
fn accepted_absence_then_readd_starts_a_new_timeline_lifetime() {
    let mut runtime = AppRuntime::<LifecycleApp>::mount(LifecycleState {
        present: true,
        duration: Duration::from_millis(100),
    });

    assert_eq!(opacity(&publish(&mut runtime)), 0.0);
    advance(&mut runtime, 50);
    assert!((opacity(&publish(&mut runtime)) - 0.5).abs() <= f32::EPSILON);

    dispatch(&mut runtime, LifecycleAction::SetPresent(false));
    let absent = publish(&mut runtime);
    assert_eq!(opacity(&absent), 1.0);

    dispatch(&mut runtime, LifecycleAction::SetPresent(true));
    assert_eq!(
        opacity(&publish(&mut runtime)),
        0.0,
        "re-adding after an accepted absence must start a fresh declaration lifetime at keyframe zero"
    );

    advance(&mut runtime, 50);
    assert!(
        (opacity(&publish(&mut runtime)) - 0.5).abs() <= f32::EPSILON,
        "the re-added lifetime must measure progress from its own commit instant"
    );
}

#[test]
fn same_id_changed_spec_is_exact_replacement_from_authored_keyframe_zero() {
    let mut runtime = AppRuntime::<LifecycleApp>::mount(LifecycleState {
        present: true,
        duration: Duration::from_millis(100),
    });

    assert_eq!(opacity(&publish(&mut runtime)), 0.0);
    advance(&mut runtime, 50);
    assert!((opacity(&publish(&mut runtime)) - 0.5).abs() <= f32::EPSILON);

    dispatch(
        &mut runtime,
        LifecycleAction::SetDuration(Duration::from_millis(200)),
    );
    assert_eq!(
        opacity(&publish(&mut runtime)),
        0.0,
        "changed spec with the same animation id is replacement and begins at authored keyframe zero"
    );

    advance(&mut runtime, 50);
    assert!(
        (opacity(&publish(&mut runtime)) - 0.25).abs() <= f32::EPSILON,
        "replacement progress must use the replacement spec and replacement commit instant"
    );
}
