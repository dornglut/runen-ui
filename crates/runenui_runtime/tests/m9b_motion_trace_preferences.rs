#![allow(refining_impl_trait)]

use std::time::Duration;

use runenui_core::{
    AnimationId, Element, ExplicitTimeline, MotionEasing, MotionKeyframe, MotionRepeat,
    MotionTarget, MotionValue, NoHostProtocol, ReducedMotionStrategy, SceneOpacity,
    StyleEnvironment, StylePreferencePolicy, StylePreferences, StyleProperties, TimelineSpec,
    TransitionSpec, UiApp, UnitInterval, View, text,
};
use runenui_runtime::{
    AppRuntime, LayoutConstraints, PumpBudget, SurfaceBuildContext, TraceMotionFact,
    TraceMotionInterpolation, TraceMotionLifecycle, TraceMotionPhase, TraceMotionPolicy,
    TraceMotionPreferenceDecision, TraceMotionSource, TraceRecordKind,
};

struct HoldTraceApp;
struct SnapTraceApp;
struct SuppressedTraceApp;
struct TransitionSnapTraceApp;

macro_rules! timeline_trace_app {
    ($app:ty, $strategy:expr) => {
        impl UiApp for $app {
            type State = ();
            type Action = ();
            type HostProtocol = NoHostProtocol;

            fn root((): &Self::State) -> Element<Self::Action> {
                text("preference trace")
                    .key("root")
                    .timeline(opacity_timeline($strategy))
                    .into_element()
            }

            fn update((): &mut Self::State, (): Self::Action) {}
        }
    };
}

timeline_trace_app!(HoldTraceApp, ReducedMotionStrategy::HoldInitial);
timeline_trace_app!(SnapTraceApp, ReducedMotionStrategy::SnapToEnd);
timeline_trace_app!(SuppressedTraceApp, ReducedMotionStrategy::PreserveEssential);

#[derive(Clone, Copy)]
struct TransitionState {
    transparent: bool,
}

impl UiApp for TransitionSnapTraceApp {
    type State = TransitionState;
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> Element<Self::Action> {
        let opacity = if state.transparent {
            SceneOpacity::TRANSPARENT
        } else {
            SceneOpacity::OPAQUE
        };
        text("transition preference trace")
            .key("root")
            .opacity(opacity)
            .transition(MotionTarget::Opacity, transition_spec())
            .into_element()
    }

    fn update(state: &mut Self::State, (): Self::Action) {
        state.transparent = true;
    }
}

fn opacity_timeline(strategy: ReducedMotionStrategy) -> ExplicitTimeline {
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
        Some(strategy),
    )
    .unwrap_or_else(|_| unreachable!("preference trace timeline is valid"));
    ExplicitTimeline::new(
        AnimationId::from_static("preference-trace")
            .unwrap_or_else(|_| unreachable!("preference trace animation id is valid")),
        spec,
    )
}

fn transition_spec() -> TransitionSpec {
    TransitionSpec::new(
        Duration::from_millis(100),
        Duration::ZERO,
        MotionEasing::Linear,
        Some(ReducedMotionStrategy::SnapToEnd),
    )
    .unwrap_or_else(|_| unreachable!("SnapToEnd transition trace spec is valid"))
}

fn reduced_motion() -> StyleEnvironment {
    StyleEnvironment::default().with_preferences(StylePreferences::new(false, true))
}

fn high_contrast() -> StyleEnvironment {
    StyleEnvironment::default()
        .with_preferences(StylePreferences::new(true, false))
        .with_preference_policy(
            StylePreferencePolicy::new()
                .with_high_contrast(StyleProperties::EMPTY.with_opacity(SceneOpacity::OPAQUE)),
        )
}

fn publish<App: UiApp>(runtime: &mut AppRuntime<App>, environment: &StyleEnvironment) {
    runtime
        .publish_surface(&SurfaceBuildContext::new(
            environment,
            LayoutConstraints::unbounded(),
        ))
        .unwrap_or_else(|_| unreachable!("preference trace publication is admitted"));
}

fn dispatch_transition(runtime: &mut AppRuntime<TransitionSnapTraceApp>) {
    runtime
        .submit_action(())
        .unwrap_or_else(|_| unreachable!("transition preference action is admitted"));
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
    Preference {
        source: ObservedSource,
        reduced_motion: bool,
        strategy: ReducedMotionStrategy,
        decision: TraceMotionPreferenceDecision,
    },
    Lifecycle(ObservedSource, TraceMotionLifecycle),
    Sample {
        source: ObservedSource,
        phase: TraceMotionPhase,
        progress_bits: Option<u32>,
        eased_progress_bits: Option<u32>,
        interpolation: TraceMotionInterpolation,
        suppressed: bool,
    },
}

