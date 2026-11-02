#![allow(refining_impl_trait)]

use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    task::{Poll, Waker},
};

use runenui_core::{
    Effects, NoHostProtocol, ProducerBudget, UiApp, View, WorkResidencyBudget, WorkStartBudget,
    text,
};
use runenui_runtime::{
    AppRuntime, InitialAdmissionErrorKind, PumpBudget, RuntimeConfig, RuntimeLimits, RuntimeStatus,
    SubmitActionErrorKind, TraceConfig, TraceRecordKind, WorkCapacityFamily,
};

enum Action {
    Start(Box<u64>),
    StartChain { residency: usize },
    Claim(WorkResidencyBudget),
    Noop,
}

struct State {
    started: Vec<u64>,
    ready: Rc<Cell<bool>>,
    wake: Rc<RefCell<Option<Waker>>>,
}

struct App;
impl UiApp for App {
    type State = State;
    type Action = Action;
    type HostProtocol = NoHostProtocol;
    fn initial_producer_budget(_: &State) -> ProducerBudget {
        ProducerBudget::new(1, 0)
            .with_reconciliation(1, 0, 1)
            .with_subscriptions(0, 0)
    }
    fn action_producer_budget(_: &State, action: &Action) -> ProducerBudget {
        match action {
            // This closed producer graph starts one local future, which only
            // completes with None. It cannot spawn another task or action.
            // Four FIFO credits cover the action/start/outcome/cancellation.
            Action::Start(_) => ProducerBudget::new(4, 1)
                .with_work_starts(WorkStartBudget::new(1, 0, 0, 0, 0))
                .with_work_residency(WorkResidencyBudget::new(1, 0, 0, 0, 0, 0)),
            // One start in each of two app turns, retaining both until polling.
            Action::StartChain { residency } => ProducerBudget::new(8, 2)
                .with_work_starts(WorkStartBudget::new(1, 0, 0, 0, 0))
                .with_work_residency(WorkResidencyBudget::new(*residency, 0, 0, 0, 0, 0)),
            Action::Claim(residency) => ProducerBudget::new(1, 0).with_work_residency(*residency),
            Action::Noop => ProducerBudget::new(1, 0),
        }
        .with_reconciliation(0, 1, 0)
        .with_subscriptions(0, 0)
    }
    fn interaction_producer_budget(_: &State) -> ProducerBudget {
        ProducerBudget::new(1, 0)
            .with_reconciliation(0, 1, 0)
            .with_subscriptions(0, 0)
    }
    fn root(_: &State) -> impl View<Action> {
        text("one pure leaf")
    }
    fn update(state: &mut State, action: Action) -> Effects<Action, NoHostProtocol> {
        match action {
            Action::Noop | Action::Claim(_) => Effects::none(),
            Action::StartChain { .. } => {
                state.started.push(1);
                pending_local_task(state).then(Effects::action(Action::Start(Box::new(2))))
            }
            Action::Start(identity) => {
                state.started.push(*identity);
                pending_local_task(state)
            }
        }
    }
}

fn pending_local_task(state: &State) -> Effects<Action, NoHostProtocol> {
    let ready = Rc::clone(&state.ready);
    let wake = Rc::clone(&state.wake);
    Effects::local_task(std::future::poll_fn(move |context| {
        if ready.get() {
            Poll::Ready(None)
        } else {
            *wake.borrow_mut() = Some(context.waker().clone());
            Poll::Pending
        }
    }))
}

const FULL: PumpBudget = PumpBudget::new(usize::MAX, usize::MAX, usize::MAX, usize::MAX);

fn refused_start(app: &mut AppRuntime<App>, identity: u64) {
    let payload = Box::new(identity);
    let allocation = std::ptr::from_ref(payload.as_ref());
    let Err(refused) = app.submit_action(Action::Start(payload)) else {
        unreachable!("an accepted pending root already owns the only local-task slot")
    };
    assert_eq!(
        refused.kind(),
        SubmitActionErrorKind::WorkCapacity(WorkCapacityFamily::LocalTask)
    );
    let Action::Start(returned) = refused.into_action() else {
        unreachable!("exact action returned")
    };
    assert_eq!(std::ptr::from_ref(returned.as_ref()), allocation);
    assert_eq!(*returned, identity);
    assert_eq!(app.status(), RuntimeStatus::Running);
}

