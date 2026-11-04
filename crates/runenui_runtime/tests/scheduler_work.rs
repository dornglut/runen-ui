use std::{
    cell::{Cell, RefCell},
    future::{Future, pending},
    pin::Pin,
    rc::Rc,
    task::{Context, Poll, Waker},
    time::Duration,
};

use runenui_core::{
    Effects, IntoEffects, NoHostProtocol, SendTaskStartFailure, TimerEffect, UiApp, View,
    WorkFamily, WorkKey, text,
};
use runenui_runtime::{
    AppRuntime, InitialAdmissionErrorKind, PumpBudget, RuntimeConfig, RuntimeLimits, RuntimeStatus,
    RuntimeTerminalReason, SendTaskCompletionError, SendTaskExecutor, SendTaskJob,
    SendTaskStartError, SendTaskStartOutcome, TimerFiringOutcome, TimerStartOutcome, TraceConfig,
    TraceRecordKind, TraceTimerTerminalOutcome, TraceWorkFamily, TraceWorkStartRefusal,
};

fn assert_scheduler_step(
    parent: &runenui_runtime::TraceRecord,
    child: &runenui_runtime::TraceRecord,
) {
    assert_eq!(child.causal_parent(), Some(parent.sequence()));
}

fn family_fact<App: UiApp>(
    runtime: &AppRuntime<App>,
    family: TraceWorkFamily,
    predicate: impl Fn(&TraceRecordKind) -> bool,
) -> &runenui_runtime::TraceRecord {
    runtime
        .trace()
        .records()
        .find(|record| {
            predicate(record.kind())
                && record
                    .work()
                    .is_some_and(|identity| identity.family() == family)
        })
        .unwrap_or_else(|| unreachable!("scheduler family fact is present"))
}

fn assert_final_action_chain<App: UiApp>(
    runtime: &AppRuntime<App>,
    terminal: &runenui_runtime::TraceRecord,
) {
    let accepted = runtime
        .trace()
        .records()
        .find(|record| {
            matches!(record.kind(), TraceRecordKind::ActionSubmissionAccepted)
                && record.causal_parent() == Some(terminal.sequence())
        })
        .unwrap_or_else(|| unreachable!("terminal work fact accepts one final action"));
    let transaction = runtime
        .trace()
        .records()
        .find(|record| {
            matches!(
                record.kind(),
                TraceRecordKind::ApplicationActionTransactionStarted
            ) && record.work_sequence() == accepted.work_sequence()
        })
        .unwrap_or_else(|| unreachable!("accepted action enters one application transaction"));
    assert_scheduler_step(accepted, transaction);
}

#[derive(Debug)]
enum Action {
    Label(&'static str),
    NonSend(Rc<()>),
}

struct LocalTaskApp;

impl UiApp for LocalTaskApp {
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

    type State = Vec<&'static str>;
    type Action = Action;
    type HostProtocol = NoHostProtocol;

    fn root(_: &Self::State) -> impl View<Self::Action> {
        text("local")
    }

    fn initial_effects(_: &Self::State) -> impl IntoEffects<Self::Action, Self::HostProtocol> {
        Effects::local_task(async { Some(Action::Label("ready")) })
    }

    fn update(
        state: &mut Self::State,
        action: Self::Action,
    ) -> impl runenui_core::IntoUpdateOutput<Self::Action, Self::HostProtocol> {
        if let Action::Label(label) = action {
            state.push(label);
        }
    }
}

#[test]
fn ready_local_task_reaches_update_only_through_canonical_envelopes() {
    let mut runtime = AppRuntime::<LocalTaskApp>::mount(Vec::new())
        .unwrap_or_else(|error| unreachable!("unexpected initial admission refusal: {error}"));
    let report = runtime
        .pump(PumpBudget::new(8, usize::MAX, usize::MAX, usize::MAX))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();
    assert_eq!(runtime.state(), &["ready"]);
    assert_eq!(report.processed_envelopes(), 3);
    assert!(report.is_quiescent());
}

struct PendingTaskApp;

impl UiApp for PendingTaskApp {
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

    type State = ();
    type Action = Action;
    type HostProtocol = NoHostProtocol;

    fn root((): &Self::State) -> impl View<Self::Action> {
        text("pending")
    }

    fn initial_effects((): &Self::State) -> impl IntoEffects<Self::Action, Self::HostProtocol> {
        Effects::local_task(pending())
    }

    fn update(
        (): &mut Self::State,
        _: Self::Action,
    ) -> impl runenui_core::IntoUpdateOutput<Self::Action, Self::HostProtocol> {
    }
}

#[test]
fn pending_local_task_does_not_prevent_quiescence() {
    let mut runtime = AppRuntime::<PendingTaskApp>::mount(())
        .unwrap_or_else(|error| unreachable!("unexpected initial admission refusal: {error}"));
    let report = runtime
        .pump(PumpBudget::new(8, usize::MAX, usize::MAX, usize::MAX))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();
    assert_eq!(report.processed_envelopes(), 2);
    assert!(report.is_quiescent());
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum LocalCreditAction {
    CancelWithChildren,
    Fanout,
    Child(u8),
    TaskReady,
}

struct PendingLocalFuture(Rc<Cell<usize>>);

impl Future for PendingLocalFuture {
    type Output = Option<LocalCreditAction>;

    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        self.0.set(self.0.get() + 1);
        Poll::Pending
    }
}

struct LocalCancelApp;

impl UiApp for LocalCancelApp {
    fn initial_producer_budget(_state: &Self::State) -> runenui_core::ProducerBudget {
        runenui_core::ProducerBudget::new(4, 3)
            .with_reconciliation(1, 0, 1)
            .with_subscriptions(0, 0)
            .with_work_starts(runenui_core::WorkStartBudget::new(1, 0, 0, 0, 0))
            .with_work_residency(runenui_core::WorkResidencyBudget::new(1, 0, 0, 0, 0, 0))
    }

    fn action_producer_budget(
        _state: &Self::State,
        action: &Self::Action,
    ) -> runenui_core::ProducerBudget {
        let (credits, outputs) = match action {
            LocalCreditAction::CancelWithChildren => (4, 3),
            LocalCreditAction::Child(_) | LocalCreditAction::TaskReady => (1, 0),
            LocalCreditAction::Fanout => (3, 3),
        };
        runenui_core::ProducerBudget::new(credits, outputs)
            .with_subscriptions(0, 0)
            .with_work_starts(runenui_core::WorkStartBudget::new(0, 0, 0, 0, 0))
    }

    fn interaction_producer_budget(_state: &Self::State) -> runenui_core::ProducerBudget {
        runenui_core::ProducerBudget::new(1, 0)
    }

    type State = (Vec<LocalCreditAction>, Rc<Cell<usize>>);
    type Action = LocalCreditAction;
    type HostProtocol = NoHostProtocol;

    fn root(_: &Self::State) -> impl View<Self::Action> {
        text("local task credits")
    }

    fn initial_effects(state: &Self::State) -> impl IntoEffects<Self::Action, Self::HostProtocol> {
        Effects::action(LocalCreditAction::CancelWithChildren).then(Effects::keyed_local_task(
            WorkKey::new("local.credit").unwrap_or_else(|_| unreachable!()),
            PendingLocalFuture(Rc::clone(&state.1)),
        ))
    }

