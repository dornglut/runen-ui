#![cfg(feature = "internal-test-seams")]
#![allow(refining_impl_trait)]

use runenui_core::{Effects, IntoEffects, UiApp, View, text};
use runenui_runtime::{AppRuntime, PumpBudget};

const TASK_COUNT: usize = 10_000;

struct Tick;

struct App;

impl UiApp for App {
    fn initial_producer_budget(_state: &Self::State) -> runenui_core::ProducerBudget {
        // One text owner, one local start, and its outcome/cancel pair. Each
        // completed generation retires before its successor's app turn.
        runenui_core::ProducerBudget::new(4, 1)
            .with_reconciliation(1, 1, 1)
            .with_subscriptions(0, 0)
            .with_work_starts(runenui_core::WorkStartBudget::new(1, 0, 0, 0, 0))
            .with_work_residency(runenui_core::WorkResidencyBudget::new(1, 0, 0, 0, 0, 0))
    }

    fn action_producer_budget(
        _state: &Self::State,
        _action: &Self::Action,
    ) -> runenui_core::ProducerBudget {
        runenui_core::ProducerBudget::new(4, 1)
            .with_reconciliation(1, 1, 1)
            .with_subscriptions(0, 0)
            .with_work_starts(runenui_core::WorkStartBudget::new(1, 0, 0, 0, 0))
            .with_work_residency(runenui_core::WorkResidencyBudget::new(1, 0, 0, 0, 0, 0))
    }

    fn interaction_producer_budget(_state: &Self::State) -> runenui_core::ProducerBudget {
        runenui_core::ProducerBudget::new(1, 0)
    }

    type State = usize;
    type Action = Tick;
    type HostProtocol = runenui_core::NoHostProtocol;

    fn root(_: &Self::State) -> impl View<Self::Action> {
        text("registry reclamation")
    }

    fn initial_effects(_: &Self::State) -> impl IntoEffects<Self::Action, Self::HostProtocol> {
        next_tick()
    }

    fn update(
        state: &mut Self::State,
        Tick: Self::Action,
    ) -> impl runenui_core::IntoUpdateOutput<Self::Action, Self::HostProtocol> {
        *state += 1;
        if *state < TASK_COUNT {
            next_tick()
        } else {
            Effects::none()
        }
    }
}

fn next_tick() -> Effects<Tick, runenui_core::NoHostProtocol> {
    Effects::local_task(async { Some(Tick) })
}

#[test]
fn ten_thousand_completed_anonymous_tasks_leave_no_registry_records() {
    let mut runtime = AppRuntime::<App>::mount(0)
        .unwrap_or_else(|_| unreachable!("fixture runtime admission is valid"));
    let report = runtime
        .pump(PumpBudget::new(40_000, usize::MAX, TASK_COUNT, usize::MAX))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();
    assert!(report.is_quiescent());
    assert_eq!(*runtime.state(), TASK_COUNT);
    assert_eq!(runtime.__live_work_record_count_for_test(), 0);
}
