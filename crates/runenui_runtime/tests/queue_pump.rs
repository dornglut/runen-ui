use std::{cell::Cell, rc::Rc};

use runenui_core::{Element, View, Widget, WidgetMountContext, WidgetUpdateContext, text};
use runenui_runtime::{
    AppRuntime, PumpBudget, PumpOutcome, RuntimeConfig, RuntimeStatus, SubmitActionError,
    SubmitActionErrorKind, UiApp,
};

#[derive(Debug, Eq, PartialEq)]
struct Action(u32);

struct OrderedApp;

impl UiApp for OrderedApp {
    type State = Vec<u32>;
    type Action = Action;

    fn root(state: &Self::State) -> Element<Self::Action> {
        text(state.len().to_string()).key("root").into_element()
    }

    fn update(state: &mut Self::State, action: Self::Action) {
        state.push(action.0);
    }
}

#[test]
fn submission_sequences_full_recovery_and_fifo_are_exact() {
    let config = RuntimeConfig::default().with_queue_capacity(2);
    let mut runtime = AppRuntime::<OrderedApp>::mount_with_config(Vec::new(), config);
    let first = runtime
        .submit_action(Action(10))
        .unwrap_or_else(|_| unreachable!());
    let second = runtime
        .submit_action(Action(20))
        .unwrap_or_else(|_| unreachable!());
    assert_eq!(first.get(), 1);
    assert_eq!(second.get(), 2);
    let Err(rejected) = runtime.submit_action(Action(30)) else {
        unreachable!()
    };
    assert_eq!(rejected.kind(), SubmitActionErrorKind::Full);
    assert_eq!(rejected.into_action(), Action(30));
    assert!(runtime.state().is_empty());

    let first_pump = runtime.pump(PumpBudget::new(1));
    assert_eq!(first_pump.processed_envelopes(), 1);
    assert_eq!(first_pump.remaining_queued_envelopes(), 1);
    assert_eq!(first_pump.outcome(), PumpOutcome::BudgetExhausted);
    assert_eq!(runtime.state(), &[10]);
    assert_eq!(runtime.reconciliation_report().generation().get(), 2);

    let third = runtime
        .submit_action(rejected_action())
        .unwrap_or_else(|_| unreachable!());
    assert_eq!(third.get(), 3, "full rejection consumed no sequence");
    let final_pump = runtime.pump(PumpBudget::new(8));
    assert_eq!(final_pump.processed_envelopes(), 2);
    assert_eq!(final_pump.remaining_queued_envelopes(), 0);
    assert_eq!(final_pump.outcome(), PumpOutcome::Quiescent);
    assert!(final_pump.is_quiescent());
    assert_eq!(runtime.state(), &[10, 20, 30]);
    assert_eq!(runtime.reconciliation_report().generation().get(), 4);
}

const fn rejected_action() -> Action {
    Action(30)
}

#[test]
fn zero_and_n_budgets_preserve_the_exact_remaining_order() {
    let mut runtime = AppRuntime::<OrderedApp>::mount(Vec::new());
    for value in 0..5 {
        runtime
            .submit_action(Action(value))
            .unwrap_or_else(|_| unreachable!());
    }
    let zero = runtime.pump(PumpBudget::new(0));
    assert_eq!(zero.processed_envelopes(), 0);
    assert_eq!(zero.remaining_queued_envelopes(), 5);
    assert_eq!(zero.outcome(), PumpOutcome::BudgetExhausted);
    let two = runtime.pump(PumpBudget::new(2));
    assert_eq!(two.processed_envelopes(), 2);
    assert_eq!(runtime.state(), &[0, 1]);
    let rest = runtime.pump(PumpBudget::new(usize::MAX));
    assert_eq!(rest.processed_envelopes(), 3);
    assert_eq!(rest.outcome(), PumpOutcome::Quiescent);
    assert_eq!(runtime.state(), &[0, 1, 2, 3, 4]);
}

#[derive(Debug)]
struct MountedValueObserver {
    value: u32,
    mounted_value: Rc<Cell<u32>>,
}

impl Widget<TransactionAction> for MountedValueObserver {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn mount(&self, (): &mut Self::State, _: &mut WidgetMountContext) {
        self.mounted_value.set(self.value);
    }

    fn update(&self, (): &mut Self::State, _: &mut WidgetUpdateContext) {
        self.mounted_value.set(self.value);
    }
}

struct TransactionState {
    value: u32,
    mounted_value: Rc<Cell<u32>>,
    mounted_values_seen_by_update: Vec<u32>,
}

struct TransactionAction(u32);
struct TransactionApp;

impl UiApp for TransactionApp {
    type State = TransactionState;
    type Action = TransactionAction;

    fn root(state: &Self::State) -> Element<Self::Action> {
        Element::new(MountedValueObserver {
            value: state.value,
            mounted_value: Rc::clone(&state.mounted_value),
        })
        .key("observer")
    }

    fn update(state: &mut Self::State, action: Self::Action) {
        state
            .mounted_values_seen_by_update
            .push(state.mounted_value.get());
        state.value = action.0;
    }
}

