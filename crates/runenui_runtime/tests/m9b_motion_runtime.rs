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
    AppRuntime, LayoutConstraints, PublishSurfaceError, PumpBudget, RuntimeConfig, RuntimeStatus,
    SurfaceBuildContext, SurfacePhase, SurfacePublication, TraceConfig, TraceMotionCollision,
    TraceMotionFact, TraceMotionPolicy, TraceMotionSource, TraceRecordKind,
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
struct DuplicateIdApp;
struct OverflowApp;

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
            .id("collision-root")
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

impl UiApp for DuplicateIdApp {
    type State = bool;
    type Action = CollisionAction;
    type HostProtocol = NoHostProtocol;

    fn root(colliding: &Self::State) -> Element<Self::Action> {
        let root = text("duplicate id")
            .id("duplicate-id-root")
            .key("root")
            .timeline(opacity_timeline("fade", Duration::from_millis(100)));
        if *colliding {
            root.timeline(opacity_timeline("fade", Duration::from_millis(100)))
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

impl UiApp for OverflowApp {
    type State = bool;
    type Action = CollisionAction;
    type HostProtocol = NoHostProtocol;

    fn root(enabled: &Self::State) -> Element<Self::Action> {
        let root = text("overflow").id("overflow-root").key("root");
        if *enabled {
            root.timeline(opacity_timeline("late", Duration::from_nanos(2)))
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
    runtime.pump(PumpBudget::new(2, usize::MAX, usize::MAX, usize::MAX));
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

    runtime
        .submit_action(CollisionAction::Set(true))
        .unwrap_or_else(|_| unreachable!("bounded collision action is accepted"));
    pump_one_action(&mut runtime);
    runtime
        .advance_time(Duration::from_millis(20))
        .unwrap_or_else(|_| unreachable!("bounded test advance is representable"));
    let phases_before_rejection = runtime.last_surface_phase_report().clone();
    let trace_before_rejection = runtime.trace().len();
    assert_eq!(
        runtime.publish_surface(&context),
        Err(PublishSurfaceError::Motion)
    );
    assert_eq!(
        runtime.last_surface_phase_report(),
        &phases_before_rejection,
        "rejected collision must not commit staged publication phases"
    );
    let collision_records = runtime
        .trace()
        .records()
        .skip(trace_before_rejection)
        .filter_map(|record| match record.kind() {
            TraceRecordKind::Motion {
                target: MotionTarget::Opacity,
                fact: TraceMotionFact::CollisionRejected { source, collision },
            } => Some((record, source, collision)),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(collision_records.len(), 1);
    let (record, source, collision) = collision_records[0];
    assert_eq!(
        source,
        &TraceMotionSource::Timeline {
            animation_id: AnimationId::from_static("other")
                .unwrap_or_else(|_| unreachable!("test animation identifier is valid")),
        }
    );
    assert_eq!(*collision, TraceMotionCollision::DuplicateTarget);
    assert_eq!(
        record.target().and_then(|target| target.authored_id()),
        Some(
            &runenui_core::ElementId::new("collision-root")
                .unwrap_or_else(|_| unreachable!("test authored identifier is valid")),
        )
    );
    assert_eq!(
        record.instant(),
        Some(
            MonotonicInstant::ZERO
                .checked_add(Duration::from_millis(50))
                .unwrap_or_else(|_| unreachable!("test candidate instant is representable")),
        )
    );
    assert!(runtime.trace().export_jsonl().contains(
        "\"fact\":\"collision_rejected\",\"source\":\"timeline\",\"animation_id\":\"other\",\"collision\":\"duplicate_target\""
    ));

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
fn duplicate_animation_id_rejection_is_canonically_attributed_and_atomic() {
    let mut runtime = AppRuntime::<DuplicateIdApp>::mount(false);
    let environment = StyleEnvironment::default();
    let context = SurfaceBuildContext::new(&environment, LayoutConstraints::unbounded());

    let initial = publish(&mut runtime, &environment);
    runtime
        .submit_action(CollisionAction::Set(true))
        .unwrap_or_else(|_| unreachable!("bounded duplicate-id action is accepted"));
    pump_one_action(&mut runtime);
    let initial_phase_report = runtime.last_surface_phase_report().clone();
    let trace_before_rejection = runtime.trace().len();

    assert_eq!(
        runtime.publish_surface(&context),
        Err(PublishSurfaceError::Motion)
    );
    assert_eq!(runtime.last_surface_phase_report(), &initial_phase_report);
    let collision_records = runtime
        .trace()
        .records()
        .skip(trace_before_rejection)
        .filter_map(|record| match record.kind() {
            TraceRecordKind::Motion {
                target: MotionTarget::Opacity,
                fact: TraceMotionFact::CollisionRejected { source, collision },
            } => Some((record, source, collision)),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(collision_records.len(), 1);
    let (record, source, collision) = collision_records[0];
    assert_eq!(
        source,
        &TraceMotionSource::Timeline {
            animation_id: AnimationId::from_static("fade")
                .unwrap_or_else(|_| unreachable!("test animation identifier is valid")),
        }
    );
    assert_eq!(*collision, TraceMotionCollision::DuplicateAnimationId);
    assert_eq!(
        record.target().and_then(|target| target.authored_id()),
        Some(
            &runenui_core::ElementId::new("duplicate-id-root")
                .unwrap_or_else(|_| unreachable!("test authored identifier is valid")),
        )
    );

    runtime
        .submit_action(CollisionAction::Set(false))
        .unwrap_or_else(|_| unreachable!("bounded duplicate-id repair is accepted"));
    pump_one_action(&mut runtime);
    let retry = publish(&mut runtime, &environment);
    assert_eq!(root_opacity(&retry), root_opacity(&initial));
}

#[test]
fn late_candidate_start_overflow_is_a_canonical_recoverable_planning_rejection() {
    let mut runtime = AppRuntime::<OverflowApp>::mount(false);
    let environment = StyleEnvironment::default();
    let context = SurfaceBuildContext::new(&environment, LayoutConstraints::unbounded());
    let initial = publish(&mut runtime, &environment);

    runtime
        .submit_action(CollisionAction::Set(true))
        .unwrap_or_else(|_| unreachable!("bounded overflow action is accepted"));
    pump_one_action(&mut runtime);
    runtime
        .advance_time(Duration::from_nanos(u64::MAX - 1))
        .unwrap_or_else(|_| unreachable!("candidate instant remains representable"));
    let phases_before_rejection = runtime.last_surface_phase_report().clone();
    let candidate = MonotonicInstant::__runtime_from_nanos(u64::MAX - 1);
    let trace_before_rejection = runtime.trace().len();

    assert_eq!(
        runtime.publish_surface(&context),
        Err(PublishSurfaceError::Motion)
    );
    assert_eq!(
        runtime.last_surface_phase_report(),
        &phases_before_rejection
    );
    assert_eq!(
        runtime
            .trace()
            .records()
            .skip(trace_before_rejection)
            .filter_map(|record| match record.kind() {
                TraceRecordKind::Motion {
                    target: MotionTarget::Opacity,
                    fact:
                        TraceMotionFact::PlanningRejected {
                            source: Some(source),
                            rejection,
                        },
                } => Some((record, source, rejection)),
                _ => None,
            })
            .map(|(record, source, rejection)| {
                (
                    record.instant(),
                    source,
                    rejection,
                    record.target().and_then(|target| target.authored_id()),
                )
            })
            .collect::<Vec<_>>(),
        vec![(
            Some(candidate),
            &TraceMotionSource::Timeline {
                animation_id: AnimationId::from_static("late")
                    .unwrap_or_else(|_| unreachable!("test animation identifier is valid")),
            },
            &runenui_runtime::TraceMotionPlanningRejection::ScheduleOverflow,
            Some(
                &runenui_core::ElementId::new("overflow-root")
                    .unwrap_or_else(|_| unreachable!("test authored identifier is valid")),
            ),
        ),]
    );
    assert_eq!(root_opacity(&initial), 1.0);
    assert!(runtime.trace().export_jsonl().contains(
        "\"fact\":\"planning_rejected\",\"source\":\"timeline\",\"animation_id\":\"late\",\"rejection\":\"schedule_overflow\""
    ));
}

#[cfg(feature = "internal-test-seams")]
#[test]
fn planning_rejection_diagnostic_does_not_consume_publication_reservation() {
    let mut runtime = AppRuntime::<OverflowApp>::mount(false);
    let environment = StyleEnvironment::default();
    let context = SurfaceBuildContext::new(&environment, LayoutConstraints::unbounded());
    let _ = publish(&mut runtime, &environment);

    runtime
        .submit_action(CollisionAction::Set(true))
        .unwrap_or_else(|_| unreachable!("bounded overflow action is accepted"));
    pump_one_action(&mut runtime);
    runtime
        .advance_time(Duration::from_nanos(u64::MAX - 1))
        .unwrap_or_else(|_| unreachable!("candidate instant remains representable"));
    runtime.__seed_next_trace_sequence_for_test(u64::MAX - 3);

    assert!(runtime.__surface_publication_trace_reserved_for_test());
    assert_eq!(runtime.__routed_trace_reservations_for_test(), 0);
    assert_eq!(
        runtime.publish_surface(&context),
        Err(PublishSurfaceError::Motion)
    );
    assert_eq!(runtime.status(), RuntimeStatus::Running);
    assert!(runtime.__surface_publication_trace_reserved_for_test());
    assert_eq!(runtime.__routed_trace_reservations_for_test(), 0);
    assert_eq!(
        runtime.__routed_sequence_state_for_test().1,
        Some(u64::MAX - 2)
    );
}

#[test]
fn enabled_transition_policy_is_recorded_in_the_canonical_trace() {
    let config = RuntimeConfig::default().with_trace_config(TraceConfig::new(128));
    let mut runtime =
        AppRuntime::<TimelineHandoffApp>::mount_with_config(Duration::from_millis(100), config);
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
        record
            .target()
            .map(runenui_runtime::TraceTarget::mounted_node_id),
        Some(root.id())
    );
    assert_eq!(record.instant(), Some(MonotonicInstant::ZERO));
}
