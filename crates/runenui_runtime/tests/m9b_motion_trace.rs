#![allow(refining_impl_trait)]

use std::time::Duration;

use runenui_core::{
    AnimationId, Element, ExplicitTimeline, MotionEasing, MotionKeyframe, MotionRepeat,
    MotionTarget, MotionValue, NoHostProtocol, ReducedMotionStrategy, SceneOpacity,
    StyleEnvironment, TimelineSpec, UiApp, UnitInterval, View, text,
};
use runenui_runtime::{
    AppRuntime, LayoutConstraints, SurfaceBuildContext, TraceMotionFact, TraceMotionPolicy,
    TraceRecordKind,
};

struct ExplicitOnlyApp;

impl UiApp for ExplicitOnlyApp {
    fn initial_producer_budget(_state: &Self::State) -> runenui_core::ProducerBudget {
        // The motion fixture has at most 1 mounted owners; timelines
        // do not introduce scheduler tasks or application output.
        runenui_core::ProducerBudget::new(1, 0)
            .with_reconciliation(1, 0, 1)
            .with_subscriptions(0, 0)
            .with_work_starts(runenui_core::WorkStartBudget::new(0, 0, 0, 0, 0))
    }

    fn action_producer_budget(
        _state: &Self::State,
        _action: &Self::Action,
    ) -> runenui_core::ProducerBudget {
        runenui_core::ProducerBudget::new(1, 0)
            .with_reconciliation(1, 1, 1)
            .with_subscriptions(0, 0)
            .with_work_starts(runenui_core::WorkStartBudget::new(0, 0, 0, 0, 0))
    }

    fn interaction_producer_budget(_state: &Self::State) -> runenui_core::ProducerBudget {
        runenui_core::ProducerBudget::new(1, 0)
    }

    type State = ();
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root((): &Self::State) -> Element<Self::Action> {
        text("trace")
            .id("trace-root")
            .key("root")
            .timeline(opacity_timeline())
            .into_element()
    }

    fn update((): &mut Self::State, (): Self::Action) {}
}

fn opacity_timeline() -> ExplicitTimeline {
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
    .unwrap_or_else(|_| unreachable!("motion trace proof timeline is valid"));
    ExplicitTimeline::new(
        AnimationId::from_static("trace-opacity")
            .unwrap_or_else(|_| unreachable!("motion trace proof animation id is valid")),
        spec,
    )
}

#[test]
fn explicit_target_without_transition_policy_records_absent_from_motion_planning() {
    let mut runtime = AppRuntime::<ExplicitOnlyApp>::mount(())
        .unwrap_or_else(|error| unreachable!("unexpected initial admission refusal: {error}"));
    let environment = StyleEnvironment::default();
    runtime
        .publish_surface(&SurfaceBuildContext::new(
            &environment,
            LayoutConstraints::unbounded(),
        ))
        .unwrap_or_else(|_| unreachable!("motion trace proof publication is admitted"));

    let policies = runtime
        .trace()
        .records()
        .filter_map(|record| match record.kind() {
            TraceRecordKind::Motion {
                target: MotionTarget::Opacity,
                fact: TraceMotionFact::PolicyResolved { policy },
            } => Some(*policy),
            _ => None,
        })
        .collect::<Vec<_>>();

    assert_eq!(
        policies,
        [TraceMotionPolicy::Absent],
        "an explicit motion target participates in policy resolution even when the normal style cascade contributes no transition policy"
    );
}