#[test]
fn accepted_root_keeps_local_task_capacity_before_materialization_and_until_completion() {
    for trace_capacity in [0, 256] {
        let ready = Rc::new(Cell::new(false));
        let wake = Rc::new(RefCell::new(None));
        let mut app = AppRuntime::<App>::mount_with_config(
            State {
                started: Vec::new(),
                ready: Rc::clone(&ready),
                wake: Rc::clone(&wake),
            },
            RuntimeConfig::default()
                .with_limits(
                    RuntimeLimits::default()
                        .with_waiting_envelopes(16)
                        .with_transaction_outputs(8)
                        .with_local_tasks(1),
                )
                .with_trace_config(TraceConfig::new(trace_capacity)),
        )
        .unwrap_or_else(|error| unreachable!("pure initial producer fits: {error}"));
        assert!(
            app.pump(FULL)
                .unwrap_or_else(|_| unreachable!())
                .report()
                .is_quiescent()
        );
        app.submit_action(Action::Start(Box::new(1)))
            .unwrap_or_else(|_| unreachable!("first task root fits"));
        refused_start(&mut app, 2);
        assert!(
            app.state().started.is_empty(),
            "refusal precedes both app updates"
        );
        if trace_capacity != 0 {
            assert_eq!(
                app.trace()
                    .kinds()
                    .filter(|kind| matches!(kind, TraceRecordKind::ActionSubmissionAccepted))
                    .count(),
                1
            );
        }
        app.submit_action(Action::Noop)
            .unwrap_or_else(|_| unreachable!("work-free producer still fits"));
        assert!(
            app.pump(FULL)
                .unwrap_or_else(|_| unreachable!())
                .report()
                .is_quiescent()
        );
        assert_eq!(app.state().started, [1]);
        refused_start(&mut app, 3);
        assert_eq!(app.state().started, [1]);

        ready.set(true);
        wake.borrow_mut()
            .take()
            .unwrap_or_else(|| unreachable!("pending future registered a waker"))
            .wake();
        assert!(
            app.pump(FULL)
                .unwrap_or_else(|_| unreachable!())
                .report()
                .is_quiescent()
        );
        app.submit_action(Action::Start(Box::new(4)))
            .unwrap_or_else(|_| unreachable!("completed work releases its family capacity"));
        assert!(
            app.pump(FULL)
                .unwrap_or_else(|_| unreachable!())
                .report()
                .is_quiescent()
        );
        assert_eq!(app.state().started, [1, 4]);
        assert_eq!(app.status(), RuntimeStatus::Running);
    }
}

#[test]
fn materialized_local_capacity_is_refused_before_application_update() {
    for trace_capacity in [0, 256] {
        let mut app = AppRuntime::<App>::mount_with_config(
            State {
                started: Vec::new(),
                ready: Rc::new(Cell::new(false)),
                wake: Rc::new(RefCell::new(None)),
            },
            RuntimeConfig::default()
                .with_limits(
                    RuntimeLimits::default()
                        .with_waiting_envelopes(16)
                        .with_transaction_outputs(8)
                        .with_local_tasks(1),
                )
                .with_trace_config(TraceConfig::new(trace_capacity)),
        )
        .unwrap_or_else(|error| unreachable!("pure initial producer fits: {error}"));
        app.pump(FULL).unwrap_or_else(|_| unreachable!());
        app.submit_action(Action::Start(Box::new(1)))
            .unwrap_or_else(|_| unreachable!("first task root fits"));
        app.pump(FULL).unwrap_or_else(|_| unreachable!());
        assert_eq!(app.state().started, [1]);
        assert_eq!(app.status(), RuntimeStatus::Running);

        let payload = Box::new(2);
        let allocation = std::ptr::from_ref(payload.as_ref());
        let submission = app.submit_action(Action::Start(payload));
        if submission.is_ok() {
            // Observe the complete accepted turn before checking the invariant:
            // admission must prevent ordinary pressure from reaching app code.
            let _ = app.pump(FULL);
        }
        assert_eq!(
            app.state().started,
            [1],
            "capacity refusal must precede arbitrary application mutation; status={:?}",
            app.status()
        );
        let Err(refused) = submission else {
            unreachable!("the only local-task slot is occupied")
        };
        assert_eq!(
            refused.kind(),
            SubmitActionErrorKind::WorkCapacity(WorkCapacityFamily::LocalTask)
        );
        let Action::Start(returned) = refused.into_action() else {
            unreachable!("exact action returned")
        };
        assert_eq!(std::ptr::from_ref(returned.as_ref()), allocation);
        assert_eq!(app.status(), RuntimeStatus::Running);
    }
}

