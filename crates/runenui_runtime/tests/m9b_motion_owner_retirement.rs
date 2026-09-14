#![allow(refining_impl_trait)]

use std::time::Duration;

use runenui_core::{
    AnimationId, Element, ExplicitTimeline, MotionEasing, MotionKeyframe, MotionRepeat,
    MotionValue, NoHostProtocol, ReducedMotionStrategy, SceneOpacity, StyleEnvironment,
    TimelineSpec, UiApp, UnitInterval, View, column, text,
};
use runenui_runtime::{
    AppRuntime, LayoutConstraints, MonotonicInstant, PumpBudget, SurfaceBuildContext,
    SurfacePublication,
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
                    .id("retirement-early")
                    .key("early")
                    .timeline(delayed_timeline("early", 25))
                    .into_element(),
            );
        }
        children.push(
            text("late")
                .id("retirement-late")
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

fn publish(runtime: &mut AppRuntime<RetirementApp>) -> SurfacePublication {
    let environment = StyleEnvironment::default();
    runtime
        .publish_surface(&SurfaceBuildContext::new(
            &environment,
            LayoutConstraints::unbounded(),
        ))
        .unwrap_or_else(|_| unreachable!("owner-retirement publication is admitted"))
}

fn observe(runtime: &mut AppRuntime<RetirementApp>) -> runenui_runtime::PumpReport {
    runtime.pump(PumpBudget::new(0, 0, 0, 0))
}

#[test]
fn retiring_one_owner_preserves_another_owners_delayed_motion_deadline() {
    let mut runtime = AppRuntime::<RetirementApp>::mount(RetirementState { show_early: true });
    let initial_publication = publish(&mut runtime);
    assert_eq!(initial_publication.frame().nodes().len(), 3);

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

    let reconciliation = runtime.reconciliation_report();
    assert_eq!(
        reconciliation.unmounted_count(),
        1,
        "removing the keyed early child must retire exactly one mounted lifetime"
    );
    assert_eq!(reconciliation.live_node_count(), 2);
    let before_republication = observe(&mut runtime);
    assert_eq!(
        before_republication.next_deadline(),
        None,
        "owner retirement must clear the aggregate motion deadline until the surviving owner is reconciled by publication"
    );
    assert!(before_republication.publication_dirty());

    let after_publication = publish(&mut runtime);
    assert_eq!(after_publication.frame().nodes().len(), 2);
    assert!(after_publication.frame().nodes().iter().all(|node| {
        node.authored_id()
            .is_none_or(|id| id.as_str() != "retirement-early")
    }));
    assert!(after_publication.frame().nodes().iter().any(|node| {
        node.authored_id()
            .is_some_and(|id| id.as_str() == "retirement-late")
    }));

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