fn observed_source(source: &TraceMotionSource) -> ObservedSource {
    match source {
        TraceMotionSource::Transition => ObservedSource::Transition,
        TraceMotionSource::Timeline { .. } => ObservedSource::Timeline,
        _ => unreachable!("trace source vocabulary is version-locked for this proof"),
    }
}

fn observed_since<App: UiApp>(runtime: &AppRuntime<App>, start: usize) -> Vec<ObservedFact> {
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
                fact:
                    TraceMotionFact::Preference {
                        source,
                        reduced_motion,
                        strategy,
                        decision,
                    },
            } => Some(ObservedFact::Preference {
                source: observed_source(source),
                reduced_motion: *reduced_motion,
                strategy: *strategy,
                decision: *decision,
            }),
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
                        suppressed,
                    },
            } => Some(ObservedFact::Sample {
                source: observed_source(source),
                phase: *phase,
                progress_bits: *progress_bits,
                eased_progress_bits: *eased_progress_bits,
                interpolation: *interpolation,
                suppressed: *suppressed,
            }),
            _ => None,
        })
        .collect()
}

#[test]
fn hold_initial_trace_restarts_from_the_preference_change_instant() {
    let mut runtime = AppRuntime::<HoldTraceApp>::mount(());
    let reduced = reduced_motion();

    let entered_at = runtime.trace().len();
    publish(&mut runtime, &reduced);
    assert_eq!(
        observed_since(&runtime, entered_at),
        [
            ObservedFact::Policy(TraceMotionPolicy::Absent),
            ObservedFact::Preference {
                source: ObservedSource::Timeline,
                reduced_motion: true,
                strategy: ReducedMotionStrategy::HoldInitial,
                decision: TraceMotionPreferenceDecision::HoldInitial,
            },
            ObservedFact::Lifecycle(
                ObservedSource::Timeline,
                TraceMotionLifecycle::HoldInitialEntered,
            ),
            ObservedFact::Sample {
                source: ObservedSource::Timeline,
                phase: TraceMotionPhase::HeldInitial,
                progress_bits: Some(UnitInterval::ZERO.get().to_bits()),
                eased_progress_bits: None,
                interpolation: TraceMotionInterpolation::Endpoint,
                suppressed: false,
            },
        ]
    );

    runtime
        .advance_time(Duration::from_millis(50))
        .unwrap_or_else(|_| unreachable!("bounded held advance is valid"));
    publish(&mut runtime, &reduced);

    let released_at = runtime.trace().len();
    publish(&mut runtime, &StyleEnvironment::default());
    assert_eq!(
        observed_since(&runtime, released_at),
        [
            ObservedFact::Policy(TraceMotionPolicy::Absent),
            ObservedFact::Preference {
                source: ObservedSource::Timeline,
                reduced_motion: false,
                strategy: ReducedMotionStrategy::HoldInitial,
                decision: TraceMotionPreferenceDecision::Normal,
            },
            ObservedFact::Lifecycle(
                ObservedSource::Timeline,
                TraceMotionLifecycle::HoldInitialReleased,
            ),
            ObservedFact::Lifecycle(ObservedSource::Timeline, TraceMotionLifecycle::Restarted,),
            ObservedFact::Sample {
                source: ObservedSource::Timeline,
                phase: TraceMotionPhase::Running,
                progress_bits: Some(UnitInterval::ZERO.get().to_bits()),
                eased_progress_bits: None,
                interpolation: TraceMotionInterpolation::Endpoint,
                suppressed: false,
            },
        ],
        "lifting HoldInitial must diagnose a restart at progress zero, not hidden elapsed time"
    );
}

#[test]
fn finite_timeline_snap_to_end_traces_terminal_state_and_completed_retention() {
    let mut runtime = AppRuntime::<SnapTraceApp>::mount(());
    let reduced = reduced_motion();

    let snapped_at = runtime.trace().len();
    publish(&mut runtime, &reduced);
    assert_eq!(
        observed_since(&runtime, snapped_at),
        [
            ObservedFact::Policy(TraceMotionPolicy::Absent),
            ObservedFact::Preference {
                source: ObservedSource::Timeline,
                reduced_motion: true,
                strategy: ReducedMotionStrategy::SnapToEnd,
                decision: TraceMotionPreferenceDecision::SnapToEnd,
            },
            ObservedFact::Lifecycle(ObservedSource::Timeline, TraceMotionLifecycle::Completed),
            ObservedFact::Sample {
                source: ObservedSource::Timeline,
                phase: TraceMotionPhase::Completed,
                progress_bits: Some(UnitInterval::ONE.get().to_bits()),
                eased_progress_bits: None,
                interpolation: TraceMotionInterpolation::Endpoint,
                suppressed: false,
            },
        ],
        "SnapToEnd must atomically expose the terminal sample and completed lifecycle without a fabricated start"
    );

    let retained_at = runtime.trace().len();
    publish(&mut runtime, &StyleEnvironment::default());
    assert_eq!(
        observed_since(&runtime, retained_at),
        [
            ObservedFact::Policy(TraceMotionPolicy::Absent),
            ObservedFact::Preference {
                source: ObservedSource::Timeline,
                reduced_motion: false,
                strategy: ReducedMotionStrategy::SnapToEnd,
                decision: TraceMotionPreferenceDecision::Normal,
            },
            ObservedFact::Lifecycle(
                ObservedSource::Timeline,
                TraceMotionLifecycle::CompletedRetained,
            ),
        ],
        "a snapped finite declaration stays completed when reduced motion lifts"
    );
}

