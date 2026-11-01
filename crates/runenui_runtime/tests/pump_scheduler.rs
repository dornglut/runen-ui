#![allow(refining_impl_trait)]

use std::{cell::Cell, rc::Rc, time::Duration};

use runenui_core::{Effects, IntoEffects, NoHostProtocol, TimerEffect, UiApp, View, text};
use runenui_runtime::{AppRuntime, PumpBudget, PumpOutcome};

struct LocalApp;

impl UiApp for LocalApp {
    fn initial_producer_budget(_state: &Self::State) -> runenui_core::ProducerBudget {
        runenui_core::ProducerBudget::new(3, 1)
            .with_reconciliation(1, 0, 1)
            .with_subscriptions(0, 0)
            .with_work_starts(runenui_core::WorkStartBudget::new(1, 0, 0, 0, 0))
            .with_work_residency(runenui_core::WorkResidencyBudget::new(1, 0, 0, 0, 0, 0))
    }

    fn action_producer_budget(
        _state: &Self::State,
        _action: &Self::Action,
    ) -> runenui_core::ProducerBudget {
        runenui_core::ProducerBudget::new(1, 0)
            .with_subscriptions(0, 0)
            .with_work_starts(runenui_core::WorkStartBudget::new(0, 0, 0, 0, 0))
    }

    fn interaction_producer_budget(_state: &Self::State) -> runenui_core::ProducerBudget {
        runenui_core::ProducerBudget::new(1, 0)
    }

    type State = usize;
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root(_: &Self::State) -> impl View<Self::Action> {
        text("local budget")
    }
    fn initial_effects(_: &Self::State) -> impl IntoEffects<Self::Action, Self::HostProtocol> {
        Effects::local_task(async { Some(()) })
    }
    fn update(state: &mut Self::State, (): Self::Action) {
        *state += 1;
    }
}

#[test]
fn local_poll_budget_is_independent_and_exact() {
    let budget = PumpBudget::new(1, 2, 3, 4);
    assert_eq!(budget.max_processed_envelopes(), 1);
    assert_eq!(budget.max_completion_imports(), 2);
    assert_eq!(budget.max_local_polls(), 3);
    assert_eq!(budget.max_timer_promotions(), 4);

    let mut runtime = AppRuntime::<LocalApp>::mount(0)
        .unwrap_or_else(|error| unreachable!("unexpected initial admission refusal: {error}"));
    let blocked = runtime
        .pump(PumpBudget::new(2, 0, 0, 0))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();
    assert_eq!(blocked.polled_local_work(), 0);
    assert!(blocked.exhausted_budgets().local_polls());
    assert_eq!(blocked.outcome(), PumpOutcome::BudgetExhausted);

    let ready = runtime
        .pump(PumpBudget::new(0, 0, 1, 0))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();
    assert_eq!(ready.polled_local_work(), 1);
    assert!(ready.exhausted_budgets().processed_envelopes());
    assert_eq!(*runtime.state(), 0);
    let _ = runtime
        .pump(PumpBudget::new(2, 0, 0, 0))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();
    assert_eq!(*runtime.state(), 1);
}

struct TimerApp;

impl UiApp for TimerApp {
    fn initial_producer_budget(_state: &Self::State) -> runenui_core::ProducerBudget {
        // One initial owner and one timer start, plus its cancellation credit.
        runenui_core::ProducerBudget::new(3, 1)
            .with_reconciliation(1, 0, 1)
            .with_subscriptions(0, 0)
            .with_work_starts(runenui_core::WorkStartBudget::new(0, 0, 1, 0, 0))
            .with_work_residency(runenui_core::WorkResidencyBudget::new(0, 0, 1, 0, 0, 0))
    }

