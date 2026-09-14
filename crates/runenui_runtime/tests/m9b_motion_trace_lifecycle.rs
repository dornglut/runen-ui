#![allow(refining_impl_trait)]

use std::time::Duration;

use runenui_core::{
    AnimationId, Element, ExplicitTimeline, MotionEasing, MotionKeyframe, MotionRepeat,
    MotionTarget, MotionValue, NoHostProtocol, ReducedMotionStrategy, SceneOpacity,
    StyleEnvironment, TimelineSpec, UiApp, UnitInterval, View, text,
};
use runenui_runtime::{
    AppRuntime, LayoutConstraints, PumpBudget, SurfaceBuildContext, TraceMotionFact,
    TraceMotionInterpolation, TraceMotionLifecycle, TraceMotionPhase, TraceMotionPolicy,
    TraceRecordKind,
};

#[derive(Clone, Copy)]
struct TraceState {
    duration: Duration,
}

struct TraceLifecycleApp;

impl UiApp for TraceLifecycleApp {
    type State = TraceState;
    type Action = Duration;
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> Element<Self::Action> {
        text("trace lifecycle")
            .id("trace-lifecycle-root")
            .key("root")
            .timeline(opacity_timeline(state.duration))
            .into_element()
    }

    fn update(state: &mut Self::State, duration: Self::Action) {
        state.duration = duration;
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
    .unwrap_or_else(|_| unreachable!("motion lifecycle trace timeline is valid"));
    ExplicitTimeline::new(
        AnimationId::from_static("trace-lifecycle")
            .unwrap_or_else(|_| unreachable!("motion lifecycle trace animation id is valid")),
        spec,
    )
}

fn publish(runtime: &mut AppRuntime<TraceLifecycleApp>) {
    let environment = StyleEnvironment::default();
    runtime
        .publish_surface(&SurfaceBuildContext::new(
            &environment,
            LayoutConstraints::unbounded(),
        ))
        .unwrap_or_else(|_| unreachable!("motion lifecycle trace publication is admitted"));
}

fn dispatch(runtime: &mut AppRuntime<TraceLifecycleApp>, duration: Duration) {
    runtime
        .submit_action(duration)
        .unwrap_or_else(|_| unreachable!("motion lifecycle trace action is admitted"));
    let report = runtime.pump(PumpBudget::new(
        usize::MAX,
        usize::MAX,
        usize::MAX,
        usize::MAX,
    ));
    assert!(report.processed_envelopes() >= 1);
    assert!(report.is_quiescent());
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ObservedFact {
    Policy(TraceMotionPolicy),
    Started,
    Replaced,
    Completed,
    CompletedRetained,
    Sample {
        phase: TraceMotionPhase,
        progress_bits: Option<u32>,
        eased_progress_bits: Option<u32>,
        interpolation: TraceMotionInterpolation,
    },
}

fn observed_since(runtime: &AppRuntime<TraceLifecycleApp>, start: usize) -> Vec<ObservedFact> {
    runtime
        .trace()
        .records()
        .skip(start)
        .filter_map(|record| match record.kind() {
            TraceRecordKind::Motion {
                target: MotionTarget::Opacity,
                fact: TraceMotionFact::PolicyResolved { policy },
            } => Some(ObservedFact::Policy(*policy)),
            TraceRecordKind::Motion {
                target: MotionTarget::Opacity,
                fact: TraceMotionFact::Lifecycle { lifecycle, .. },
            } => match lifecycle {
                TraceMotionLifecycle::Started => Some(ObservedFact::Started),
                TraceMotionLifecycle::Replaced => Some(ObservedFact::Replaced),
                TraceMotionLifecycle::Completed => Some(ObservedFact::Completed),
                TraceMotionLifecycle::CompletedRetained => Some(ObservedFact::CompletedRetained),
                _ => None,
            },
            TraceRecordKind::Motion {
                target: MotionTarget::Opacity,
                fact:
                    TraceMotionFact::Sampled {
                        phase,
                        progress_bits,
                        eased_progress_bits,
                        interpolation,
                        ..
                    },
            } => Some(ObservedFact::Sample {
                phase: *phase,
                progress_bits: *progress_bits,
                eased_progress_bits: *eased_progress_bits,
                interpolation: *interpolation,
            }),
            _ => None,
        })
        .collect()
}

#[test]
fn explicit_trace_uses_core_sampling_metadata_and_retains_completion() {
    let mut runtime = AppRuntime::<TraceLifecycleApp>::mount(TraceState {
        duration: Duration::from_millis(100),
    });

    let initial_start = runtime.trace().len();
    publish(&mut runtime);
    assert_eq!(
        observed_since(&runtime, initial_start),
        [
            ObservedFact::Policy(TraceMotionPolicy::Absent),
            ObservedFact::Started,
            ObservedFact::Sample {
                phase: TraceMotionPhase::Running,
                progress_bits: Some(UnitInterval::ZERO.get().to_bits()),
                eased_progress_bits: None,
                interpolation: TraceMotionInterpolation::Endpoint,
            },
        ],
        "policy resolution must precede a fresh explicit start and exact keyframe-zero sample"
    );

    runtime
        .advance_time(Duration::from_millis(50))
        .unwrap_or_else(|_| unreachable!("bounded motion lifecycle trace advance is valid"));
    let middle_start = runtime.trace().len();
    publish(&mut runtime);
    assert_eq!(
        observed_since(&runtime, middle_start),
        [
            ObservedFact::Policy(TraceMotionPolicy::Absent),
            ObservedFact::Sample {
                phase: TraceMotionPhase::Running,
                progress_bits: Some(0.5_f32.to_bits()),
                eased_progress_bits: Some(0.5_f32.to_bits()),
                interpolation: TraceMotionInterpolation::Continuous,
            },
        ],
        "the canonical trace must resolve policy before carrying source progress, easing output, and the core interpolation decision used by sampling"
    );

    runtime
        .advance_time(Duration::from_millis(50))
        .unwrap_or_else(|_| unreachable!("bounded terminal trace advance is valid"));
    let terminal_start = runtime.trace().len();
    publish(&mut runtime);
    assert_eq!(
        observed_since(&runtime, terminal_start),
        [
            ObservedFact::Policy(TraceMotionPolicy::Absent),
            ObservedFact::Completed,
            ObservedFact::Sample {
                phase: TraceMotionPhase::Completed,
                progress_bits: Some(UnitInterval::ONE.get().to_bits()),
                eased_progress_bits: None,
                interpolation: TraceMotionInterpolation::Endpoint,
            },
        ],
        "completion must follow policy resolution and precede the exact terminal endpoint sample"
    );

    let retained_start = runtime.trace().len();
    publish(&mut runtime);
    assert_eq!(
        observed_since(&runtime, retained_start),
        [
            ObservedFact::Policy(TraceMotionPolicy::Absent),
            ObservedFact::CompletedRetained,
        ],
        "a completed unchanged declaration is retained after policy resolution without reacquiring sample authority"
    );
}

#[test]
fn explicit_replacement_traces_old_sample_then_replacement_before_new_start() {
    let mut runtime = AppRuntime::<TraceLifecycleApp>::mount(TraceState {
        duration: Duration::from_millis(100),
    });
    publish(&mut runtime);
    runtime
        .advance_time(Duration::from_millis(50))
        .unwrap_or_else(|_| unreachable!("bounded replacement trace advance is valid"));

    dispatch(&mut runtime, Duration::from_millis(200));
    let replacement_start = runtime.trace().len();
    publish(&mut runtime);
    assert_eq!(
        observed_since(&runtime, replacement_start),
        [
            ObservedFact::Policy(TraceMotionPolicy::Absent),
            ObservedFact::Sample {
                phase: TraceMotionPhase::Running,
                progress_bits: Some(0.5_f32.to_bits()),
                eased_progress_bits: Some(0.5_f32.to_bits()),
                interpolation: TraceMotionInterpolation::Continuous,
            },
            ObservedFact::Replaced,
            ObservedFact::Started,
            ObservedFact::Sample {
                phase: TraceMotionPhase::Running,
                progress_bits: Some(UnitInterval::ZERO.get().to_bits()),
                eased_progress_bits: None,
                interpolation: TraceMotionInterpolation::Endpoint,
            },
        ],
        "replacement must resolve policy, observe the old same-clock sample before retiring it, then start and sample the replacement at authored keyframe zero"
    );
}

#[test]
fn zero_duration_trace_preserves_start_before_same_candidate_completion() {
    let mut runtime = AppRuntime::<TraceLifecycleApp>::mount(TraceState {
        duration: Duration::ZERO,
    });

    let start = runtime.trace().len();
    publish(&mut runtime);
    assert_eq!(
        observed_since(&runtime, start),
        [
            ObservedFact::Policy(TraceMotionPolicy::Absent),
            ObservedFact::Started,
            ObservedFact::Completed,
            ObservedFact::Sample {
                phase: TraceMotionPhase::Completed,
                progress_bits: Some(UnitInterval::ONE.get().to_bits()),
                eased_progress_bits: None,
                interpolation: TraceMotionInterpolation::Endpoint,
            },
        ],
        "a zero-duration ordinary timeline starts and completes atomically in one successful candidate without losing the start observation"
    );
}