#[test]
fn residency_covers_multiple_app_turns_independently_of_per_turn_starts() {
    for trace_capacity in [0, 256] {
        let ready = Rc::new(Cell::new(false));
        let mut app = AppRuntime::<App>::mount_with_config(
            State {
                started: Vec::new(),
                ready: Rc::clone(&ready),
                wake: Rc::new(RefCell::new(None)),
            },
            RuntimeConfig::default()
                .with_limits(
                    RuntimeLimits::default()
                        .with_waiting_envelopes(32)
                        .with_transaction_outputs(8)
                        .with_local_tasks(2),
                )
                .with_trace_config(TraceConfig::new(trace_capacity)),
        )
        .unwrap_or_else(|error| unreachable!("pure initial producer fits: {error}"));
        app.pump(FULL).unwrap_or_else(|_| unreachable!());
        app.submit_action(Action::StartChain { residency: 2 })
            .unwrap_or_else(|_| unreachable!("two-resident producer fits"));
        app.pump(PumpBudget::new(usize::MAX, 0, 0, 0))
            .unwrap_or_else(|_| unreachable!());
        assert_eq!(app.state().started, [1, 2]);
        assert_eq!(app.status(), RuntimeStatus::Running);
        refused_start(&mut app, 3);
        app.submit_action(Action::Noop)
            .unwrap_or_else(|_| unreachable!());
        ready.set(true);
        app.pump(FULL).unwrap_or_else(|_| unreachable!());
        app.submit_action(Action::Start(Box::new(4)))
            .unwrap_or_else(|_| unreachable!("both completed tasks release the root claim"));
        app.pump(FULL).unwrap_or_else(|_| unreachable!());
        assert_eq!(app.state().started, [1, 2, 4]);
        assert_eq!(app.status(), RuntimeStatus::Running);
    }
}

#[test]
fn understated_residency_is_integrity_failure_even_when_global_capacity_is_available() {
    for trace_capacity in [0, 256] {
        let mut app = AppRuntime::<App>::mount_with_config(
            State {
                started: Vec::new(),
                ready: Rc::new(Cell::new(false)),
                wake: Rc::new(RefCell::new(None)),
            },
            RuntimeConfig::default()
                .with_limits(
                    RuntimeLimits::default()
                        .with_waiting_envelopes(32)
                        .with_transaction_outputs(8)
                        .with_local_tasks(2),
                )
                .with_trace_config(TraceConfig::new(trace_capacity)),
        )
        .unwrap_or_else(|error| unreachable!("pure initial producer fits: {error}"));
        app.pump(FULL).unwrap_or_else(|_| unreachable!());
        app.submit_action(Action::StartChain { residency: 1 })
            .unwrap_or_else(|_| unreachable!());
        let _ = app.pump(PumpBudget::new(usize::MAX, 0, 0, 0));
        assert_eq!(
            app.status(),
            RuntimeStatus::Terminal(
                runenui_runtime::RuntimeTerminalReason::ProducerContractViolation(
                    runenui_core::ProducerContractViolation {
                        kind: runenui_core::ProducerViolationKind::DeclarationExceeded,
                        phase: runenui_core::ProducerPhase::ApplicationUpdate,
                        component: runenui_core::BudgetComponent::WorkResidency(
                            runenui_core::WorkFamily::LocalTask
                        ),
                        declared: 1,
                        attempted: Some(2)
                    }
                )
            )
        );
    }
}

struct InitialCapacityApp;
impl UiApp for InitialCapacityApp {
    type State = State;
    type Action = Action;
    type HostProtocol = NoHostProtocol;
    fn initial_producer_budget(state: &State) -> ProducerBudget {
        App::initial_producer_budget(state)
            .with_work_residency(WorkResidencyBudget::new(1, 0, 0, 0, 0, 0))
    }
    fn action_producer_budget(state: &State, action: &Action) -> ProducerBudget {
        App::action_producer_budget(state, action)
    }
    fn interaction_producer_budget(state: &State) -> ProducerBudget {
        App::interaction_producer_budget(state)
    }
    fn root(state: &State) -> impl View<Action> {
        state.ready.set(true);
        App::root(state)
    }
    fn initial_effects(state: &State) -> Effects<Action, NoHostProtocol> {
        state.ready.set(true);
        Effects::none()
    }
    fn update(state: &mut State, action: Action) -> Effects<Action, NoHostProtocol> {
        App::update(state, action)
    }
}

