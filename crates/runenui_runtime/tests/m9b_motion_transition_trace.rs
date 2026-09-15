#![allow(refining_impl_trait)]

use std::time::Duration;

use runenui_core::{
    AnimationId, Element, ExplicitTimeline, MotionEasing, MotionKeyframe, MotionRepeat,
    MotionTarget, MotionValue, NoHostProtocol, ReducedMotionStrategy, SceneOpacity,
    StyleEnvironment, TimelineSpec, TransitionSpec, UiApp, UnitInterval, View, text,
};
use runenui_runtime::{
    AppRuntime, LayoutConstraints, PumpBudget, SurfaceBuildContext, TraceMotionFact,
    TraceMotionInterpolation, TraceMotionLifecycle, TraceMotionPhase, TraceMotionPolicy,
    TraceMotionSource, TraceRecordKind,
};

#[derive(Clone, Copy)]
struct TraceTransitionState {
    transparent: bool,
    disabled: bool,
    explicit: bool,
}

#[derive(Clone, Copy)]
enum TraceTransitionAction {
    SetTarget(bool),
    DisablePolicy,
    EnableExplicit,
}

struct TraceTransitionApp;

impl UiApp for TraceTransitionApp {
    type State = TraceTransitionState;
    type Action = TraceTransitionAction;
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> Element<Self::Action> {
        let opacity = if state.transparent {
            SceneOpacity::TRANSPARENT
        } else {
            SceneOpacity::OPAQUE
        };
        let root = text("transition trace").key("root").opacity(opacity);
        let root = if state.disabled {
            root.transition_disabled(MotionTarget::Opacity)
        } else {
            root.transition(MotionTarget::Opacity, transition_spec())
        };
        if state.explicit {
            root.timeline(explicit_timeline()).into_element()
        } else {
            root.into_element()
        }
    }

    fn update(state: &mut Self::State, action: Self::Action) {
        match action {
            TraceTransitionAction::SetTarget(transparent) => state.transparent = transparent,
            TraceTransitionAction::DisablePolicy => state.disabled = true,
            TraceTransitionAction::EnableExplicit => state.explicit = true,
        }
    }
}

const fn initial_state() -> TraceTransitionState {
    TraceTransitionState {
        transparent: false,
        disabled: false,
        explicit: false,
    }
}

fn transition_spec() -> TransitionSpec {
    TransitionSpec::new(
        Duration::from_millis(100),
        Duration::ZERO,
        MotionEasing::Linear,
        Some(ReducedMotionStrategy::PreserveEssential),
    )
    .unwrap_or_else(|_| unreachable!("transition trace spec is valid"))
}

fn explicit_timeline() -> ExplicitTimeline {
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
        Duration::from_millis(100),
        Duration::ZERO,
        MotionRepeat::ONCE,
        Some(ReducedMotionStrategy::PreserveEssential),
    )
    .unwrap_or_else(|_| unreachable!("transition-preemption timeline is valid"));
    ExplicitTimeline::new(
        AnimationId::from_static("transition-preemptor")
            .unwrap_or_else(|_| unreachable!("transition-preemption animation id is valid")),
        spec,
    )
}

fn publish(runtime: &mut AppRuntime<TraceTransitionApp>) {
    let environment = StyleEnvironment::default();
    runtime
        .publish_surface(&SurfaceBuildContext::new(
            &environment,
            LayoutConstraints::unbounded(),
        ))
        .unwrap_or_else(|_| unreachable!("transition trace publication is admitted"));
}