    fn update(
        state: &mut Self::State,
        action: Self::Action,
    ) -> impl runenui_core::IntoUpdateOutput<Self::Action, Self::HostProtocol> {
        state.0.push(action);
        match action {
            LocalCreditAction::CancelWithChildren => Effects::cancel(
                WorkFamily::LocalTask,
                WorkKey::new("local.credit").unwrap_or_else(|_| unreachable!()),
            )
            .then(Effects::action(LocalCreditAction::Child(1)))
            .then(Effects::action(LocalCreditAction::Child(2))),
            LocalCreditAction::Child(_)
            | LocalCreditAction::Fanout
            | LocalCreditAction::TaskReady => Effects::none(),
        }
    }
}

#[test]
fn pending_local_task_cancellation_keeps_fifo_start_and_uses_protected_credit_after_fanout() {
    let polls = Rc::new(Cell::new(0));
    let mut runtime = AppRuntime::<LocalCancelApp>::mount((Vec::new(), Rc::clone(&polls)))
        .unwrap_or_else(|error| unreachable!("unexpected initial admission refusal: {error}"));

    let first = runtime
        .pump(PumpBudget::new(2, 0, 0, 0))
        .unwrap_or_else(|_| unreachable!("initial reconcile and cancel action are processed"))
        .report()
        .to_owned();
    assert_eq!(first.processed_envelopes(), 2);
    assert_eq!(polls.get(), 0, "the still-pending Start is never polled");
    assert!(
        matches!(runtime.status(), RuntimeStatus::Running),
        "status: {:?}; trace: {:#?}",
        runtime.status(),
        runtime
            .trace()
            .records()
            .map(runenui_runtime::TraceRecord::kind)
            .collect::<Vec<_>>()
    );

    let drained = runtime
        .pump(PumpBudget::new(8, usize::MAX, usize::MAX, usize::MAX))
        .unwrap_or_else(|_| unreachable!("stale Start, tail cancellation, and children drain"))
        .report()
        .to_owned();
    assert_eq!(drained.processed_envelopes(), 4);
    assert_eq!(polls.get(), 0);
    assert_eq!(
        runtime.state().0,
        [
            LocalCreditAction::CancelWithChildren,
            LocalCreditAction::Child(1),
            LocalCreditAction::Child(2),
        ]
    );
    assert!(matches!(runtime.status(), RuntimeStatus::Running));

    let records: Vec<_> = runtime.trace().records().collect();
    let root_action = records
        .iter()
        .find(|record| matches!(record.kind(), TraceRecordKind::ActionSubmissionAccepted))
        .unwrap_or_else(|| unreachable!("initial root action acceptance is traced"));
    let cancellation = records
        .iter()
        .find(|record| matches!(record.kind(), TraceRecordKind::WorkCleanupProcessed))
        .unwrap_or_else(|| unreachable!("the tail cancellation is processed"));
    let child_sequences = records
        .iter()
        .filter(|record| matches!(record.kind(), TraceRecordKind::ActionSubmissionAccepted))
        .filter_map(|record| record.work_sequence())
        .filter(|sequence| {
            sequence.get()
                > cancellation
                    .work_sequence()
                    .unwrap_or_else(|| unreachable!())
                    .get()
        })
        .count();
    assert!(
        cancellation
            .work_sequence()
            .unwrap_or_else(|| unreachable!())
            .get()
            > root_action
                .work_sequence()
                .unwrap_or_else(|| unreachable!())
                .get()
    );
    assert_eq!(child_sequences, 2);
    assert!(
        !records
            .iter()
            .any(|record| matches!(record.kind(), TraceRecordKind::LocalWorkPolled))
    );
    assert!(!records.iter().any(|record| {
        matches!(record.kind(), TraceRecordKind::WorkStartAccepted)
            && record
                .work()
                .is_some_and(|identity| identity.family() == TraceWorkFamily::LocalTask)
    }));
}

struct LocalOutcomeApp;

impl UiApp for LocalOutcomeApp {
    fn initial_producer_budget(_state: &Self::State) -> runenui_core::ProducerBudget {
        runenui_core::ProducerBudget::new(4, 3)
            .with_reconciliation(1, 0, 1)
            .with_subscriptions(0, 0)
            .with_work_starts(runenui_core::WorkStartBudget::new(1, 0, 0, 0, 0))
            .with_work_residency(runenui_core::WorkResidencyBudget::new(1, 0, 0, 0, 0, 0))
    }

    fn action_producer_budget(
        _state: &Self::State,
        action: &Self::Action,
    ) -> runenui_core::ProducerBudget {
        let (credits, outputs) = match action {
            LocalCreditAction::Fanout => (3, 3),
            LocalCreditAction::Child(_) | LocalCreditAction::TaskReady => (1, 0),
            LocalCreditAction::CancelWithChildren => (3, 2),
        };
        runenui_core::ProducerBudget::new(credits, outputs)
            .with_subscriptions(0, 0)
            .with_work_starts(runenui_core::WorkStartBudget::new(0, 0, 0, 0, 0))
    }

    fn interaction_producer_budget(_state: &Self::State) -> runenui_core::ProducerBudget {
        runenui_core::ProducerBudget::new(1, 0)
    }

    type State = Vec<LocalCreditAction>;
    type Action = LocalCreditAction;
    type HostProtocol = NoHostProtocol;

    fn root(_: &Self::State) -> impl View<Self::Action> {
        text("local outcome credits")
    }

    fn initial_effects(_: &Self::State) -> impl IntoEffects<Self::Action, Self::HostProtocol> {
        Effects::keyed_local_task(
            WorkKey::new("local.outcome").unwrap_or_else(|_| unreachable!()),
            async { Some(LocalCreditAction::TaskReady) },
        )
        .then(Effects::action(LocalCreditAction::Fanout))
    }

    fn update(
        state: &mut Self::State,
        action: Self::Action,
    ) -> impl runenui_core::IntoUpdateOutput<Self::Action, Self::HostProtocol> {
        state.push(action);
        if action == LocalCreditAction::Fanout {
            Effects::action(LocalCreditAction::Child(1))
                .then(Effects::action(LocalCreditAction::Child(2)))
                .then(Effects::action(LocalCreditAction::Child(3)))
        } else {
            Effects::none()
        }
    }
}

#[test]
fn local_task_ready_action_uses_its_future_credit_when_queue_has_no_p() {
    let config = RuntimeConfig::default().with_queue_capacity(4);
    let mut runtime = AppRuntime::<LocalOutcomeApp>::mount_with_config(Vec::new(), config)
        .unwrap_or_else(|error| unreachable!("unexpected initial admission refusal: {error}"));

    let fanout = runtime
        .pump(PumpBudget::new(3, 0, 0, 0))
        .unwrap_or_else(|_| unreachable!("start and fanout commit before local polling"))
        .report()
        .to_owned();
    assert_eq!(fanout.processed_envelopes(), 3);
    assert_eq!(
        fanout.remaining_queued_envelopes(),
        3,
        "status: {:?}; trace: {:#?}",
        runtime.status(),
        runtime
            .trace()
            .records()
            .map(runenui_runtime::TraceRecord::kind)
            .collect::<Vec<_>>()
    );
    assert!(matches!(runtime.status(), RuntimeStatus::Running));

    let ready = runtime
        .pump(PumpBudget::new(0, 0, 1, 0))
        .unwrap_or_else(|_| unreachable!("LocalTask outcome credit replaces one reserved slot"))
        .report()
        .to_owned();
    assert_eq!(ready.polled_local_work(), 1);
    assert_eq!(ready.remaining_queued_envelopes(), 4);
    assert!(matches!(runtime.status(), RuntimeStatus::Running));

    let drained = runtime
        .pump(PumpBudget::new(4, usize::MAX, usize::MAX, usize::MAX))
        .unwrap_or_else(|_| unreachable!("three children and one task action drain"))
        .report()
        .to_owned();
    assert_eq!(drained.processed_envelopes(), 4);
    assert_eq!(
        runtime.state(),
        &[
            LocalCreditAction::Fanout,
            LocalCreditAction::Child(1),
            LocalCreditAction::Child(2),
            LocalCreditAction::Child(3),
            LocalCreditAction::TaskReady,
        ]
    );
    assert!(matches!(runtime.status(), RuntimeStatus::Running));
}

struct TimerApp;

struct UnderfundedTimerApp;

impl UiApp for UnderfundedTimerApp {
    fn initial_producer_budget(_state: &Self::State) -> runenui_core::ProducerBudget {
        // Two timer starts need two start Q credits, two cancellation credits,
        // and one dirty-owner credit before any external start can commit.
        runenui_core::ProducerBudget::new(4, 2)
            .with_reconciliation(1, 0, 1)
            .with_subscriptions(0, 0)
            .with_work_starts(runenui_core::WorkStartBudget::new(0, 0, 2, 0, 0))
            .with_work_residency(runenui_core::WorkResidencyBudget::new(0, 0, 2, 0, 0, 0))
    }

    fn action_producer_budget(
        _state: &Self::State,
        _action: &Self::Action,
    ) -> runenui_core::ProducerBudget {
        runenui_core::ProducerBudget::new(1, 0)
    }

    fn interaction_producer_budget(_state: &Self::State) -> runenui_core::ProducerBudget {
        runenui_core::ProducerBudget::new(1, 0)
    }

    type State = Rc<Cell<usize>>;
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> impl View<Self::Action> {
        state.set(state.get() + 1);
        text("underfunded timers")
    }

    fn initial_effects(_: &Self::State) -> impl IntoEffects<Self::Action, Self::HostProtocol> {
        Effects::timer(TimerEffect::once(Duration::from_millis(5), || ())).then(Effects::timer(
            TimerEffect::once(Duration::from_millis(5), || ()),
        ))
    }