#[test]
fn initial_residency_refusal_returns_owned_state_before_initial_producers() {
    for trace_capacity in [0, 256] {
        let observer = Rc::new(Cell::new(false));
        let Err(refusal) = AppRuntime::<InitialCapacityApp>::mount_with_config(
            State {
                started: Vec::new(),
                ready: Rc::clone(&observer),
                wake: Rc::new(RefCell::new(None)),
            },
            RuntimeConfig::default()
                .with_limits(
                    RuntimeLimits::default()
                        .with_waiting_envelopes(16)
                        .with_transaction_outputs(8)
                        .with_local_tasks(0),
                )
                .with_trace_config(TraceConfig::new(trace_capacity)),
        ) else {
            unreachable!("one resident task cannot fit zero task slots")
        };
        assert_eq!(
            refusal.kind(),
            InitialAdmissionErrorKind::WorkCapacity(WorkCapacityFamily::LocalTask)
        );
        assert!(!observer.get(), "root and initial_effects must not run");
        assert!(Rc::ptr_eq(&refusal.state().ready, &observer));
        let (state, config) = refusal.into_parts();
        assert_eq!(config.limits().local_tasks(), 0);
        let mut app = AppRuntime::<InitialCapacityApp>::mount_with_config(
            state,
            config.with_limits(config.limits().with_local_tasks(1)),
        )
        .unwrap_or_else(|error| unreachable!("corrected capacity admits the same state: {error}"));
        assert!(observer.get());
        app.pump(FULL).unwrap_or_else(|_| unreachable!());
        app.submit_action(Action::Start(Box::new(1)))
            .unwrap_or_else(|_| unreachable!());
        app.pump(FULL).unwrap_or_else(|_| unreachable!());
        assert_eq!(app.state().started, [1]);
        assert_eq!(app.status(), RuntimeStatus::Running);
    }
}

#[test]
fn mixed_predecessor_roots_keep_work_and_owned_refusals_isolated() {
    for trace_capacity in [0, 256] {
        let ready = Rc::new(Cell::new(false));
        let wake = Rc::new(RefCell::new(None));
        let mut app = AppRuntime::<App>::mount_with_config(
            State {
                started: Vec::new(),
                ready: Rc::clone(&ready),
                wake: Rc::clone(&wake),
            },
            RuntimeConfig::default()
                .with_limits(
                    RuntimeLimits::default()
                        .with_waiting_envelopes(16)
                        .with_transaction_outputs(8)
                        .with_local_tasks(1)
                        .with_host_requests(1)
                        .with_timers(1)
                        .with_subscriptions(1),
                )
                .with_trace_config(TraceConfig::new(trace_capacity)),
        )
        .unwrap_or_else(|error| unreachable!("pure initial producer fits: {error}"));
        app.pump(FULL).unwrap_or_else(|_| unreachable!());

        let host = WorkResidencyBudget::new(0, 0, 0, 0, 1, 0);
        let timer = WorkResidencyBudget::new(0, 0, 1, 0, 0, 0);
        let subscription = WorkResidencyBudget::new(0, 0, 0, 1, 0, 0);
        app.submit_action(Action::Start(Box::new(11)))
            .unwrap_or_else(|_| unreachable!("predecessor owns a future local task"));
        for claim in [host, timer, subscription] {
            app.submit_action(Action::Claim(claim))
                .unwrap_or_else(|_| unreachable!("distinct family capacity is independent"));
        }
        let accepted_before = app
            .trace()
            .kinds()
            .filter(|kind| matches!(kind, TraceRecordKind::ActionSubmissionAccepted))
            .count();

        refused_start(&mut app, 12);
        let Err(refusal) = app.submit_action(Action::Claim(host)) else {
            unreachable!("the queued host root retains its reservation")
        };
        assert_eq!(
            refusal.kind(),
            SubmitActionErrorKind::WorkCapacity(WorkCapacityFamily::HostRequest)
        );
        let Action::Claim(returned) = refusal.into_action() else {
            unreachable!("the same original claim is returned")
        };
        assert_eq!(returned, host);
        assert_eq!(
            app.trace()
                .kinds()
                .filter(|kind| matches!(kind, TraceRecordKind::ActionSubmissionAccepted))
                .count(),
            accepted_before,
            "refusal must not accept a sequence or disturb prior acceptance"
        );
        assert!(
            app.state().started.is_empty(),
            "all refusals precede update"
        );

        app.submit_action(Action::Noop)
            .unwrap_or_else(|_| unreachable!("work-free follower is admissible"));
        app.pump(FULL).unwrap_or_else(|_| unreachable!());
        assert_eq!(app.state().started, [11]);
        assert_eq!(app.status(), RuntimeStatus::Running);
        refused_start(&mut app, 13);

        app.submit_action(Action::Claim(host))
            .unwrap_or_else(|_| unreachable!("completed host claim returns capacity"));
        app.pump(FULL).unwrap_or_else(|_| unreachable!());
        ready.set(true);
        wake.borrow_mut()
            .take()
            .unwrap_or_else(|| unreachable!("pending predecessor registered a waker"))
            .wake();
        app.pump(FULL).unwrap_or_else(|_| unreachable!());
        app.submit_action(Action::Start(Box::new(14)))
            .unwrap_or_else(|_| unreachable!("completed local task releases its claim"));
        app.pump(FULL).unwrap_or_else(|_| unreachable!());
        assert_eq!(app.state().started, [11, 14]);
        assert_eq!(app.status(), RuntimeStatus::Running);
        app.shutdown()
            .unwrap_or_else(|_| unreachable!("shutdown observation succeeds"));
    }
}