#[test]
fn high_contrast_trace_is_independent_from_same_clock_motion_preference() {
    let mut runtime = AppRuntime::<SuppressedTraceApp>::mount(());
    let high_contrast = high_contrast();
    publish(&mut runtime, &high_contrast);

    runtime
        .advance_time(Duration::from_millis(50))
        .unwrap_or_else(|_| unreachable!("bounded suppressed advance is valid"));
    let suppressed_at = runtime.trace().len();
    publish(&mut runtime, &high_contrast);
    assert_eq!(
        observed_since(&runtime, suppressed_at),
        [
            ObservedFact::Policy(TraceMotionPolicy::Absent),
            ObservedFact::Preference {
                source: ObservedSource::Timeline,
                reduced_motion: false,
                strategy: ReducedMotionStrategy::PreserveEssential,
                decision: TraceMotionPreferenceDecision::Normal,
            },
            ObservedFact::Preference {
                source: ObservedSource::Timeline,
                reduced_motion: false,
                strategy: ReducedMotionStrategy::PreserveEssential,
                decision: TraceMotionPreferenceDecision::HighContrastSuppressed,
            },
            ObservedFact::Sample {
                source: ObservedSource::Timeline,
                phase: TraceMotionPhase::Running,
                progress_bits: Some(0.5_f32.to_bits()),
                eased_progress_bits: Some(0.5_f32.to_bits()),
                interpolation: TraceMotionInterpolation::Continuous,
                suppressed: true,
            },
        ],
        "mandatory suppression must be diagnosed separately while the source continues on the same clock"
    );

    let revealed_at = runtime.trace().len();
    publish(&mut runtime, &StyleEnvironment::default());
    assert_eq!(
        observed_since(&runtime, revealed_at),
        [
            ObservedFact::Policy(TraceMotionPolicy::Absent),
            ObservedFact::Preference {
                source: ObservedSource::Timeline,
                reduced_motion: false,
                strategy: ReducedMotionStrategy::PreserveEssential,
                decision: TraceMotionPreferenceDecision::Normal,
            },
            ObservedFact::Sample {
                source: ObservedSource::Timeline,
                phase: TraceMotionPhase::Running,
                progress_bits: Some(0.5_f32.to_bits()),
                eased_progress_bits: Some(0.5_f32.to_bits()),
                interpolation: TraceMotionInterpolation::Continuous,
                suppressed: false,
            },
        ],
        "lifting high contrast must reveal the existing same-clock sample without restart"
    );
}

#[test]
fn transition_snap_to_end_traces_terminal_value_and_completion_atomically() {
    let mut runtime =
        AppRuntime::<TransitionSnapTraceApp>::mount(TransitionState { transparent: false });
    publish(&mut runtime, &StyleEnvironment::default());
    dispatch_transition(&mut runtime);

    let snapped_at = runtime.trace().len();
    publish(&mut runtime, &reduced_motion());
    assert_eq!(
        observed_since(&runtime, snapped_at),
        [
            ObservedFact::Policy(TraceMotionPolicy::Enabled),
            ObservedFact::Preference {
                source: ObservedSource::Transition,
                reduced_motion: true,
                strategy: ReducedMotionStrategy::SnapToEnd,
                decision: TraceMotionPreferenceDecision::SnapToEnd,
            },
            ObservedFact::Lifecycle(ObservedSource::Transition, TraceMotionLifecycle::Completed),
            ObservedFact::Sample {
                source: ObservedSource::Transition,
                phase: TraceMotionPhase::Completed,
                progress_bits: Some(UnitInterval::ONE.get().to_bits()),
                eased_progress_bits: None,
                interpolation: TraceMotionInterpolation::Endpoint,
                suppressed: false,
            },
        ],
        "transition SnapToEnd must diagnose the terminal sample and completed lifecycle in the same accepted candidate"
    );
}
