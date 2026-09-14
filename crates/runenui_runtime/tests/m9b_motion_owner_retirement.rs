#![allow(refining_impl_trait)]

use std::time::Duration;

use runenui_core::{
    AnimationId, Element, ExplicitTimeline, MotionEasing, MotionKeyframe, MotionRepeat,
    MotionValue, NoHostProtocol, ReducedMotionStrategy, SceneOpacity, StyleEnvironment,
    TimelineSpec, UiApp, UnitInterval, View, column, text,
};
use runenui_runtime::{
    AppRuntime, LayoutConstraints, MonotonicInstant, PumpBudget, SurfaceBuildContext,
};

#[derive(Clone, Copy)]
struct RetirementState {
    show_early: bool,
}

struct RetirementApp;

impl UiApp for RetirementApp {
    type State = RetirementState;
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> Element<Self::Action> {
        let mut children = Vec::new();
        if state.show_early {
            children.push(
                text("early")
                    .key("early")
                    .timeline(delayed_timeline("early", 25))
                    .into_element(),
            );
        }
        children.push(
            text("late")
                .key("late")
                .timeline(delayed_timeline("late", 100))
                .into_element(),
        );
        column(children).key("root").into_element()
    }

    fn update(state: &mut Self::State, (): Self::Action) {
        state.show_early = false;
    }
}

fn delayed_timeline(id: &'static str, delay_millis: u64) -> ExplicitTimeline {
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
        Duration::from_millis(delay_millis),
        MotionRepeat::ONCE,
        Some(ReducedMotionStrategy::PreserveEssential),
    )
    .unwrap_or_else(|_| unreachable!("bounded owner-retirement timeline is valid"));
    ExplicitTimeline::new(
        AnimationId::from_static(id)
            .unwrap_or_else(|_| unreachable!("owner-retirement animation id is valid")),
        spec,
    )
}

fn publish(runtime: &mut AppRuntime<RetirementApp>) {
    let environment = StyleEnvironment::default();
    runtime
        .publish_surface(&SurfaceBuildContext::new(
            &environment,
            LayoutConstraints::unbounded(),
        ))
        .unwrap_or_else(|_| unreachable!("owner-retirement publication is admitted"));
}

fn observe(runtime: &mut AppRuntime<RetirementApp>) -> runenui_runtime::PumpReport {
    runtime.pump(PumpBudget::new(0, 0, 0, 0))
}

#[test]
fn retiring_one_owner_preserves_another_owners_delayed_motion_deadline() {
    let mut runtime = AppRuntime::<RetirementApp>::mount(RetirementState { show_early: true });
    publish(&mut runtime);

    let early_deadline = MonotonicInstant::ZERO
        .checked_add(Duration::from_millis(25))
        .unwrap_or_else(|_| unreachable!("bounded early deadline is representable"));
    let late_deadline = MonotonicInstant::ZERO
        .checked_add(Duration::from_millis(100))
        .unwrap_or_else(|_| unreachable!("bounded late deadline is representable"));
    assert_eq!(observe(&mut runtime).next_deadline(), Some(early_deadline));

    runtime
        .submit_action(())
        .unwrap_or_else(|_| unreachable!("owner-retirement action is admitted"));
    runtime.pump(PumpBudget::new(2, usize::MAX, usize::MAX, usize::MAX));
    publish(&mut runtime);

    let after_retirement = observe(&mut runtime);
    assert_eq!(
        after_retirement.next_deadline(),
        Some(late_deadline),
        "retiring the owner of the earliest deadline must not erase a later live owner's wake"
    );
    assert!(!after_retirement.publication_dirty());

    runtime
        .advance_time(Duration::from_millis(99))
        .unwrap_or_else(|_| unreachable!("bounded owner-retirement advance is representable"));
    let before_late = observe(&mut runtime);
    assert_eq!(before_late.next_deadline(), Some(late_deadline));
    assert!(!before_late.publication_dirty());

    runtime
        .advance_time(Duration::from_millis(1))
        .unwrap_or_else(|_| unreachable!("bounded owner-retirement advance is representable"));
    let due = observe(&mut runtime);
    assert_eq!(due.next_deadline(), Some(late_deadline));
    assert!(
        due.publication_dirty(),
        "the surviving owner's delayed motion must wake publication at its exact deadline"
    );
    assert!(!due.due_timers_pending());
    assert_eq!(due.promoted_timers(), 0);
}