#[test]
fn all_six_family_claims_are_independent_and_released_with_queued_roots() {
    let claims = [
        (
            WorkCapacityFamily::LocalTask,
            WorkResidencyBudget::new(1, 0, 0, 0, 0, 0),
        ),
        (
            WorkCapacityFamily::SendTask,
            WorkResidencyBudget::new(0, 1, 0, 0, 0, 0),
        ),
        (
            WorkCapacityFamily::Timer,
            WorkResidencyBudget::new(0, 0, 1, 0, 0, 0),
        ),
        (
            WorkCapacityFamily::Subscription,
            WorkResidencyBudget::new(0, 0, 0, 1, 0, 0),
        ),
        (
            WorkCapacityFamily::HostRequest,
            WorkResidencyBudget::new(0, 0, 0, 0, 1, 0),
        ),
        (
            WorkCapacityFamily::FrameworkService,
            WorkResidencyBudget::new(0, 0, 0, 0, 0, 1),
        ),
    ];
    for trace_capacity in [0, 256] {
        for (index, (family, claim)) in claims.iter().enumerate() {
            let mut app = AppRuntime::<App>::mount_with_config(
                State {
                    started: Vec::new(),
                    ready: Rc::new(Cell::new(false)),
                    wake: Rc::new(RefCell::new(None)),
                },
                RuntimeConfig::default()
                    .with_limits(
                        RuntimeLimits::default()
                            .with_waiting_envelopes(16)
                            .with_transaction_outputs(8)
                            .with_local_tasks(1)
                            .with_send_tasks(1)
                            .with_timers(1)
                            .with_subscriptions(1)
                            .with_host_requests(1)
                            .with_framework_services(1),
                    )
                    .with_trace_config(TraceConfig::new(trace_capacity)),
            )
            .unwrap_or_else(|error| unreachable!("pure initial producer fits: {error}"));
            app.pump(FULL).unwrap_or_else(|_| unreachable!());
            app.submit_action(Action::Claim(*claim))
                .unwrap_or_else(|_| unreachable!());
            let Err(refusal) = app.submit_action(Action::Claim(*claim)) else {
                unreachable!("queued root reserves this family")
            };
            assert_eq!(refusal.kind(), SubmitActionErrorKind::WorkCapacity(*family));
            let Action::Claim(returned) = refusal.into_action() else {
                unreachable!()
            };
            assert_eq!(returned, *claim);
            app.submit_action(Action::Claim(claims[(index + 1) % claims.len()].1))
                .unwrap_or_else(|_| unreachable!("another work family remains available"));
            app.submit_action(Action::Noop)
                .unwrap_or_else(|_| unreachable!());
            app.pump(FULL).unwrap_or_else(|_| unreachable!());
            app.submit_action(Action::Claim(*claim))
                .unwrap_or_else(|_| {
                    unreachable!("completed root releases its unused family claim")
                });
            app.pump(FULL).unwrap_or_else(|_| unreachable!());
            assert_eq!(app.status(), RuntimeStatus::Running);
        }
    }
}
