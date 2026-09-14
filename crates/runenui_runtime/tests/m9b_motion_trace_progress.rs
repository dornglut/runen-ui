#![allow(refining_impl_trait)]

use std::time::Duration;

use runenui_core::{
    AnimationId, Element, ExplicitTimeline, MotionEasing, MotionKeyframe, MotionRepeat,
    MotionTarget, MotionValue, NoHostProtocol, ReducedMotionStrategy, SceneOpacity,
    StyleEnvironment, TimelineSpec, UiApp, UnitInterval, View, text,
};
use runenui_runtime::{
    AppRuntime, LayoutConstraints, SurfaceBuildContext, TraceMotionFact,
    TraceMotionInterpolation, TraceMotionPhase, TraceRecordKind,
};

struct TraceProgressApp;

impl UiApp for TraceProgressApp {
    type State = ();
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root(_: &Self::State) -> Element<Self::Action> {
        text("trace progress")
            .id("trace-progress-root")
            .key("root")
            .timeline(three_keyframe_timeline())
            .into_element()
    }

    fn update(_: &mut Self::State, (): Self::Action) {}
}

fn opacity(value: f32) -> MotionValue {
    MotionValue::Opacity(
        SceneOpacity::new(value)
            .unwrap_or_else(|_| unreachable!("trace progress opacity is normalized")),
    )
}

fn three_keyframe_timeline() -> ExplicitTimeline {
    let spec = TimelineSpec::new(
        vec![
            MotionKeyframe::new(UnitInterval::ZERO, opacity(0.0)),
            MotionKeyframe::new(
                UnitInterval::new(0.25)
                    .unwrap_or_else(|_| unreachable!("trace progress offset is normalized")),
                opacity(0.25),
            ),
            MotionKeyframe::new(UnitInterval::ONE, opacity(1.0)),
        ],
        vec![MotionEasing::Linear, MotionEasing::Linear],
        Duration::from_millis(100),
        Duration::ZERO,
        MotionRepeat::ONCE,
        Some(ReducedMotionStrategy::PreserveEssential),
    )
    .unwrap_or_else(|_| unreachable!("three-keyframe trace timeline is valid"));
    ExplicitTimeline::new(
        AnimationId::from_static("trace-progress")
            .unwrap_or_else(|_| unreachable!("trace progress animation id is valid")),
        spec,
    )
}

fn publish(runtime: &mut AppRuntime<TraceProgressApp>) {
    let environment = StyleEnvironment::default();
    runtime
        .publish_surface(&SurfaceBuildContext::new(
            &environment,
            LayoutConstraints::unbounded(),
        ))
        .unwrap_or_else(|_| unreachable!("trace progress publication is admitted"));
}

fn samples_since(
    runtime: &AppRuntime<TraceProgressApp>,
    start: usize,
) -> Vec<(TraceMotionPhase, Option<u32>, Option<u32>, TraceMotionInterpolation)> {
    runtime
        .trace()
        .records()
        .skip(start)
        .filter_map(|record| match record.kind() {
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
            } => Some((
                *phase,
                *progress_bits,
                *eased_progress_bits,
                *interpolation,
            )),
            _ => None,
        })
        .collect()
}

#[test]
fn explicit_trace_distinguishes_source_progress_from_segment_easing_progress() {
    let mut runtime = AppRuntime::<TraceProgressApp>::mount(());

    let initial = runtime.trace().len();
    publish(&mut runtime);
    assert_eq!(
        samples_since(&runtime, initial),
        [(
            TraceMotionPhase::Running,
            Some(UnitInterval::ZERO.get().to_bits()),
            None,
            TraceMotionInterpolation::Endpoint,
        )],
        "an exact authored keyframe-zero sample retains source progress without inventing an easing decision"
    );

    runtime
        .advance_time(Duration::from_millis(25))
        .unwrap_or_else(|_| unreachable!("bounded trace progress advance is valid"));
    let keyframe = runtime.trace().len();
    publish(&mut runtime);
    assert_eq!(
        samples_since(&runtime, keyframe),
        [(
            TraceMotionPhase::Running,
            Some(0.25_f32.to_bits()),
            None,
            TraceMotionInterpolation::Endpoint,
        )],
        "an exact interior authored keyframe is an endpoint decision at its source-normalized timeline progress"
    );

    runtime
        .advance_time(Duration::from_millis(25))
        .unwrap_or_else(|_| unreachable!("bounded trace progress advance is valid"));
    let interior = runtime.trace().len();
    publish(&mut runtime);
    let eased_segment_progress = (1.0_f32 / 3.0_f32).to_bits();
    assert_eq!(
        samples_since(&runtime, interior),
        [(
            TraceMotionPhase::Running,
            Some(0.5_f32.to_bits()),
            Some(eased_segment_progress),
            TraceMotionInterpolation::Continuous,
        )],
        "source progress must remain the overall iteration position while eased progress records the segment-local easing output actually used for interpolation"
    );

    runtime
        .advance_time(Duration::from_millis(50))
        .unwrap_or_else(|_| unreachable!("bounded terminal trace progress advance is valid"));
    let terminal = runtime.trace().len();
    publish(&mut runtime);
    assert_eq!(
        samples_since(&runtime, terminal),
        [(
            TraceMotionPhase::Completed,
            Some(UnitInterval::ONE.get().to_bits()),
            None,
            TraceMotionInterpolation::Endpoint,
        )],
        "a natural finite completion records exact source progress one without fabricating a segment easing decision"
    );
}