#[test]
fn every_action_reconciles_before_the_next_update_begins() {
    let mounted_value = Rc::new(Cell::new(u32::MAX));
    let state = TransactionState {
        value: 0,
        mounted_value: Rc::clone(&mounted_value),
        mounted_values_seen_by_update: Vec::new(),
    };
    let mut runtime = AppRuntime::<TransactionApp>::mount(state);
    runtime
        .submit_action(TransactionAction(1))
        .unwrap_or_else(|_| unreachable!());
    runtime
        .submit_action(TransactionAction(2))
        .unwrap_or_else(|_| unreachable!());

    assert_eq!(runtime.pump(PumpBudget::new(2)).processed_envelopes(), 2);
    assert_eq!(runtime.state().mounted_values_seen_by_update, [0, 1]);
    assert_eq!(mounted_value.get(), 2);
    assert_eq!(runtime.reconciliation_report().generation().get(), 3);
}

#[test]
fn capacity_zero_mounts_but_rejects_every_external_action() {
    let config = RuntimeConfig::default().with_queue_capacity(0);
    let mut runtime = AppRuntime::<OrderedApp>::mount_with_config(Vec::new(), config);
    assert_eq!(runtime.index().nodes().len(), 1);
    let Err(error) = runtime.submit_action(Action(7)) else {
        unreachable!()
    };
    assert_eq!(error.kind(), SubmitActionErrorKind::Full);
    assert_eq!(error.into_action(), Action(7));
    assert_eq!(
        runtime.pump(PumpBudget::new(4)).outcome(),
        PumpOutcome::Quiescent
    );
}

#[test]
fn logical_queue_capacity_does_not_eagerly_reserve() {
    let config = RuntimeConfig::default().with_queue_capacity(usize::MAX);
    let mut runtime = AppRuntime::<OrderedApp>::mount_with_config(Vec::new(), config);

    runtime
        .submit_action(Action(1))
        .unwrap_or_else(|_| unreachable!());
    let report = runtime.pump(PumpBudget::new(1));

    assert_eq!(report.processed_envelopes(), 1);
    assert_eq!(report.remaining_queued_envelopes(), 0);
    assert_eq!(report.outcome(), PumpOutcome::Quiescent);
    assert_eq!(runtime.state(), &[1]);
}

#[test]
fn a_large_queue_is_processed_iteratively() {
    const COUNT: usize = 10_000;
    let config = RuntimeConfig::default().with_queue_capacity(COUNT);
    let mut runtime = AppRuntime::<OrderedApp>::mount_with_config(Vec::new(), config);
    for value in 0..COUNT {
        runtime
            .submit_action(Action(
                u32::try_from(value).unwrap_or_else(|_| unreachable!()),
            ))
            .unwrap_or_else(|_| unreachable!());
    }
    let report = runtime.pump(PumpBudget::new(COUNT));
    assert_eq!(report.processed_envelopes(), COUNT);
    assert_eq!(report.outcome(), PumpOutcome::Quiescent);
    assert_eq!(runtime.state().len(), COUNT);
    assert_eq!(runtime.state()[0], 0);
    assert_eq!(
        runtime.state()[COUNT - 1],
        u32::try_from(COUNT - 1).unwrap_or_else(|_| unreachable!())
    );
}

#[derive(Eq, PartialEq)]
struct LocalOnlyAction(Rc<()>);

struct LocalOnlyApp;

impl UiApp for LocalOnlyApp {
    type State = usize;
    type Action = LocalOnlyAction;

    fn root(_: &Self::State) -> Element<Self::Action> {
        text("local").into_element()
    }

    fn update(state: &mut Self::State, _: Self::Action) {
        *state += 1;
    }
}

#[test]
fn actions_need_neither_clone_send_nor_debug() {
    let mut runtime = AppRuntime::<LocalOnlyApp>::mount(0);
    let action = LocalOnlyAction(Rc::new(()));
    assert!(runtime.submit_action(action).is_ok());
    runtime.pump(PumpBudget::new(1));
    assert_eq!(runtime.state(), &1);
}

#[test]
fn shutdown_is_idempotent_cancels_waiting_actions_and_closes_submission() {
    let mut runtime = AppRuntime::<OrderedApp>::mount(Vec::new());
    runtime
        .submit_action(Action(1))
        .unwrap_or_else(|_| unreachable!());
    runtime
        .submit_action(Action(2))
        .unwrap_or_else(|_| unreachable!());
    let first = runtime.shutdown();
    assert!(!first.already_complete());
    assert_eq!(first.cancelled_queued_envelopes(), 2);
    assert_eq!(first.unmounted_lifetimes(), 1);
    assert_eq!(runtime.status(), RuntimeStatus::Closed);
    assert_eq!(
        runtime.pump(PumpBudget::new(8)).outcome(),
        PumpOutcome::Closed
    );
    assert!(runtime.state().is_empty());
    let Err(error) = runtime.submit_action(Action(3)) else {
        unreachable!()
    };
    assert_eq!(error.kind(), SubmitActionErrorKind::Closed);
    assert_eq!(error.into_action(), Action(3));
    let second = runtime.shutdown();
    assert!(second.already_complete());
    assert_eq!(second.cancelled_queued_envelopes(), 0);
    assert_eq!(second.unmounted_lifetimes(), 0);
}

#[test]
fn error_debugging_does_not_require_action_debug() {
    let config = RuntimeConfig::default().with_queue_capacity(0);
    let mut runtime = AppRuntime::<LocalOnlyApp>::mount_with_config(0, config);
    let Err(error) = runtime.submit_action(LocalOnlyAction(Rc::new(()))) else {
        unreachable!()
    };
    assert_eq!(format!("{error:?}"), "SubmitActionError { kind: Full, .. }");
    assert!(matches!(error, SubmitActionError::Full(_)));
}
