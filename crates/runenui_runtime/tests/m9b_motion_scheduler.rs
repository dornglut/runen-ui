#![allow(refining_impl_trait)]

use std::time::Duration;

use runenui_core::{
    AnimationId, Element, ExplicitTimeline, MotionEasing, MotionKeyframe, MotionRepeat,
    MotionValue, NoHostProtocol, ReducedMotionStrategy, SceneOpacity, StyleEnvironment,
    TimelineSpec, UiApp, UnitInterval, View, text,
};
use runenui_runtime::{
    AppRuntime, LayoutConstraints, MonotonicInstant, PumpBudget, SurfaceBuildContext,
};

struct DelayedMotionApp;

impl UiApp for DelayedMotionApp {
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
        text("scheduler")
            .key("root")
            .timeline(delayed_timeline())
            .into_element()
    }

    fn update((): &mut Self::State, (): Self::Action) {}
}

fn delayed_timeline() -> ExplicitTimeline {
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
        Duration::from_millis(50),
        MotionRepeat::ONCE,
        Some(ReducedMotionStrategy::PreserveEssential),
    )
    .unwrap_or_else(|_| unreachable!("bounded scheduler proof timeline is valid"));
    ExplicitTimeline::new(
        AnimationId::from_static("delayed")
            .unwrap_or_else(|_| unreachable!("scheduler proof animation id is valid")),
        spec,
    )
}

fn observe(runtime: &mut AppRuntime<DelayedMotionApp>) -> runenui_runtime::PumpReport {
    runtime
        .pump(PumpBudget::new(0, 0, 0, 0))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned()
}

#[test]
fn delayed_motion_uses_the_existing_scheduler_deadline_and_publication_dirty_signal() {
    let mut runtime = AppRuntime::<DelayedMotionApp>::mount(())
        .unwrap_or_else(|error| unreachable!("unexpected initial admission refusal: {error}"));
    let environment = StyleEnvironment::default();
    runtime
        .publish_surface(&SurfaceBuildContext::new(
            &environment,
            LayoutConstraints::unbounded(),
        ))
        .unwrap_or_else(|_| unreachable!("scheduler proof publication is admitted"));

    let deadline = MonotonicInstant::ZERO
        .checked_add(Duration::from_millis(50))
        .unwrap_or_else(|_| unreachable!("bounded scheduler deadline is representable"));
    let initial = observe(&mut runtime);
    assert_eq!(initial.next_deadline(), Some(deadline));
    assert!(!initial.publication_dirty());
    assert!(!initial.due_timers_pending());
    assert_eq!(initial.promoted_timers(), 0);

    runtime
        .advance_time(Duration::from_millis(49))
        .unwrap_or_else(|_| unreachable!("bounded scheduler advance is representable"));
    let before = observe(&mut runtime);
    assert_eq!(before.next_deadline(), Some(deadline));
    assert!(!before.publication_dirty());
    assert!(!before.due_timers_pending());
    assert_eq!(before.promoted_timers(), 0);

    runtime
        .advance_time(Duration::from_millis(1))
        .unwrap_or_else(|_| unreachable!("bounded scheduler advance is representable"));
    let due = observe(&mut runtime);
    assert_eq!(due.next_deadline(), Some(deadline));
    assert!(
        due.publication_dirty(),
        "a due motion deadline must request publication through the existing scheduler observation"
    );
    assert!(
        !due.due_timers_pending(),
        "motion deadlines are not a second timer queue"
    );
    assert_eq!(due.promoted_timers(), 0);
}