    fn update(
        _: &mut Self::State,
        (): Self::Action,
    ) -> impl runenui_core::IntoUpdateOutput<Self::Action, Self::HostProtocol> {
    }
}

#[test]
fn initial_timer_future_credits_are_checked_before_external_start() {
    let root_calls = Rc::new(Cell::new(0));
    let Err(error) = AppRuntime::<UnderfundedTimerApp>::mount(Rc::clone(&root_calls)) else {
        unreachable!("two cancellation futures do not fit the declared frontier")
    };
    assert_eq!(
        error.kind(),
        InitialAdmissionErrorKind::ProducerContractViolation(
            runenui_core::ProducerContractViolation {
                kind: runenui_core::ProducerViolationKind::DeclarationExceeded,
                phase: runenui_core::ProducerPhase::InitialEffects,
                component: runenui_core::BudgetComponent::QueueFrontier,
                declared: 4,
                attempted: Some(5),
            }
        )
    );
    assert_eq!(
        root_calls.get(),
        1,
        "APP-02 mounts the root before collecting actual initial Effects"
    );
    let (state, _config) = error.into_parts();
    assert!(Rc::ptr_eq(&state, &root_calls));
}

impl UiApp for TimerApp {
    fn initial_producer_budget(_state: &Self::State) -> runenui_core::ProducerBudget {
        // Two timer starts share one dirty initial owner. Each start Q funds
        // its future firing slot; each live one-shot retains a cancel credit.
        runenui_core::ProducerBudget::new(5, 2)
            .with_reconciliation(1, 0, 1)
            .with_subscriptions(0, 0)
            .with_work_starts(runenui_core::WorkStartBudget::new(0, 0, 2, 0, 0))
            .with_work_residency(runenui_core::WorkResidencyBudget::new(0, 0, 2, 0, 0, 0))
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

    type State = Vec<&'static str>;
    type Action = Action;
    type HostProtocol = NoHostProtocol;

    fn root(_: &Self::State) -> impl View<Self::Action> {
        text("timers")
    }

    fn initial_effects(_: &Self::State) -> impl IntoEffects<Self::Action, Self::HostProtocol> {
        Effects::timer(TimerEffect::once(Duration::from_millis(5), || {
            Action::Label("first")
        }))
        .then(Effects::timer(TimerEffect::once(
            Duration::from_millis(5),
            || Action::Label("second"),
        )))
    }

    fn update(
        state: &mut Self::State,
        action: Self::Action,
    ) -> impl runenui_core::IntoUpdateOutput<Self::Action, Self::HostProtocol> {
        if let Action::Label(label) = action {
            state.push(label);
        }
    }
}

#[test]
fn equal_deadline_timers_fire_in_creation_order() {
    let mut runtime = AppRuntime::<TimerApp>::mount(Vec::new())
        .unwrap_or_else(|error| unreachable!("unexpected initial admission refusal: {error}"));
    assert!(
        runtime
            .pump(PumpBudget::new(3, usize::MAX, usize::MAX, usize::MAX))
            .unwrap_or_else(|_| unreachable!("pump observation"))
            .report()
            .to_owned()
            .is_quiescent()
    );
    runtime
        .advance_time(Duration::from_millis(5))
        .unwrap_or_else(|_| unreachable!());
    assert!(
        runtime
            .pump(PumpBudget::new(8, usize::MAX, usize::MAX, usize::MAX))
            .unwrap_or_else(|_| unreachable!("pump observation"))
            .report()
            .to_owned()
            .is_quiescent()
    );
    assert_eq!(runtime.state(), &["first", "second"]);
}

#[test]
fn sleeping_timer_future_credit_is_quiescent_and_isolated_from_another_action_root() {
    let mut runtime = AppRuntime::<TimerApp>::mount(Vec::new())
        .unwrap_or_else(|error| unreachable!("unexpected initial admission refusal: {error}"));
    let idle = runtime
        .pump(PumpBudget::new(3, usize::MAX, usize::MAX, usize::MAX))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();
    assert!(idle.is_quiescent());
    assert!(idle.next_deadline().is_some());

    runtime
        .submit_action(Action::Label("unrelated"))
        .unwrap_or_else(|_| unreachable!("independent action is admitted"));
    runtime
        .advance_time(Duration::from_millis(5))
        .unwrap_or_else(|_| unreachable!());
    let _ = runtime
        .pump(PumpBudget::new(8, usize::MAX, usize::MAX, usize::MAX))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();

    assert_eq!(runtime.state(), &["unrelated", "first", "second"]);
    assert_eq!(runtime.status(), RuntimeStatus::Running);
}

struct RepeatingTimerApp;

impl UiApp for RepeatingTimerApp {
    fn initial_producer_budget(_state: &Self::State) -> runenui_core::ProducerBudget {
        // The repeating timer retains distinct callback and cancellation
        // credits; its start Q becomes the first firing credit.
        runenui_core::ProducerBudget::new(4, 1)
            .with_reconciliation(1, 0, 1)
            .with_subscriptions(0, 0)
            .with_work_starts(runenui_core::WorkStartBudget::new(0, 0, 1, 0, 0))
            .with_work_residency(runenui_core::WorkResidencyBudget::new(0, 0, 1, 0, 0, 0))
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
        text("repeat")
    }

    fn initial_effects(_: &Self::State) -> impl IntoEffects<Self::Action, Self::HostProtocol> {
        Effects::timer(TimerEffect::repeating(Duration::from_millis(10), || ()))
    }

    fn update(
        state: &mut Self::State,
        (): Self::Action,
    ) -> impl runenui_core::IntoUpdateOutput<Self::Action, Self::HostProtocol> {
        *state += 1;
    }
}

#[test]
fn repeating_timer_coalesces_missed_ticks_from_logical_deadline() {
    let mut runtime = AppRuntime::<RepeatingTimerApp>::mount(0)
        .unwrap_or_else(|error| unreachable!("unexpected initial admission refusal: {error}"));
    let _ = runtime
        .pump(PumpBudget::new(2, usize::MAX, usize::MAX, usize::MAX))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();
    runtime
        .advance_time(Duration::from_millis(35))
        .unwrap_or_else(|_| unreachable!());
    let _report = runtime
        .pump(PumpBudget::new(8, usize::MAX, usize::MAX, usize::MAX))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();
    assert_eq!(*runtime.state(), 1);
    runtime
        .advance_time(Duration::from_millis(4))
        .unwrap_or_else(|_| unreachable!());
    assert_eq!(
        runtime
            .pump(PumpBudget::new(8, usize::MAX, usize::MAX, usize::MAX))
            .unwrap_or_else(|_| unreachable!("pump observation"))
            .report()
            .to_owned()
            .processed_envelopes(),
        0
    );
    runtime
        .advance_time(Duration::from_millis(1))
        .unwrap_or_else(|_| unreachable!());
    let _ = runtime
        .pump(PumpBudget::new(8, usize::MAX, usize::MAX, usize::MAX))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();
    assert_eq!(*runtime.state(), 2);

    let requested = family_fact(&runtime, TraceWorkFamily::Timer, |kind| {
        matches!(kind, TraceRecordKind::WorkRequested)
    });
    let committed = family_fact(&runtime, TraceWorkFamily::Timer, |kind| {
        matches!(kind, TraceRecordKind::WorkGenerationCommitted)
    });
    let attempted = family_fact(&runtime, TraceWorkFamily::Timer, |kind| {
        matches!(kind, TraceRecordKind::WorkStartAttempted)
    });
    let accepted = family_fact(&runtime, TraceWorkFamily::Timer, |kind| {
        matches!(kind, TraceRecordKind::WorkStartAccepted)
    });
    let promoted = family_fact(&runtime, TraceWorkFamily::Timer, |kind| {
        matches!(kind, TraceRecordKind::TimerPromoted)
    });
    let fired = family_fact(&runtime, TraceWorkFamily::Timer, |kind| {
        matches!(kind, TraceRecordKind::TimerFired)
    });
    assert_scheduler_step(requested, committed);
    assert_scheduler_step(committed, attempted);
    assert_scheduler_step(attempted, accepted);
    assert_scheduler_step(accepted, promoted);
    assert_scheduler_step(promoted, fired);
    assert_final_action_chain(&runtime, fired);
}

struct OverflowingRepeatingTimerApp;

impl UiApp for OverflowingRepeatingTimerApp {
    fn initial_producer_budget(_state: &Self::State) -> runenui_core::ProducerBudget {
        runenui_core::ProducerBudget::new(4, 1)
            .with_reconciliation(1, 0, 1)
            .with_subscriptions(0, 0)
            .with_work_starts(runenui_core::WorkStartBudget::new(0, 0, 1, 0, 0))
            .with_work_residency(runenui_core::WorkResidencyBudget::new(0, 0, 1, 0, 0, 0))
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
        text("overflowing repeat")
    }

    fn initial_effects(_: &Self::State) -> impl IntoEffects<Self::Action, Self::HostProtocol> {
        Effects::timer(TimerEffect::repeating(Duration::from_nanos(2), || ()))
    }

