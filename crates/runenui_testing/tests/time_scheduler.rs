use core::num::NonZeroUsize;
use std::time::Duration;

use runenui_core::{
    Effects, HostProtocol, IntoEffects, NoHostProtocol, TimerEffect, UiApp, View, text,
};
use runenui_runtime::PumpBudget;
use runenui_testing::{SettleBudget, SettleOutcome, TestHarness};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TimerAction {
    Tick,
}

struct TimerApp;

impl UiApp for TimerApp {
    type State = u32;
    type Action = TimerAction;
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> impl View<Self::Action> {
        text(format!("ticks:{state}"))
    }

    fn initial_effects(_: &Self::State) -> impl IntoEffects<Self::Action, Self::HostProtocol> {
        Effects::timer(TimerEffect::once(Duration::from_secs(5), || {
            TimerAction::Tick
        }))
    }

    fn update(
        state: &mut Self::State,
        action: Self::Action,
    ) -> impl IntoEffects<Self::Action, Self::HostProtocol> {
        match action {
            TimerAction::Tick => *state += 1,
        }
    }
}

fn settle_budget() -> SettleBudget {
    SettleBudget::new(
        NonZeroUsize::new(8).unwrap_or(NonZeroUsize::MIN),
        PumpBudget::new(64, 64, 64, 64),
    )
}

#[test]
fn manual_time_exposes_dormant_future_timer_without_sleeping() {
    let mut harness = TestHarness::<TimerApp>::mount(0);

    let first = harness.run_until_idle(settle_budget());
    assert_eq!(first.outcome(), SettleOutcome::Idle);
    assert_eq!(*harness.state(), 0);
    assert!(first.last_pump().next_deadline().is_some());
    assert!(harness.last_timer_start_outcome().is_some());

    assert!(harness.advance_time(Duration::from_secs(4)).is_ok());
    assert_eq!(
        harness.run_until_idle(settle_budget()).outcome(),
        SettleOutcome::Idle
    );
    assert_eq!(*harness.state(), 0);

    assert!(harness.advance_time(Duration::from_secs(1)).is_ok());
    assert_eq!(
        harness.run_until_idle(settle_budget()).outcome(),
        SettleOutcome::Idle
    );
    assert_eq!(*harness.state(), 1);
    assert!(harness.last_timer_firing_outcome().is_some());
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum LoopAction {
    Again,
}

struct LoopApp;

impl UiApp for LoopApp {
    type State = u32;
    type Action = LoopAction;
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> impl View<Self::Action> {
        text(format!("loops:{state}"))
    }

    fn initial_effects(_: &Self::State) -> impl IntoEffects<Self::Action, Self::HostProtocol> {
        Effects::action(LoopAction::Again)
    }

    fn update(
        state: &mut Self::State,
        action: Self::Action,
    ) -> impl IntoEffects<Self::Action, Self::HostProtocol> {
        match action {
            LoopAction::Again => {
                *state += 1;
                Effects::action(LoopAction::Again)
            }
        }
    }
}

#[test]
fn self_requeue_stops_at_the_explicit_iteration_limit() {
    let mut harness = TestHarness::<LoopApp>::mount(0);
    let budget = SettleBudget::new(
        NonZeroUsize::new(3).unwrap_or(NonZeroUsize::MIN),
        PumpBudget::new(1, 1, 1, 1),
    );

    let report = harness.run_until_idle(budget);
    assert_eq!(report.outcome(), SettleOutcome::IterationLimit);
    assert_eq!(report.iterations(), 3);
    assert!(report.last_pump().remaining_queued_envelopes() > 0);
    assert!(
        report
            .last_pump()
            .exhausted_budgets()
            .processed_envelopes()
    );
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct PendingProtocol;

impl HostProtocol for PendingProtocol {
    type Command = ();
    type Response = ();
    type ResponseKind = ();

    fn expected_response(_: &Self::Command) -> Self::ResponseKind {}

    fn response_kind(_: &Self::Response) -> Self::ResponseKind {}
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PendingAction {
    Completed,
}

struct PendingApp;

impl UiApp for PendingApp {
    type State = bool;
    type Action = PendingAction;
    type HostProtocol = PendingProtocol;

    fn root(state: &Self::State) -> impl View<Self::Action> {
        text(format!("completed:{state}"))
    }

    fn initial_effects(_: &Self::State) -> impl IntoEffects<Self::Action, Self::HostProtocol> {
        Effects::host_request(None, (), |()| PendingAction::Completed)
    }

    fn update(
        state: &mut Self::State,
        action: Self::Action,
    ) -> impl IntoEffects<Self::Action, Self::HostProtocol> {
        match action {
            PendingAction::Completed => *state = true,
        }
    }
}

#[test]
fn externally_pending_host_work_is_observable_and_does_not_busy_wait() {
    let mut harness = TestHarness::<PendingApp>::mount(false);

    let report = harness.run_until_idle(settle_budget());
    assert_eq!(report.outcome(), SettleOutcome::Idle);
    assert_eq!(harness.pending_host_requests().len(), 1);
    assert!(!*harness.state());
    assert_eq!(report.last_pump().processed_envelopes(), 0);
    assert_eq!(report.last_pump().remaining_queued_envelopes(), 0);
}