    fn action_producer_budget(
        _state: &Self::State,
        _action: &Self::Action,
    ) -> runenui_core::ProducerBudget {
        runenui_core::ProducerBudget::new(2, 1)
            .with_subscriptions(0, 0)
            .with_work_starts(runenui_core::WorkStartBudget::new(0, 0, 0, 0, 0))
    }

    fn interaction_producer_budget(_state: &Self::State) -> runenui_core::ProducerBudget {
        runenui_core::ProducerBudget::new(1, 0)
    }

    type State = usize;
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root(_: &Self::State) -> impl View<Self::Action> {
        text("timer budget")
    }
    fn initial_effects(_: &Self::State) -> impl IntoEffects<Self::Action, Self::HostProtocol> {
        Effects::timer(TimerEffect::once(Duration::from_millis(5), || ()))
    }
    fn update(state: &mut Self::State, (): Self::Action) {
        *state += 1;
    }
}

#[test]
fn timer_budget_and_future_deadline_observation_are_exact() {
    let mut runtime = AppRuntime::<TimerApp>::mount(0)
        .unwrap_or_else(|error| unreachable!("unexpected initial admission refusal: {error}"));
    let future = runtime
        .pump(PumpBudget::new(2, 0, 0, 0))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();
    assert!(future.is_quiescent());
    assert_eq!(
        future
            .next_deadline()
            .map(runenui_runtime::MonotonicInstant::as_nanos),
        Some(5_000_000)
    );
    runtime
        .advance_time(Duration::from_millis(5))
        .unwrap_or_else(|_| unreachable!());
    let blocked = runtime
        .pump(PumpBudget::new(0, 0, 0, 0))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();
    assert!(blocked.due_timers_pending());
    assert!(blocked.exhausted_budgets().timer_promotions());
    let promoted = runtime
        .pump(PumpBudget::new(0, 0, 0, 1))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();
    assert_eq!(promoted.promoted_timers(), 1);
    assert!(promoted.exhausted_budgets().processed_envelopes());
    let _ = runtime
        .pump(PumpBudget::new(2, 0, 0, 0))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();
    assert_eq!(*runtime.state(), 1);
}

struct SleepingApp;

impl UiApp for SleepingApp {
    fn initial_producer_budget(_state: &Self::State) -> runenui_core::ProducerBudget {
        runenui_core::ProducerBudget::new(3, 1)
            .with_reconciliation(1, 0, 1)
            .with_subscriptions(0, 0)
            .with_work_starts(runenui_core::WorkStartBudget::new(1, 0, 0, 0, 0))
            .with_work_residency(runenui_core::WorkResidencyBudget::new(1, 0, 0, 0, 0, 0))
    }

    fn action_producer_budget(
        _state: &Self::State,
        _action: &Self::Action,
    ) -> runenui_core::ProducerBudget {
        runenui_core::ProducerBudget::new(1, 0)
            .with_subscriptions(0, 0)
            .with_work_starts(runenui_core::WorkStartBudget::new(0, 0, 0, 0, 0))
    }

    fn interaction_producer_budget(_state: &Self::State) -> runenui_core::ProducerBudget {
        runenui_core::ProducerBudget::new(1, 0)
    }

    type State = Rc<Cell<usize>>;
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root(_: &Self::State) -> impl View<Self::Action> {
        text("sleeping")
    }
    fn initial_effects(_: &Self::State) -> impl IntoEffects<Self::Action, Self::HostProtocol> {
        Effects::local_task(std::future::pending())
    }
    fn update(_: &mut Self::State, (): Self::Action) {}
}

#[test]
fn sleeping_local_task_does_not_prevent_quiescence() {
    let mut runtime = AppRuntime::<SleepingApp>::mount(Rc::new(Cell::new(0)))
        .unwrap_or_else(|error| unreachable!("unexpected initial admission refusal: {error}"));
    let report = runtime
        .pump(PumpBudget::new(2, 0, 1, 0))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();
    assert_eq!(report.polled_local_work(), 1);
    assert!(report.is_quiescent());
}