    fn update(
        state: &mut Self::State,
        (): Self::Action,
    ) -> impl runenui_core::IntoUpdateOutput<Self::Action, Self::HostProtocol> {
        *state += 1;
    }
}

#[test]
fn repeating_deadline_overflow_terminates_only_the_timer_after_current_firing() {
    let mut runtime = AppRuntime::<OverflowingRepeatingTimerApp>::mount(0)
        .unwrap_or_else(|error| unreachable!("unexpected initial admission refusal: {error}"));
    let _ = runtime
        .pump(PumpBudget::new(2, usize::MAX, usize::MAX, usize::MAX))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();
    runtime
        .advance_time(Duration::from_nanos(u64::MAX))
        .unwrap_or_else(|_| unreachable!());

    let report = runtime
        .pump(PumpBudget::new(8, usize::MAX, usize::MAX, usize::MAX))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();

    assert_eq!(*runtime.state(), 1);
    assert_eq!(runtime.status(), RuntimeStatus::Running);
    assert_eq!(
        runtime.last_timer_firing_outcome(),
        Some(TimerFiringOutcome::RepeatDeadlineOverflow)
    );
    assert!(report.is_quiescent());
    assert!(runtime.trace().kinds().any(|kind| matches!(
        kind,
        TraceRecordKind::TimerTerminated {
            outcome: TraceTimerTerminalOutcome::RepeatDeadlineOverflow
        }
    )));
    assert_eq!(
        runtime
            .pump(PumpBudget::new(8, usize::MAX, usize::MAX, usize::MAX))
            .unwrap_or_else(|_| unreachable!("pump observation"))
            .report()
            .to_owned()
            .processed_envelopes(),
        0
    );
}

struct ZeroTimerApp;

impl UiApp for ZeroTimerApp {
    fn initial_producer_budget(_state: &Self::State) -> runenui_core::ProducerBudget {
        runenui_core::ProducerBudget::new(4, 1)
            .with_reconciliation(1, 0, 1)
            .with_subscriptions(0, 0)
            .with_work_starts(runenui_core::WorkStartBudget::new(0, 0, 1, 0, 0))
            .with_work_residency(runenui_core::WorkResidencyBudget::new(0, 0, 1, 0, 0, 0))
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

    type State = ();
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root((): &Self::State) -> impl View<Self::Action> {
        text("zero")
    }

    fn initial_effects((): &Self::State) -> impl IntoEffects<Self::Action, Self::HostProtocol> {
        Effects::timer(TimerEffect::repeating(Duration::ZERO, || ()))
    }

    fn update(
        (): &mut Self::State,
        (): Self::Action,
    ) -> impl runenui_core::IntoUpdateOutput<Self::Action, Self::HostProtocol> {
    }
}

#[test]
fn zero_repeating_interval_is_explicit_and_non_poisoning() {
    let mut runtime = AppRuntime::<ZeroTimerApp>::mount(())
        .unwrap_or_else(|error| unreachable!("unexpected initial admission refusal: {error}"));
    assert!(
        runtime
            .pump(PumpBudget::new(2, usize::MAX, usize::MAX, usize::MAX))
            .unwrap_or_else(|_| unreachable!("pump observation"))
            .report()
            .to_owned()
            .is_quiescent()
    );
    assert_eq!(
        runtime.last_timer_start_outcome(),
        Some(TimerStartOutcome::ZeroInterval)
    );
}

#[derive(Clone)]
struct HoldingExecutor(Rc<RefCell<Vec<SendTaskJob>>>);

impl SendTaskExecutor for HoldingExecutor {
    fn start(&mut self, job: SendTaskJob) -> Result<(), SendTaskStartError> {
        self.0.borrow_mut().push(job);
        Ok(())
    }
}

fn run_ready(job: SendTaskJob) -> Result<(), SendTaskCompletionError> {
    let mut future = Box::pin(job.run());
    let mut context = Context::from_waker(Waker::noop());
    match Pin::new(&mut future).poll(&mut context) {
        Poll::Ready(result) => result,
        Poll::Pending => unreachable!(),
    }
}

struct SendTaskApp;

impl UiApp for SendTaskApp {
    fn initial_producer_budget(_state: &Self::State) -> runenui_core::ProducerBudget {
        runenui_core::ProducerBudget::new(3, 1)
            .with_reconciliation(1, 0, 1)
            .with_subscriptions(0, 0)
            .with_work_starts(runenui_core::WorkStartBudget::new(0, 1, 0, 0, 0))
            .with_work_residency(runenui_core::WorkResidencyBudget::new(0, 1, 0, 0, 0, 0))
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
    type Action = Action;
    type HostProtocol = NoHostProtocol;

    fn root(_: &Self::State) -> impl View<Self::Action> {
        text("send")
    }

    fn initial_effects(_: &Self::State) -> impl IntoEffects<Self::Action, Self::HostProtocol> {
        Effects::send_task(async { 7_u8 }, |_| Action::NonSend(Rc::new(())))
    }

    fn update(
        state: &mut Self::State,
        action: Self::Action,
    ) -> impl runenui_core::IntoUpdateOutput<Self::Action, Self::HostProtocol> {
        if let Action::NonSend(value) = action {
            *state += Rc::strong_count(&value);
        }
    }
}

#[test]
fn send_task_transports_output_without_requiring_action_send() {
    let jobs = Rc::new(RefCell::new(Vec::new()));
    let mut runtime = AppRuntime::<SendTaskApp>::mount(0)
        .unwrap_or_else(|error| unreachable!("unexpected initial admission refusal: {error}"));
    runtime.set_send_task_executor(HoldingExecutor(Rc::clone(&jobs)));
    let _ = runtime
        .pump(PumpBudget::new(2, usize::MAX, usize::MAX, usize::MAX))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();
    assert_eq!(
        runtime.last_send_task_start_outcome(),
        Some(SendTaskStartOutcome::Started)
    );
    let job = jobs.borrow_mut().pop().unwrap_or_else(|| unreachable!());
    run_ready(job).unwrap_or_else(|_| unreachable!());
    let _ = runtime
        .pump(PumpBudget::new(8, usize::MAX, usize::MAX, usize::MAX))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();
    assert_eq!(*runtime.state(), 1);

    let requested = family_fact(&runtime, TraceWorkFamily::SendTask, |kind| {
        matches!(kind, TraceRecordKind::WorkRequested)
    });
    let committed = family_fact(&runtime, TraceWorkFamily::SendTask, |kind| {
        matches!(kind, TraceRecordKind::WorkGenerationCommitted)
    });
    let attempted = family_fact(&runtime, TraceWorkFamily::SendTask, |kind| {
        matches!(kind, TraceRecordKind::WorkStartAttempted)
    });
    let accepted = family_fact(&runtime, TraceWorkFamily::SendTask, |kind| {
        matches!(kind, TraceRecordKind::WorkStartAccepted)
    });
    let imported = family_fact(&runtime, TraceWorkFamily::SendTask, |kind| {
        matches!(kind, TraceRecordKind::WorkCompletionImported)
    });
    let mapped = family_fact(&runtime, TraceWorkFamily::SendTask, |kind| {
        matches!(kind, TraceRecordKind::WorkCompletionMapped)
    });
    assert_scheduler_step(requested, committed);
    assert_scheduler_step(committed, attempted);
    assert_scheduler_step(attempted, accepted);
    assert_scheduler_step(accepted, imported);
    assert_scheduler_step(imported, mapped);
    assert_final_action_chain(&runtime, mapped);
}

struct RefusalApp;

impl UiApp for RefusalApp {
    fn initial_producer_budget(_state: &Self::State) -> runenui_core::ProducerBudget {
        runenui_core::ProducerBudget::new(3, 1)
            .with_reconciliation(1, 0, 1)
            .with_subscriptions(0, 0)
            .with_work_starts(runenui_core::WorkStartBudget::new(0, 1, 0, 0, 0))
            .with_work_residency(runenui_core::WorkResidencyBudget::new(0, 1, 0, 0, 0, 0))
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

    type State = Option<SendTaskStartFailure>;
    type Action = SendTaskStartFailure;
    type HostProtocol = NoHostProtocol;

    fn root(_: &Self::State) -> impl View<Self::Action> {
        text("refusal")
    }

    fn initial_effects(_: &Self::State) -> impl IntoEffects<Self::Action, Self::HostProtocol> {
        Effects::send_task_with_failure(async { 1_u8 }, |_| unreachable!(), |failure| failure)
    }

    fn update(
        state: &mut Self::State,
        action: Self::Action,
    ) -> impl runenui_core::IntoUpdateOutput<Self::Action, Self::HostProtocol> {
        *state = Some(action);
    }
}

#[test]
fn executor_refusal_is_terminal_for_work_but_recoverable_for_runtime() {
    let mut runtime = AppRuntime::<RefusalApp>::mount(None)
        .unwrap_or_else(|error| unreachable!("unexpected initial admission refusal: {error}"));
    assert!(
        runtime
            .pump(PumpBudget::new(8, usize::MAX, usize::MAX, usize::MAX))
            .unwrap_or_else(|_| unreachable!("pump observation"))
            .report()
            .to_owned()
            .is_quiescent()
    );
    assert_eq!(
        runtime.last_send_task_start_outcome(),
        Some(SendTaskStartOutcome::Unavailable)
    );
    assert_eq!(runtime.state(), &Some(SendTaskStartFailure::Unavailable));
}

#[cfg(feature = "internal-test-seams")]
#[derive(Clone, Copy)]
enum RefusalKind {
    Unavailable,
    Full,
    Closed,
    Rejected,
}

#[cfg(feature = "internal-test-seams")]
struct RefusingExecutor {
    kind: RefusalKind,
    calls: Rc<Cell<usize>>,
}

#[cfg(feature = "internal-test-seams")]
impl SendTaskExecutor for RefusingExecutor {
    fn start(&mut self, job: SendTaskJob) -> Result<(), SendTaskStartError> {
        self.calls.set(self.calls.get() + 1);
        Err(match self.kind {
            RefusalKind::Unavailable => SendTaskStartError::Unavailable(job),
            RefusalKind::Full => SendTaskStartError::Full(job),
            RefusalKind::Closed => SendTaskStartError::Closed(job),
            RefusalKind::Rejected => SendTaskStartError::Rejected(job),
        })
    }
}

#[cfg(feature = "internal-test-seams")]
fn assert_executor_refusal(
    kind: RefusalKind,
    expected_start: SendTaskStartOutcome,
    expected_failure: SendTaskStartFailure,
    expected_trace: TraceWorkStartRefusal,
) {
    let calls = Rc::new(Cell::new(0));
    let mut runtime = AppRuntime::<RefusalApp>::mount(None)
        .unwrap_or_else(|error| unreachable!("unexpected initial admission refusal: {error}"));
    runtime.set_send_task_executor(RefusingExecutor {
        kind,
        calls: Rc::clone(&calls),
    });

    assert!(
        runtime
            .pump(PumpBudget::new(8, usize::MAX, usize::MAX, usize::MAX))
            .unwrap_or_else(|_| unreachable!("pump observation"))
            .report()
            .to_owned()
            .is_quiescent()
    );
    assert_eq!(calls.get(), 1);
    assert_eq!(runtime.last_send_task_start_outcome(), Some(expected_start));
    assert_eq!(runtime.state(), &Some(expected_failure));
    assert_eq!(runtime.__live_work_record_count_for_test(), 0);
    let refusal = runtime
        .trace()
        .records()
        .find(|record| {
            matches!(
                record.kind(),
                TraceRecordKind::WorkStartRefused { outcome } if *outcome == expected_trace
            )
        })
        .unwrap_or_else(|| unreachable!());
    assert!(refusal.work().is_some());
    let requested = family_fact(&runtime, TraceWorkFamily::SendTask, |record| {
        matches!(record, TraceRecordKind::WorkRequested)
    });
    let committed = family_fact(&runtime, TraceWorkFamily::SendTask, |record| {
        matches!(record, TraceRecordKind::WorkGenerationCommitted)
    });
    let attempted = family_fact(&runtime, TraceWorkFamily::SendTask, |record| {
        matches!(record, TraceRecordKind::WorkStartAttempted)
    });
    assert_scheduler_step(requested, committed);
    assert_scheduler_step(committed, attempted);
    assert_scheduler_step(attempted, refusal);
    assert_final_action_chain(&runtime, refusal);
}

#[cfg(feature = "internal-test-seams")]
#[test]
fn unavailable_executor_refuses_once_and_reclaims_generation() {
    assert_executor_refusal(
        RefusalKind::Unavailable,
        SendTaskStartOutcome::Unavailable,
        SendTaskStartFailure::Unavailable,
        TraceWorkStartRefusal::ExecutorUnavailable,
    );
}

#[cfg(feature = "internal-test-seams")]
#[test]
fn full_executor_refuses_once_and_reclaims_generation() {
    assert_executor_refusal(
        RefusalKind::Full,
        SendTaskStartOutcome::Full,
        SendTaskStartFailure::Full,
        TraceWorkStartRefusal::ExecutorFull,
    );
}

#[cfg(feature = "internal-test-seams")]
#[test]
fn closed_executor_refuses_once_and_reclaims_generation() {
    assert_executor_refusal(
        RefusalKind::Closed,
        SendTaskStartOutcome::Closed,
        SendTaskStartFailure::Closed,
        TraceWorkStartRefusal::ExecutorClosed,
    );
}

#[cfg(feature = "internal-test-seams")]
#[test]
fn rejected_executor_refuses_once_and_reclaims_generation() {
    assert_executor_refusal(
        RefusalKind::Rejected,
        SendTaskStartOutcome::Rejected,
        SendTaskStartFailure::Rejected,
        TraceWorkStartRefusal::ExecutorRejected,
    );
}

#[test]
fn full_completion_ingress_returns_exact_retryable_completion() {
    let jobs = Rc::new(RefCell::new(Vec::new()));
    let limits = RuntimeLimits::default().with_completion_ingress(0);
    let mut runtime = AppRuntime::<SendTaskApp>::mount_with_config(
        0,
        RuntimeConfig::default().with_limits(limits),
    )
    .unwrap_or_else(|error| unreachable!("unexpected initial admission refusal: {error}"));
    runtime.set_send_task_executor(HoldingExecutor(Rc::clone(&jobs)));
    let _ = runtime
        .pump(PumpBudget::new(2, usize::MAX, usize::MAX, usize::MAX))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();
    let job = jobs.borrow_mut().pop().unwrap_or_else(|| unreachable!());
    let Err(SendTaskCompletionError::Full(completion)) = run_ready(job) else {
        unreachable!()
    };
    let _ = runtime
        .shutdown()
        .unwrap_or_else(|_| unreachable!("shutdown observation"))
        .report()
        .to_owned();
    assert!(matches!(
        completion.submit(),
        Err(SendTaskCompletionError::Closed(_))
    ));
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum IngressReservationAction {
    Start(u8),
    Complete(u8),
}

struct IngressReservationApp;

impl UiApp for IngressReservationApp {
    fn initial_producer_budget(_state: &Self::State) -> runenui_core::ProducerBudget {
        runenui_core::ProducerBudget::new(1, 0)
            .with_reconciliation(1, 0, 1)
            .with_subscriptions(0, 0)
    }

    fn action_producer_budget(
        _state: &Self::State,
        action: &Self::Action,
    ) -> runenui_core::ProducerBudget {
        match action {
            IngressReservationAction::Start(_) => runenui_core::ProducerBudget::new(2, 1)
                .with_subscriptions(0, 0)
                .with_work_starts(runenui_core::WorkStartBudget::new(0, 1, 0, 0, 0))
                .with_work_residency(runenui_core::WorkResidencyBudget::new(0, 1, 0, 0, 0, 0)),
            IngressReservationAction::Complete(_) => runenui_core::ProducerBudget::new(1, 0)
                .with_subscriptions(0, 0)
                .with_work_starts(runenui_core::WorkStartBudget::default()),
        }
    }

    fn interaction_producer_budget(_state: &Self::State) -> runenui_core::ProducerBudget {
        runenui_core::ProducerBudget::new(1, 0)
    }

    type State = Vec<u8>;
    type Action = IngressReservationAction;
    type HostProtocol = NoHostProtocol;

    fn root(_: &Self::State) -> impl View<Self::Action> {
        text("ingress reservation")
    }

    fn initial_effects(_: &Self::State) -> impl IntoEffects<Self::Action, Self::HostProtocol> {
        Effects::none()
    }

    fn update(
        state: &mut Self::State,
        action: Self::Action,
    ) -> impl runenui_core::IntoUpdateOutput<Self::Action, Self::HostProtocol> {
        match action {
            IngressReservationAction::Start(value) => {
                Effects::send_task(async move { value }, IngressReservationAction::Complete)
            }
            IngressReservationAction::Complete(value) => {
                state.push(value);
                Effects::none()
            }
        }
    }
}

#[test]
fn independent_send_task_roots_can_receive_typed_ingress_backpressure() {
    let jobs = Rc::new(RefCell::new(Vec::new()));
    let limits = RuntimeLimits::default().with_completion_ingress(1);
    let mut runtime = AppRuntime::<IngressReservationApp>::mount_with_config(
        Vec::new(),
        RuntimeConfig::default()
            .with_queue_capacity(8)
            .with_limits(limits),
    )
    .unwrap_or_else(|error| unreachable!("unexpected initial admission refusal: {error}"));
    runtime.set_send_task_executor(HoldingExecutor(Rc::clone(&jobs)));

    runtime
        .submit_action(IngressReservationAction::Start(1))
        .unwrap_or_else(|_| unreachable!("first independent action root is admitted"));
    runtime
        .submit_action(IngressReservationAction::Start(2))
        .unwrap_or_else(|_| unreachable!("second independent action root is admitted"));
    let _ = runtime
        .pump(PumpBudget::new(8, 0, 0, 0))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();
    let mut jobs = jobs.borrow_mut();
    assert_eq!(jobs.len(), 2);
    let first = jobs.pop().unwrap_or_else(|| unreachable!());
    let second = jobs.pop().unwrap_or_else(|| unreachable!());
    assert!(run_ready(first).is_ok());
    let Err(SendTaskCompletionError::Full(completion)) = run_ready(second) else {
        unreachable!("ingress saturation returns the exact completion")
    };
    drop(jobs);
    assert_eq!(runtime.__completion_payload_count_for_test(), 1);
    assert!(matches!(
        completion.submit(),
        Err(SendTaskCompletionError::Full(_))
    ));
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SameRootSendAction {
    Fanout,
    Leaf(u8),
    Complete(u8),
}

struct SameRootSendApp;

impl UiApp for SameRootSendApp {
    fn initial_producer_budget(_state: &Self::State) -> runenui_core::ProducerBudget {
        runenui_core::ProducerBudget::new(6, 6)
            .with_reconciliation(1, 0, 1)
            .with_subscriptions(0, 0)
            .with_work_starts(runenui_core::WorkStartBudget::new(0, 1, 0, 0, 0))
            .with_work_residency(runenui_core::WorkResidencyBudget::new(0, 1, 0, 0, 0, 0))
    }

    fn action_producer_budget(
        _state: &Self::State,
        action: &Self::Action,
    ) -> runenui_core::ProducerBudget {
        let (credits, outputs) = match action {
            SameRootSendAction::Fanout => (6, 5),
            SameRootSendAction::Leaf(_) | SameRootSendAction::Complete(_) => (1, 0),
        };
        runenui_core::ProducerBudget::new(credits, outputs)
            .with_subscriptions(0, 0)
            .with_work_starts(runenui_core::WorkStartBudget::default())
    }

    fn interaction_producer_budget(_state: &Self::State) -> runenui_core::ProducerBudget {
        runenui_core::ProducerBudget::new(1, 0)
    }

    type State = Vec<u8>;
    type Action = SameRootSendAction;
    type HostProtocol = NoHostProtocol;

    fn root(_: &Self::State) -> impl View<Self::Action> {
        text("same root send task")
    }

    fn initial_effects(_: &Self::State) -> impl IntoEffects<Self::Action, Self::HostProtocol> {
        Effects::send_task(async { 7_u8 }, SameRootSendAction::Complete)
            .then(Effects::action(SameRootSendAction::Fanout))
    }

    fn update(
        state: &mut Self::State,
        action: Self::Action,
    ) -> impl runenui_core::IntoUpdateOutput<Self::Action, Self::HostProtocol> {
        match action {
            SameRootSendAction::Fanout => Effects::action(SameRootSendAction::Leaf(1))
                .then(Effects::action(SameRootSendAction::Leaf(2)))
                .then(Effects::action(SameRootSendAction::Leaf(3)))
                .then(Effects::action(SameRootSendAction::Leaf(4)))
                .then(Effects::action(SameRootSendAction::Leaf(5))),
            SameRootSendAction::Leaf(_) => Effects::none(),
            SameRootSendAction::Complete(value) => {
                state.push(value);
                Effects::none()
            }
        }
    }
}

#[test]
fn same_root_send_completion_waits_when_sibling_outputs_exhaust_p() {
    let jobs = Rc::new(RefCell::new(Vec::new()));
    let limits = RuntimeLimits::default().with_completion_ingress(1);
    let mut runtime = AppRuntime::<SameRootSendApp>::mount_with_config(
        Vec::new(),
        RuntimeConfig::default().with_limits(limits),
    )
    .unwrap_or_else(|error| unreachable!("unexpected initial admission refusal: {error}"));
    runtime.set_send_task_executor(HoldingExecutor(Rc::clone(&jobs)));
    let _ = runtime
        .pump(PumpBudget::new(3, 0, 0, 0))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();
    assert_eq!(
        jobs.borrow().len(),
        1,
        "start outcome={:?} trace={:?}",
        runtime.last_send_task_start_outcome(),
        runtime.trace().kinds().collect::<Vec<_>>()
    );
    let job = jobs.borrow_mut().pop().unwrap_or_else(|| unreachable!());
    let completion_result = run_ready(job);
    assert!(
        completion_result.is_ok(),
        "send slots={} trace={:?}",
        runtime.__send_task_slot_count_for_test(),
        runtime.trace().kinds().collect::<Vec<_>>()
    );

    let _ = runtime
        .pump(PumpBudget::new(0, 1, 0, 0))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();
    assert_eq!(runtime.__completion_payload_count_for_test(), 0);
    assert_eq!(runtime.state(), &Vec::<u8>::new());

    let _ = runtime
        .pump(PumpBudget::new(1, 0, 0, 0))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();
    let _ = runtime
        .pump(PumpBudget::new(0, 1, 0, 0))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();
    let _ = runtime
        .pump(PumpBudget::new(8, 0, 0, 0))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();
    assert_eq!(
        runtime.state(),
        &[7],
        "completion_payloads={} trace={:?}",
        runtime.__completion_payload_count_for_test(),
        runtime.trace().kinds().collect::<Vec<_>>()
    );
}

#[cfg(feature = "internal-test-seams")]
struct SendMapperIntegrityApp;

#[cfg(feature = "internal-test-seams")]
impl UiApp for SendMapperIntegrityApp {
    fn initial_producer_budget(_state: &Self::State) -> runenui_core::ProducerBudget {
        runenui_core::ProducerBudget::new(3, 1)
            .with_reconciliation(1, 0, 1)
            .with_subscriptions(0, 0)
            .with_work_starts(runenui_core::WorkStartBudget::new(0, 1, 0, 0, 0))
            .with_work_residency(runenui_core::WorkResidencyBudget::new(0, 1, 0, 0, 0, 0))
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
        text("send mapper integrity")
    }

    fn initial_effects(state: &Self::State) -> impl IntoEffects<Self::Action, Self::HostProtocol> {
        let calls = Rc::clone(state);
        Effects::send_task(async { 1_u8 }, move |_| {
            calls.set(calls.get() + 1);
        })
    }

    fn update(
        _: &mut Self::State,
        (): Self::Action,
    ) -> impl runenui_core::IntoUpdateOutput<Self::Action, Self::HostProtocol> {
    }
}

#[cfg(feature = "internal-test-seams")]
#[test]
fn work_sequence_exhaustion_prevents_send_completion_mapper() {
    let calls = Rc::new(Cell::new(0));
    let jobs = Rc::new(RefCell::new(Vec::new()));
    let mut runtime = AppRuntime::<SendMapperIntegrityApp>::mount(Rc::clone(&calls))
        .unwrap_or_else(|error| unreachable!("unexpected initial admission refusal: {error}"));
    runtime.set_send_task_executor(HoldingExecutor(Rc::clone(&jobs)));
    let _ = runtime
        .pump(PumpBudget::new(2, 0, 0, 0))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();
    run_ready(jobs.borrow_mut().pop().unwrap_or_else(|| unreachable!()))
        .unwrap_or_else(|_| unreachable!());
    runtime.__seed_next_work_sequence_for_test(0);

    let _ = runtime
        .pump(PumpBudget::new(0, 1, 0, 0))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();

    assert_eq!(calls.get(), 0);
    assert_eq!(
        runtime.status(),
        RuntimeStatus::Terminal(RuntimeTerminalReason::WorkSequenceExhausted)
    );
}

#[cfg(feature = "internal-test-seams")]
fn accepted_last_sequence<App: UiApp>(runtime: &AppRuntime<App>) -> usize {
    runtime
        .trace()
        .records()
        .filter(|record| {
            matches!(record.kind(), TraceRecordKind::ActionSubmissionAccepted)
                && record
                    .work_sequence()
                    .is_some_and(|sequence| sequence.get() == u64::MAX)
        })
        .count()
}

#[cfg(feature = "internal-test-seams")]
#[test]
#[allow(clippy::assert_is_empty)]
fn one_remaining_sequence_is_consumed_only_by_each_final_scheduler_action() {
    let mut local = AppRuntime::<LocalTaskApp>::mount(Vec::new())
        .unwrap_or_else(|error| unreachable!("unexpected initial admission refusal: {error}"));
    let _ = local
        .pump(PumpBudget::new(2, 0, 0, 0))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();
    local.__seed_next_work_sequence_for_test(u64::MAX);
    let _ = local
        .pump(PumpBudget::new(0, 0, 1, 0))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();
    assert_eq!(accepted_last_sequence(&local), 1);
    assert_eq!(local.status(), RuntimeStatus::Running);
    assert!(local.state().is_empty());

    let jobs = Rc::new(RefCell::new(Vec::new()));
    let mut send = AppRuntime::<SendTaskApp>::mount(0)
        .unwrap_or_else(|error| unreachable!("unexpected initial admission refusal: {error}"));
    send.set_send_task_executor(HoldingExecutor(Rc::clone(&jobs)));
    let _ = send
        .pump(PumpBudget::new(2, 0, 0, 0))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();
    run_ready(jobs.borrow_mut().pop().unwrap_or_else(|| unreachable!()))
        .unwrap_or_else(|_| unreachable!());
    send.__seed_next_work_sequence_for_test(u64::MAX);
    let _ = send
        .pump(PumpBudget::new(0, 1, 0, 0))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();
    assert_eq!(accepted_last_sequence(&send), 1);
    assert_eq!(send.status(), RuntimeStatus::Running);
    assert_eq!(send.state(), &0);

    let mut timer = AppRuntime::<RepeatingTimerApp>::mount(0)
        .unwrap_or_else(|error| unreachable!("unexpected initial admission refusal: {error}"));
    let _ = timer
        .pump(PumpBudget::new(2, 0, 0, 0))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();
    timer
        .advance_time(Duration::from_millis(10))
        .unwrap_or_else(|_| unreachable!());
    let _ = timer
        .pump(PumpBudget::new(0, 0, 0, 1))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();
    timer.__seed_next_work_sequence_for_test(u64::MAX);
    let _ = timer
        .pump(PumpBudget::new(1, 0, 0, 0))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();
    assert_eq!(accepted_last_sequence(&timer), 1);
    assert_eq!(timer.status(), RuntimeStatus::Running);
    assert_eq!(timer.state(), &0);

    let refusal_calls = Rc::new(Cell::new(0));
    let mut refusal = AppRuntime::<RefusalApp>::mount(None)
        .unwrap_or_else(|error| unreachable!("unexpected initial admission refusal: {error}"));
    refusal.set_send_task_executor(RefusingExecutor {
        kind: RefusalKind::Unavailable,
        calls: Rc::clone(&refusal_calls),
    });
    refusal.__seed_next_work_sequence_for_test(u64::MAX);
    let _ = refusal
        .pump(PumpBudget::new(2, 0, 0, 0))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();
    assert_eq!(refusal_calls.get(), 1);
    assert_eq!(accepted_last_sequence(&refusal), 1);
    assert_eq!(refusal.status(), RuntimeStatus::Running);
    assert_eq!(refusal.state(), &None);
}

struct SendCancelState {
    mapper_calls: Rc<Cell<usize>>,
    completed_updates: usize,
}

struct SendCancelApp;

impl UiApp for SendCancelApp {
    fn initial_producer_budget(_state: &Self::State) -> runenui_core::ProducerBudget {
        runenui_core::ProducerBudget::new(3, 1)
            .with_reconciliation(1, 0, 1)
            .with_subscriptions(0, 0)
            .with_work_starts(runenui_core::WorkStartBudget::new(0, 1, 0, 0, 0))
            .with_work_residency(runenui_core::WorkResidencyBudget::new(0, 1, 0, 0, 0, 0))
    }

    fn action_producer_budget(
        _state: &Self::State,
        action: &Self::Action,
    ) -> runenui_core::ProducerBudget {
        runenui_core::ProducerBudget::new(1, usize::from(!*action))
            .with_subscriptions(0, 0)
            .with_work_starts(runenui_core::WorkStartBudget::new(0, 0, 0, 0, 0))
    }

    fn interaction_producer_budget(_state: &Self::State) -> runenui_core::ProducerBudget {
        runenui_core::ProducerBudget::new(1, 0)
    }

    type State = SendCancelState;
    type Action = bool;
    type HostProtocol = NoHostProtocol;

    fn root(_: &Self::State) -> impl View<Self::Action> {
        text("send-cancel")
    }

    fn initial_effects(state: &Self::State) -> impl IntoEffects<Self::Action, Self::HostProtocol> {
        let calls = Rc::clone(&state.mapper_calls);
        Effects::keyed_send_task(send_key(), async { 1_u8 }, move |_| {
            calls.set(calls.get() + 1);
            true
        })
    }

    fn update(
        state: &mut Self::State,
        completed: Self::Action,
    ) -> impl runenui_core::IntoUpdateOutput<Self::Action, Self::HostProtocol> {
        if completed {
            state.completed_updates += 1;
            Effects::none()
        } else {
            Effects::cancel(WorkFamily::SendTask, send_key())
        }
    }
}

fn send_key() -> WorkKey {
    WorkKey::new("cancel.send").unwrap_or_else(|_| unreachable!())
}

#[test]
fn cancelled_send_completion_never_invokes_ui_mapper() {
    let mapper_calls = Rc::new(Cell::new(0));
    let jobs = Rc::new(RefCell::new(Vec::new()));
    let mut runtime = AppRuntime::<SendCancelApp>::mount(SendCancelState {
        mapper_calls: Rc::clone(&mapper_calls),
        completed_updates: 0,
    })
    .unwrap_or_else(|error| unreachable!("unexpected initial admission refusal: {error}"));
    runtime.set_send_task_executor(HoldingExecutor(Rc::clone(&jobs)));
    let _ = runtime
        .pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();
    runtime
        .submit_action(false)
        .unwrap_or_else(|_| unreachable!());
    let _ = runtime
        .pump(PumpBudget::new(2, usize::MAX, usize::MAX, usize::MAX))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();

    let job = jobs.borrow_mut().pop().unwrap_or_else(|| unreachable!());
    assert!(matches!(
        run_ready(job),
        Err(SendTaskCompletionError::Stale(_))
    ));
    assert!(
        runtime
            .pump(PumpBudget::new(8, usize::MAX, usize::MAX, usize::MAX))
            .unwrap_or_else(|_| unreachable!("pump observation"))
            .report()
            .to_owned()
            .is_quiescent()
    );
    assert_eq!(mapper_calls.get(), 0);
    assert_eq!(runtime.state().completed_updates, 0);

    assert!(!runtime.trace().kinds().any(|kind| matches!(
        kind,
        TraceRecordKind::WorkCompletionImported | TraceRecordKind::WorkCompletionRejectedStale
    )));
}

#[cfg(feature = "internal-test-seams")]
fn trace_boundary_send_task() -> (AppRuntime<SendCancelApp>, Rc<Cell<usize>>, SendTaskJob) {
    let mapper_calls = Rc::new(Cell::new(0));
    let jobs = Rc::new(RefCell::new(Vec::new()));
    let mut runtime = AppRuntime::<SendCancelApp>::mount(SendCancelState {
        mapper_calls: Rc::clone(&mapper_calls),
        completed_updates: 0,
    })
    .unwrap_or_else(|error| unreachable!("unexpected initial admission refusal: {error}"));
    runtime.set_send_task_executor(HoldingExecutor(Rc::clone(&jobs)));
    let _ = runtime
        .pump(PumpBudget::new(8, usize::MAX, usize::MAX, usize::MAX))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();
    let job = jobs
        .borrow_mut()
        .pop()
        .unwrap_or_else(|| unreachable!("executor retained one exact job"));
    (runtime, mapper_calls, job)
}

#[cfg(feature = "internal-test-seams")]
#[test]
fn send_task_completion_admits_its_exact_three_record_plan_beside_publication_authority() {
    let (mut runtime, mapper_calls, job) = trace_boundary_send_task();
    run_ready(job).unwrap_or_else(|_| unreachable!("live completion enters ingress"));
    assert!(runtime.__surface_publication_trace_reserved_for_test());
    runtime.__seed_next_trace_sequence_for_test(u64::MAX - 3);
    let _ = runtime
        .pump(PumpBudget::new(0, 1, 0, 0))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();

    assert_eq!(mapper_calls.get(), 1);
    assert_eq!(runtime.status(), RuntimeStatus::Running);
    assert_eq!(runtime.__send_task_slot_count_for_test(), 0);
    assert_eq!(runtime.__send_task_mapper_count_for_test(), 0);
    let kinds: Vec<_> = runtime.trace().kinds().collect();
    let tail = &kinds[kinds.len() - 3..];
    assert!(matches!(tail[0], TraceRecordKind::WorkCompletionImported));
    assert!(matches!(tail[1], TraceRecordKind::WorkCompletionMapped));
    assert!(matches!(tail[2], TraceRecordKind::ActionSubmissionAccepted));
}

#[cfg(feature = "internal-test-seams")]
#[test]
fn send_task_completion_with_only_two_unreserved_records_never_runs_mapper() {
    let (mut runtime, mapper_calls, job) = trace_boundary_send_task();
    run_ready(job).unwrap_or_else(|_| unreachable!("live completion enters ingress"));
    assert!(runtime.__surface_publication_trace_reserved_for_test());
    runtime.__seed_next_trace_sequence_for_test(u64::MAX - 2);
    let _ = runtime
        .pump(PumpBudget::new(0, 1, 0, 0))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();

    assert_eq!(mapper_calls.get(), 0);
    assert_eq!(
        runtime.status(),
        RuntimeStatus::Terminal(RuntimeTerminalReason::TraceSequenceExhausted)
    );
    assert_eq!(runtime.__send_task_slot_count_for_test(), 0);
    assert_eq!(runtime.__send_task_mapper_count_for_test(), 0);
    assert_eq!(runtime.__completion_payload_count_for_test(), 0);
}

#[test]
#[allow(clippy::assert_is_empty)]
fn disabled_trace_changes_no_send_completion_behavior() {
    let mapper_calls = Rc::new(Cell::new(0));
    let jobs = Rc::new(RefCell::new(Vec::new()));
    let mut runtime = AppRuntime::<SendCancelApp>::mount_with_config(
        SendCancelState {
            mapper_calls: Rc::clone(&mapper_calls),
            completed_updates: 0,
        },
        RuntimeConfig::default().with_trace_config(TraceConfig::new(0)),
    )
    .unwrap_or_else(|error| unreachable!("unexpected initial admission refusal: {error}"));
    runtime.set_send_task_executor(HoldingExecutor(Rc::clone(&jobs)));
    let _ = runtime
        .pump(PumpBudget::new(8, usize::MAX, usize::MAX, usize::MAX))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();
    let job = jobs
        .borrow_mut()
        .pop()
        .unwrap_or_else(|| unreachable!("executor retained one exact job"));
    run_ready(job).unwrap_or_else(|_| unreachable!("live completion enters ingress"));
    let _ = runtime
        .pump(PumpBudget::new(0, 1, 0, 0))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();

    assert_eq!(mapper_calls.get(), 1);
    assert_eq!(runtime.status(), RuntimeStatus::Running);
    assert!(runtime.trace().is_empty());
}

struct CancelTimerApp;

impl UiApp for CancelTimerApp {
    fn initial_producer_budget(_state: &Self::State) -> runenui_core::ProducerBudget {
        // The pending timer start and dirty owner each use one Q slot; the
        // admitted timer also retains its future cancellation credit.
        runenui_core::ProducerBudget::new(3, 1)
            .with_reconciliation(1, 0, 1)
            .with_subscriptions(0, 0)
            .with_work_starts(runenui_core::WorkStartBudget::new(0, 0, 1, 0, 0))
            .with_work_residency(runenui_core::WorkResidencyBudget::new(0, 0, 1, 0, 0, 0))
    }

    fn action_producer_budget(
        _state: &Self::State,
        fired: &Self::Action,
    ) -> runenui_core::ProducerBudget {
        runenui_core::ProducerBudget::new(1, usize::from(!*fired))
            .with_subscriptions(0, 0)
            .with_work_starts(runenui_core::WorkStartBudget::new(0, 0, 0, 0, 0))
    }

    fn interaction_producer_budget(_state: &Self::State) -> runenui_core::ProducerBudget {
        runenui_core::ProducerBudget::new(1, 0)
    }

    type State = usize;
    type Action = bool;
    type HostProtocol = NoHostProtocol;

    fn root(_: &Self::State) -> impl View<Self::Action> {
        text("cancel")
    }

    fn initial_effects(_: &Self::State) -> impl IntoEffects<Self::Action, Self::HostProtocol> {
        Effects::timer(TimerEffect::once(Duration::from_millis(5), || true).keyed(timer_key()))
    }

    fn update(
        state: &mut Self::State,
        fired: Self::Action,
    ) -> impl runenui_core::IntoUpdateOutput<Self::Action, Self::HostProtocol> {
        if fired {
            *state += 1;
            Effects::none()
        } else {
            Effects::cancel(WorkFamily::Timer, timer_key())
        }
    }
}

fn timer_key() -> WorkKey {
    WorkKey::new("cancel.timer").unwrap_or_else(|_| unreachable!())
}

#[test]
fn cancellation_before_timer_firing_suppresses_action_factory_result() {
    let mut runtime = AppRuntime::<CancelTimerApp>::mount(0)
        .unwrap_or_else(|error| unreachable!("unexpected initial admission refusal: {error}"));
    let _ = runtime
        .pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();
    runtime
        .submit_action(false)
        .unwrap_or_else(|_| unreachable!());
    let _ = runtime
        .pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();
    runtime
        .advance_time(Duration::from_millis(5))
        .unwrap_or_else(|_| unreachable!());
    let _ = runtime
        .pump(PumpBudget::new(8, usize::MAX, usize::MAX, usize::MAX))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();
    assert_eq!(*runtime.state(), 0);
}

#[test]
fn cancellation_after_timer_promotion_keeps_the_stale_firing_fifo_owned() {
    let mut runtime = AppRuntime::<CancelTimerApp>::mount(0)
        .unwrap_or_else(|error| unreachable!("unexpected initial admission refusal: {error}"));
    let _ = runtime
        .pump(PumpBudget::new(2, usize::MAX, usize::MAX, usize::MAX))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();
    runtime
        .advance_time(Duration::from_millis(5))
        .unwrap_or_else(|_| unreachable!());
    runtime
        .submit_action(false)
        .unwrap_or_else(|_| unreachable!("cancel action admission"));
    let promoted = runtime
        .pump(PumpBudget::new(0, 0, 0, 1))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();
    assert_eq!(promoted.promoted_timers(), 1);
    assert_eq!(promoted.processed_envelopes(), 0);

    let _ = runtime
        .pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();
    let _ = runtime
        .pump(PumpBudget::new(8, usize::MAX, usize::MAX, usize::MAX))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();

    assert_eq!(*runtime.state(), 0);
    assert_eq!(runtime.status(), RuntimeStatus::Running);
}

#[derive(Clone, Copy)]
enum TimerCancellationAction {
    Fired,
    Cancel,
}

struct TimerCancellationState {
    fired_updates: usize,
    factory_calls: std::rc::Rc<std::cell::Cell<usize>>,
}

struct RepeatingTimerCancellationApp;

impl UiApp for RepeatingTimerCancellationApp {
    fn initial_producer_budget(_state: &Self::State) -> runenui_core::ProducerBudget {
        // One initial dirty owner and one timer start Q, plus distinct
        // repeating callback and cancellation future credits. The start Q
        // becomes the first firing credit when its FIFO envelope is popped.
        runenui_core::ProducerBudget::new(5, 1)
            .with_reconciliation(1, 0, 1)
            .with_subscriptions(0, 0)
            .with_work_starts(runenui_core::WorkStartBudget::new(0, 0, 1, 0, 0))
            .with_work_residency(runenui_core::WorkResidencyBudget::new(0, 0, 1, 0, 0, 0))
    }

    fn action_producer_budget(
        _state: &Self::State,
        action: &Self::Action,
    ) -> runenui_core::ProducerBudget {
        runenui_core::ProducerBudget::new(
            1,
            usize::from(matches!(action, TimerCancellationAction::Cancel)),
        )
        .with_subscriptions(0, 0)
        .with_work_starts(runenui_core::WorkStartBudget::new(0, 0, 0, 0, 0))
    }

    fn interaction_producer_budget(_state: &Self::State) -> runenui_core::ProducerBudget {
        runenui_core::ProducerBudget::new(1, 0)
    }

    type State = TimerCancellationState;
    type Action = TimerCancellationAction;
    type HostProtocol = NoHostProtocol;

    fn root(_: &Self::State) -> impl View<Self::Action> {
        text("repeating timer cancellation")
    }

    fn initial_effects(state: &Self::State) -> impl IntoEffects<Self::Action, Self::HostProtocol> {
        let factory_calls = std::rc::Rc::clone(&state.factory_calls);
        Effects::timer(
            TimerEffect::repeating(Duration::from_millis(5), move || {
                factory_calls.set(factory_calls.get() + 1);
                TimerCancellationAction::Fired
            })
            .keyed(timer_key()),
        )
    }

    fn update(
        state: &mut Self::State,
        action: Self::Action,
    ) -> impl runenui_core::IntoUpdateOutput<Self::Action, Self::HostProtocol> {
        match action {
            TimerCancellationAction::Fired => {
                state.fired_updates += 1;
                Effects::none()
            }
            TimerCancellationAction::Cancel => Effects::cancel(WorkFamily::Timer, timer_key()),
        }
    }
}

#[test]
fn repeating_timer_callback_and_next_firing_keep_distinct_credits_until_fifo_cancellation() {
    let factory_calls = std::rc::Rc::new(std::cell::Cell::new(0));
    let mut runtime = AppRuntime::<RepeatingTimerCancellationApp>::mount(TimerCancellationState {
        fired_updates: 0,
        factory_calls: std::rc::Rc::clone(&factory_calls),
    })
    .unwrap_or_else(|error| unreachable!("unexpected initial admission refusal: {error}"));
    let _ = runtime
        .pump(PumpBudget::new(4, usize::MAX, usize::MAX, usize::MAX))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();

    runtime
        .advance_time(Duration::from_millis(5))
        .unwrap_or_else(|_| unreachable!());
    let _ = runtime
        .pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();
    assert_eq!(factory_calls.get(), 1);
    assert_eq!(runtime.state().fired_updates, 0);

    runtime
        .advance_time(Duration::from_millis(5))
        .unwrap_or_else(|_| unreachable!());
    runtime
        .submit_action(TimerCancellationAction::Cancel)
        .unwrap_or_else(|_| unreachable!("timer cancellation action is admitted"));
    let promoted = runtime
        .pump(PumpBudget::new(0, 0, 0, 1))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();
    assert_eq!(promoted.promoted_timers(), 1);
    assert_eq!(promoted.processed_envelopes(), 0);

    let _ = runtime
        .pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();
    assert_eq!(runtime.state().fired_updates, 1);
    let _ = runtime
        .pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();
    let _ = runtime
        .pump(PumpBudget::new(8, usize::MAX, usize::MAX, usize::MAX))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();

    assert_eq!(factory_calls.get(), 1);
    assert_eq!(runtime.state().fired_updates, 1);
    assert_eq!(runtime.status(), RuntimeStatus::Running);
}
