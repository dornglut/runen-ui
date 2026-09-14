#![allow(refining_impl_trait)]
#![allow(
    clippy::float_cmp,
    reason = "M9 motion endpoint proofs require exact accepted 0/1 sample identity"
)]

use std::time::Duration;

use runenui_core::{
    AnimationId, Element, ExplicitTimeline, MonotonicInstant, MotionEasing, MotionKeyframe,
    MotionRepeat, MotionTarget, MotionValue, NoHostProtocol, ReducedMotionStrategy, SceneOpacity,
    StyleEnvironment, TimelineSpec, TransitionSpec, UiApp, UnitInterval, View, text,
};
use runenui_runtime::{
    AppRuntime, LayoutConstraints, PublishSurfaceError, PumpBudget, RuntimeConfig,
    SurfaceBuildContext, SurfacePhase, SurfacePublication, TraceConfig, TraceMotionFact,
    TraceMotionPolicy, TraceRecordKind,
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

struct TimelineHandoffApp;

impl UiApp for TimelineHandoffApp {
    type State = Duration;
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root(transition_duration: &Self::State) -> Element<Self::Action> {
        text("handoff")
            .key("root")
            .opacity(SceneOpacity::TRANSPARENT)
            .transition(
                MotionTarget::Opacity,
                linear_transition(*transition_duration),
            )
            .timeline(opacity_timeline("fade", Duration::from_millis(100)))
            .into_element()
    }

    fn update(_: &mut Self::State, (): Self::Action) {}
}

struct CollisionApp;

#[derive(Clone, Copy)]
enum CollisionAction {
    Set(bool),
}

impl UiApp for CollisionApp {
    type State = bool;
    type Action = CollisionAction;
    type HostProtocol = NoHostProtocol;

    fn root(colliding: &Self::State) -> Element<Self::Action> {
        let root = text("collision")
            .key("root")
            .timeline(opacity_timeline("fade", Duration::from_millis(100)));
        if *colliding {
            root.timeline(opacity_timeline("other", Duration::from_millis(100)))
                .into_element()
        } else {
            root.into_element()
        }
    }

    fn update(state: &mut Self::State, action: Self::Action) {
        let CollisionAction::Set(value) = action;
        *state = value;
    }
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

fn linear_transition(duration: Duration) -> TransitionSpec {
    TransitionSpec::new(
        duration,
        Duration::ZERO,
        MotionEasing::Linear,
        Some(ReducedMotionStrategy::PreserveEssential),
    )
    .unwrap_or_else(|_| unreachable!("bounded test transition is valid"))
}

fn publish<App: UiApp>(
    runtime: &mut AppRuntime<App>,
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

fn pump_one_action<App: UiApp>(runtime: &mut AppRuntime<App>) {
    runtime.pump(PumpBudget::new(
        2,
        usize::MAX,
        usize::MAX,
        usize::MAX,
    ));
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

#[test]
fn timeline_completion_hands_off_from_exact_terminal_sample() {
    let mut runtime = AppRuntime::<TimelineHandoffApp>::mount(Duration::from_millis(100));
    let environment = StyleEnvironment::default();

    let initial = publish(&mut runtime, &environment);
    assert_eq!(root_opacity(&initial), 0.0);

    runtime
        .advance_time(Duration::from_millis(100))
        .unwrap_or_else(|_| unreachable!("bounded test advance is representable"));
    let terminal = publish(&mut runtime, &environment);
    assert_eq!(
        root_opacity(&terminal),
        1.0,
        "the exact final timeline boundary must publish keyframe 1 before the style transition continues"
    );

    runtime
        .advance_time(Duration::from_millis(50))
        .unwrap_or_else(|_| unreachable!("bounded test advance is representable"));
    let transition_middle = publish(&mut runtime, &environment);
    assert!((root_opacity(&transition_middle) - 0.5).abs() <= f32::EPSILON);
}

#[test]
fn zero_duration_handoff_commits_style_target_in_the_same_candidate() {
    let mut runtime = AppRuntime::<TimelineHandoffApp>::mount(Duration::ZERO);
    let environment = StyleEnvironment::default();

    let initial = publish(&mut runtime, &environment);
    assert_eq!(root_opacity(&initial), 0.0);

    runtime
        .advance_time(Duration::from_millis(100))
        .unwrap_or_else(|_| unreachable!("bounded test advance is representable"));
    let terminal = publish(&mut runtime, &environment);
    assert_eq!(
        root_opacity(&terminal),
        0.0,
        "a zero-duration transition must reach its governed target in the same candidate that ends the timeline"
    );

    let retained = publish(&mut runtime, &environment);
    assert_eq!(root_opacity(&retained), 0.0);
    assert!(runtime.last_surface_phase_report().executed().is_empty());
}

#[test]
fn duplicate_target_rejection_does_not_advance_or_restart_retained_motion() {
    let mut runtime = AppRuntime::<CollisionApp>::mount(false);
    let environment = StyleEnvironment::default();
    let context = SurfaceBuildContext::new(&environment, LayoutConstraints::unbounded());

    let initial = publish(&mut runtime, &environment);
    assert_eq!(root_opacity(&initial), 0.0);
    runtime
        .advance_time(Duration::from_millis(30))
        .unwrap_or_else(|_| unreachable!("bounded test advance is representable"));
    let accepted = publish(&mut runtime, &environment);
    assert!((root_opacity(&accepted) - 0.3).abs() <= f32::EPSILON);
    let phases_before_rejection = runtime.last_surface_phase_report().clone();

    runtime
        .submit_action(CollisionAction::Set(true))
        .unwrap_or_else(|_| unreachable!("bounded collision action is accepted"));
    pump_one_action(&mut runtime);
    runtime
        .advance_time(Duration::from_millis(20))
        .unwrap_or_else(|_| unreachable!("bounded test advance is representable"));
    assert_eq!(
        runtime.publish_surface(&context),
        Err(PublishSurfaceError::Motion)
    );
    assert_eq!(
        runtime.last_surface_phase_report(),
        &phases_before_rejection,
        "rejected collision must not commit staged publication phases"
    );

    runtime
        .submit_action(CollisionAction::Set(false))
        .unwrap_or_else(|_| unreachable!("bounded repair action is accepted"));
    pump_one_action(&mut runtime);
    let retry = publish(&mut runtime, &environment);
    assert!(
        (root_opacity(&retry) - 0.5).abs() <= f32::EPSILON,
        "retry at the same clock instant must continue the pre-rejection timeline instead of restarting or advancing it during failure"
    );
}

#[test]
fn enabled_transition_policy_is_recorded_in_the_canonical_trace() {
    let config = RuntimeConfig::default().with_trace_config(TraceConfig::new(128));
    let mut runtime = AppRuntime::<TimelineHandoffApp>::mount_with_config(
        Duration::from_millis(100),
        config,
    );
    let environment = StyleEnvironment::default();

    let publication = publish(&mut runtime, &environment);
    let root = publication
        .frame()
        .root()
        .unwrap_or_else(|| unreachable!("motion trace proof has a root node"));
    let policy_records = runtime
        .trace()
        .records()
        .filter(|record| {
            matches!(
                record.kind(),
                TraceRecordKind::Motion {
                    target: MotionTarget::Opacity,
                    fact: TraceMotionFact::PolicyResolved {
                        policy: TraceMotionPolicy::Enabled
                    }
                }
            )
        })
        .collect::<Vec<_>>();

    assert_eq!(policy_records.len(), 1);
    let record = policy_records[0];
    assert_eq!(
        record.target().map(|target| target.mounted_node_id()),
        Some(root.id())
    );
    assert_eq!(record.instant(), Some(MonotonicInstant::ZERO));
}