fn dispatch(runtime: &mut AppRuntime<TraceTransitionApp>, action: TraceTransitionAction) {
    runtime
        .submit_action(action)
        .unwrap_or_else(|_| unreachable!("transition trace action is admitted"));
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
enum ObservedSource {
    Transition,
    Timeline,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ObservedFact {
    Policy(TraceMotionPolicy),
    Lifecycle(ObservedSource, TraceMotionLifecycle),
    Sample {
        source: ObservedSource,
        phase: TraceMotionPhase,
        progress_bits: Option<u32>,
        eased_progress_bits: Option<u32>,
        interpolation: TraceMotionInterpolation,
    },
}

fn observed_source(source: &TraceMotionSource) -> ObservedSource {
    match source {
        TraceMotionSource::Transition => ObservedSource::Transition,
        TraceMotionSource::Timeline { .. } => ObservedSource::Timeline,
        _ => unreachable!("trace source vocabulary is version-locked for this proof"),
    }
}

fn observed_since(runtime: &AppRuntime<TraceTransitionApp>, start: usize) -> Vec<ObservedFact> {
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
                fact: TraceMotionFact::Lifecycle { source, lifecycle },
            } => Some(ObservedFact::Lifecycle(observed_source(source), *lifecycle)),
            TraceRecordKind::Motion {
                target: MotionTarget::Opacity,
                fact:
                    TraceMotionFact::Sampled {
                        source,
                        phase,
                        progress_bits,
                        eased_progress_bits,
                        interpolation,
                        ..
                    },
            } => Some(ObservedFact::Sample {
                source: observed_source(source),
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
fn transition_trace_preserves_start_sampling_replacement_and_disable_order() {
    let mut runtime = AppRuntime::<TraceTransitionApp>::mount(initial_state());
    publish(&mut runtime);

    dispatch(&mut runtime, TraceTransitionAction::SetTarget(true));
    let started_at = runtime.trace().len();
    publish(&mut runtime);
    assert_eq!(
        observed_since(&runtime, started_at),
        [
            ObservedFact::Policy(TraceMotionPolicy::Enabled),
            ObservedFact::Lifecycle(ObservedSource::Transition, TraceMotionLifecycle::Started,),
            ObservedFact::Sample {
                source: ObservedSource::Transition,
                phase: TraceMotionPhase::Running,
                progress_bits: Some(UnitInterval::ZERO.get().to_bits()),
                eased_progress_bits: Some(UnitInterval::ZERO.get().to_bits()),
                interpolation: TraceMotionInterpolation::Endpoint,
            },
        ],
        "a target change must trace policy before the transition start and exact progress-zero sample"
    );

    runtime
        .advance_time(Duration::from_millis(40))
        .unwrap_or_else(|_| unreachable!("bounded transition trace advance is valid"));
    let running_at = runtime.trace().len();
    publish(&mut runtime);
    assert_eq!(
        observed_since(&runtime, running_at),
        [
            ObservedFact::Policy(TraceMotionPolicy::Enabled),
            ObservedFact::Sample {
                source: ObservedSource::Transition,
                phase: TraceMotionPhase::Running,
                progress_bits: Some(0.4_f32.to_bits()),
                eased_progress_bits: Some(0.4_f32.to_bits()),
                interpolation: TraceMotionInterpolation::Continuous,
            },
        ],
        "a retained transition must expose the exact source progress, eased progress, and core interpolation decision"
    );

    dispatch(&mut runtime, TraceTransitionAction::SetTarget(false));
    let replaced_at = runtime.trace().len();
    publish(&mut runtime);
    assert_eq!(
        observed_since(&runtime, replaced_at),
        [
            ObservedFact::Policy(TraceMotionPolicy::Enabled),
            ObservedFact::Sample {
                source: ObservedSource::Transition,
                phase: TraceMotionPhase::Running,
                progress_bits: Some(0.4_f32.to_bits()),
                eased_progress_bits: Some(0.4_f32.to_bits()),
                interpolation: TraceMotionInterpolation::Continuous,
            },
            ObservedFact::Lifecycle(ObservedSource::Transition, TraceMotionLifecycle::Replaced,),
            ObservedFact::Lifecycle(ObservedSource::Transition, TraceMotionLifecycle::Started,),
            ObservedFact::Sample {
                source: ObservedSource::Transition,
                phase: TraceMotionPhase::Running,
                progress_bits: Some(UnitInterval::ZERO.get().to_bits()),
                eased_progress_bits: Some(UnitInterval::ZERO.get().to_bits()),
                interpolation: TraceMotionInterpolation::Endpoint,
            },
        ],
        "interruption must sample the old transition before replacement, then start and sample the new transition at the same candidate instant"
    );

    dispatch(&mut runtime, TraceTransitionAction::DisablePolicy);
    let cancelled_at = runtime.trace().len();
    publish(&mut runtime);
    assert_eq!(
        observed_since(&runtime, cancelled_at),
        [
            ObservedFact::Policy(TraceMotionPolicy::Disabled),
            ObservedFact::Sample {
                source: ObservedSource::Transition,
                phase: TraceMotionPhase::Running,
                progress_bits: Some(UnitInterval::ZERO.get().to_bits()),
                eased_progress_bits: Some(UnitInterval::ZERO.get().to_bits()),
                interpolation: TraceMotionInterpolation::Endpoint,
            },
            ObservedFact::Lifecycle(ObservedSource::Transition, TraceMotionLifecycle::Cancelled,),
        ],
        "explicit disable must observe the live transition at the candidate instant before cancelling it"
    );
}

#[test]
fn active_transition_is_cancelled_before_a_new_explicit_timeline_starts() {
    let mut runtime = AppRuntime::<TraceTransitionApp>::mount(initial_state());
    publish(&mut runtime);
    dispatch(&mut runtime, TraceTransitionAction::SetTarget(true));
    publish(&mut runtime);
    runtime
        .advance_time(Duration::from_millis(40))
        .unwrap_or_else(|_| unreachable!("bounded transition preemption advance is valid"));

    dispatch(&mut runtime, TraceTransitionAction::EnableExplicit);
    let preempted_at = runtime.trace().len();
    publish(&mut runtime);
    assert_eq!(
        observed_since(&runtime, preempted_at),
        [
            ObservedFact::Policy(TraceMotionPolicy::Enabled),
            ObservedFact::Sample {
                source: ObservedSource::Transition,
                phase: TraceMotionPhase::Running,
                progress_bits: Some(0.4_f32.to_bits()),
                eased_progress_bits: Some(0.4_f32.to_bits()),
                interpolation: TraceMotionInterpolation::Continuous,
            },
            ObservedFact::Lifecycle(ObservedSource::Transition, TraceMotionLifecycle::Cancelled,),
            ObservedFact::Lifecycle(ObservedSource::Timeline, TraceMotionLifecycle::Started,),
            ObservedFact::Sample {
                source: ObservedSource::Timeline,
                phase: TraceMotionPhase::Running,
                progress_bits: Some(UnitInterval::ZERO.get().to_bits()),
                eased_progress_bits: None,
                interpolation: TraceMotionInterpolation::Endpoint,
            },
        ],
        "the displaced transition must be sampled and cancelled before the explicit timeline acquires sole target ownership"
    );
}
