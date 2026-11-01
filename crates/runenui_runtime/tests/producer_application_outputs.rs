#![allow(refining_impl_trait)]

use std::{cell::Cell, rc::Rc};

use runenui_core::{Effects, NoHostProtocol, ProducerBudget, UiApp, View, text};
use runenui_runtime::{
    AppRuntime, PumpBudget, RuntimeConfig, RuntimeLimits, RuntimeStatus, TraceConfig,
    TraceRecordKind,
};

enum Action {
    Fanout(Box<usize>),
    Overrun,
    EmitWideChild,
    EmitQualifiedChild,
    Noop,
}

struct State {
    updates: Rc<Cell<usize>>,
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
        let budget = match action {
            Action::Fanout(_) => ProducerBudget::new(5, 4),
            Action::Overrun => ProducerBudget::new(3, 2).with_application_outputs(1),
            Action::EmitWideChild => ProducerBudget::new(6, 4).with_application_outputs(1),
            Action::EmitQualifiedChild => ProducerBudget::new(6, 4),
            Action::Noop => ProducerBudget::new(1, 0),
        };
        budget.with_reconciliation(0, 1, 0).with_subscriptions(0, 0)
    }
    fn interaction_producer_budget(_: &State) -> ProducerBudget {
        ProducerBudget::new(1, 0)
            .with_reconciliation(0, 1, 0)
            .with_subscriptions(0, 0)
    }
    fn root(_: &State) -> impl View<Action> {
        text("one fixed pure leaf")
    }
    fn update(state: &mut State, action: Action) -> Effects<Action, NoHostProtocol> {
        state.updates.set(state.updates.get() + 1);
        match action {
            Action::Fanout(_) => Effects::action(Action::Noop)
                .then(Effects::action(Action::Noop))
                .then(Effects::action(Action::Noop))
                .then(Effects::action(Action::Noop)),
            Action::Overrun => Effects::action(Action::Noop).then(Effects::action(Action::Noop)),
            Action::EmitWideChild | Action::EmitQualifiedChild => {
                Effects::action(Action::Fanout(Box::new(17)))
            }
            Action::Noop => Effects::none(),
        }
    }
}

#[test]
fn application_component_overrun_poison_executes_no_staged_child() {
    for trace_capacity in [0, 256] {
        let updates = Rc::new(Cell::new(0));
        let mut app = AppRuntime::<App>::mount_with_config(
            State {
                updates: Rc::clone(&updates),
            },
            RuntimeConfig::default()
                .with_queue_capacity(16)
                .with_limits(
                    RuntimeLimits::default()
                        .with_waiting_envelopes(16)
                        .with_transaction_outputs(3),
                )
                .with_trace_config(TraceConfig::new(trace_capacity)),
        )
        .unwrap_or_else(|error| unreachable!("initial pure leaf fits: {error}"));
        let full = PumpBudget::new(usize::MAX, usize::MAX, usize::MAX, usize::MAX);
        assert!(
            app.pump(full)
                .unwrap_or_else(|_| unreachable!())
                .report()
                .is_quiescent()
        );
        app.submit_action(Action::Overrun)
            .unwrap_or_else(|_| unreachable!("declared one-output component fits"));
        app.submit_action(Action::Noop)
            .unwrap_or_else(|_| unreachable!("unrelated follower fits"));
        app.pump(full).unwrap_or_else(|_| unreachable!());
        assert_eq!(
            app.status(),
            RuntimeStatus::Terminal(
                runenui_runtime::RuntimeTerminalReason::ProducerContractViolation(
                    runenui_core::ProducerContractViolation {
                        kind: runenui_core::ProducerViolationKind::DeclarationExceeded,
                        phase: runenui_core::ProducerPhase::ApplicationUpdate,
                        component: runenui_core::BudgetComponent::ApplicationOutputs,
                        declared: 1,
                        attempted: Some(2)
                    }
                )
            )
        );
        assert_eq!(
            updates.get(),
            1,
            "only the violating producer ran; no child or follower executes"
        );
        if trace_capacity != 0 {
            assert_eq!(
                app.trace()
                    .kinds()
                    .filter(|kind| matches!(kind, TraceRecordKind::ActionSubmissionAccepted))
                    .count(),
                2,
                "only the explicit producer and follower were accepted"
            );
        }
        assert_eq!(
            app.pump(full)
                .unwrap_or_else(|_| unreachable!())
                .report()
                .processed_envelopes(),
            0
        );
    }
}

struct InitialTooWide;
#[test]
fn ordinary_pressure_returns_the_same_nonclone_action_without_acceptance_or_mutation() {
    for capacity in [16, 4096] {
        for trace in [0, 256] {
            let updates = Rc::new(Cell::new(0));
            let mut app = AppRuntime::<App>::mount_with_config(
                State {
                    updates: Rc::clone(&updates),
                },
                RuntimeConfig::default()
                    .with_queue_capacity(capacity)
                    .with_trace_config(TraceConfig::new(trace)),
            )
            .unwrap_or_else(|_| unreachable!("one leaf fits"));
            let full = PumpBudget::new(usize::MAX, usize::MAX, usize::MAX, usize::MAX);
            app.pump(full).unwrap_or_else(|_| unreachable!());
            let mut previous = 0;
            for _ in 0..capacity - 4 {
                previous = app
                    .submit_action(Action::Noop)
                    .unwrap_or_else(|_| unreachable!())
                    .get();
            }
            let payload = Box::new(475);
            let address = std::ptr::from_ref(payload.as_ref());
            let refused = app
                .submit_action(Action::Fanout(payload))
                .err()
                .unwrap_or_else(|| {
                    unreachable!("five-credit root cannot enter four free positions")
                });
            assert_eq!(refused.kind(), runenui_runtime::SubmitActionErrorKind::Full);
            let Action::Fanout(payload) = refused.into_action() else {
                unreachable!("same owned variant")
            };
            assert_eq!(std::ptr::from_ref(payload.as_ref()), address);
            assert_eq!(updates.get(), 0);
            assert_eq!(app.status(), RuntimeStatus::Running);
            assert_eq!(
                app.submit_action(Action::Noop)
                    .unwrap_or_else(|_| unreachable!())
                    .get(),
                previous + 1,
                "refused original allocated no sequence or hidden envelope"
            );
            app.pump(PumpBudget::new(0, 0, 0, 0))
                .unwrap_or_else(|_| unreachable!());
            assert_eq!(updates.get(), 0);
            app.pump(full).unwrap_or_else(|_| unreachable!());
            assert_eq!(updates.get(), capacity - 3);
            app.submit_action(Action::Fanout(payload))
                .unwrap_or_else(|_| unreachable!("explicit retry after drain"));
            app.pump(full).unwrap_or_else(|_| unreachable!());
            assert_eq!(updates.get(), capacity + 2);
            assert_eq!(app.status(), RuntimeStatus::Running);
        }
    }
}

impl UiApp for InitialTooWide {
    type State = State;
    type Action = Action;
    type HostProtocol = NoHostProtocol;
    fn initial_producer_budget(_: &State) -> ProducerBudget {
        ProducerBudget::new(5, 4)
            .with_reconciliation(1, 0, 1)
            .with_subscriptions(0, 0)
    }
    fn initial_effects(state: &State) -> Effects<Action, NoHostProtocol> {
        state.updates.set(state.updates.get() + 1);
        Effects::action(Action::Noop)
            .then(Effects::action(Action::Noop))
            .then(Effects::action(Action::Noop))
            .then(Effects::action(Action::Noop))
    }
    fn action_producer_budget(state: &State, action: &Action) -> ProducerBudget {
        App::action_producer_budget(state, action)
    }
    fn interaction_producer_budget(state: &State) -> ProducerBudget {
        App::interaction_producer_budget(state)
    }
    fn root(state: &State) -> impl View<Action> {
        App::root(state)
    }
    fn update(state: &mut State, action: Action) -> Effects<Action, NoHostProtocol> {
        App::update(state, action)
    }
}

#[test]
fn infeasible_initial_component_returns_state_without_invoking_initial_effects() {
    let updates = Rc::new(Cell::new(0));
    let config = RuntimeConfig::default()
        .with_queue_capacity(16)
        .with_limits(
            RuntimeLimits::default()
                .with_waiting_envelopes(16)
                .with_transaction_outputs(3),
        );
    let Err(refused) = AppRuntime::<InitialTooWide>::mount_with_config(
        State {
            updates: Rc::clone(&updates),
        },
        config,
    ) else {
        unreachable!("infeasible initial outputs must refuse construction");
    };
    assert_eq!(
        refused.kind(),
        runenui_runtime::InitialAdmissionErrorKind::InvalidProducerProfile
    );
    let (returned, returned_config) = refused.into_parts();
    assert!(Rc::ptr_eq(&returned.updates, &updates));
    assert_eq!(updates.get(), 0);
    assert_eq!(returned_config.queue_capacity(), 16);
    assert_eq!(returned_config.limits().transaction_outputs(), 3);
}

#[test]
fn infeasible_application_outputs_return_the_owned_action_before_mutation_or_acceptance() {
    for trace_capacity in [0, 256] {
        let updates = Rc::new(Cell::new(0));
        let mut app = AppRuntime::<App>::mount_with_config(
            State {
                updates: Rc::clone(&updates),
            },
            RuntimeConfig::default()
                .with_queue_capacity(16)
                .with_limits(
                    RuntimeLimits::default()
                        .with_waiting_envelopes(16)
                        .with_transaction_outputs(3),
                )
                .with_trace_config(TraceConfig::new(trace_capacity)),
        )
        .unwrap_or_else(|error| unreachable!("initial pure leaf fits: {error}"));
        let full = PumpBudget::new(usize::MAX, usize::MAX, usize::MAX, usize::MAX);
        assert!(
            app.pump(full)
                .unwrap_or_else(|_| unreachable!())
                .report()
                .is_quiescent()
        );
        for _ in 0..4 {
            let payload = Box::new(17);
            let identity = std::ptr::from_ref(payload.as_ref());
            let refused = app
                .submit_action(Action::Fanout(payload))
                .err()
                .unwrap_or_else(|| {
                    unreachable!(
                        "four application outputs cannot fit a three-output application transaction"
                    )
                });
            assert_eq!(
                refused.kind(),
                runenui_runtime::SubmitActionErrorKind::InvalidProducerProfile
            );
            let Action::Fanout(returned) = refused.into_action() else {
                unreachable!("original action kind")
            };
            assert_eq!(std::ptr::from_ref(returned.as_ref()), identity);
            assert_eq!(*returned, 17);
            assert_eq!(updates.get(), 0);
            assert_eq!(app.status(), RuntimeStatus::Running);
            assert!(
                !app.trace()
                    .kinds()
                    .any(|kind| matches!(kind, TraceRecordKind::ActionSubmissionAccepted))
            );
            assert_eq!(
                app.pump(full)
                    .unwrap_or_else(|_| unreachable!())
                    .report()
                    .processed_envelopes(),
                0
            );
        }
        app.submit_action(Action::Noop)
            .unwrap_or_else(|_| unreachable!("valid producer still admits"));
        assert!(
            app.pump(full)
                .unwrap_or_else(|_| unreachable!())
                .report()
                .is_quiescent()
        );
        assert_eq!(updates.get(), 1);
        assert_eq!(app.status(), RuntimeStatus::Running);
    }
}

#[test]
fn feasible_application_component_keeps_all_four_owned_outputs() {
    let updates = Rc::new(Cell::new(0));
    let mut app = AppRuntime::<App>::mount_with_config(
        State {
            updates: Rc::clone(&updates),
        },
        RuntimeConfig::default().with_limits(
            RuntimeLimits::default()
                .with_waiting_envelopes(16)
                .with_transaction_outputs(4),
        ),
    )
    .unwrap_or_else(|error| unreachable!("pure initial leaf fits: {error}"));
    let full = PumpBudget::new(usize::MAX, usize::MAX, usize::MAX, usize::MAX);
    assert!(
        app.pump(full)
            .unwrap_or_else(|_| unreachable!())
            .report()
            .is_quiescent()
    );
    app.submit_action(Action::Fanout(Box::new(17)))
        .unwrap_or_else(|_| unreachable!("the four-output component fits"));
    assert!(
        app.pump(full)
            .unwrap_or_else(|_| unreachable!())
            .report()
            .is_quiescent()
    );
    assert_eq!(
        updates.get(),
        5,
        "the producer and every bounded child execute once"
    );
    assert_eq!(app.status(), RuntimeStatus::Running);
}

#[test]
fn child_application_component_cannot_exceed_its_causal_root_declaration() {
    for qualified in [false, true] {
        let updates = Rc::new(Cell::new(0));
        let mut app = AppRuntime::<App>::mount_with_config(
            State {
                updates: Rc::clone(&updates),
            },
            RuntimeConfig::default().with_limits(
                RuntimeLimits::default()
                    .with_waiting_envelopes(16)
                    .with_transaction_outputs(4),
            ),
        )
        .unwrap_or_else(|error| unreachable!("pure initial leaf fits: {error}"));
        let full = PumpBudget::new(usize::MAX, usize::MAX, usize::MAX, usize::MAX);
        assert!(
            app.pump(full)
                .unwrap_or_else(|_| unreachable!())
                .report()
                .is_quiescent()
        );
        app.submit_action(if qualified {
            Action::EmitQualifiedChild
        } else {
            Action::EmitWideChild
        })
        .unwrap_or_else(|_| unreachable!("the parent's own component fits"));
        app.pump(full).unwrap_or_else(|_| unreachable!());
        if qualified {
            assert_eq!(app.status(), RuntimeStatus::Running);
            assert_eq!(
                updates.get(),
                6,
                "parent, wide child, and its four leaf actions commit"
            );
        } else {
            assert_eq!(
                app.status(),
                RuntimeStatus::Terminal(
                    runenui_runtime::RuntimeTerminalReason::ProducerContractViolation(
                        runenui_core::ProducerContractViolation {
                            kind: runenui_core::ProducerViolationKind::DescendantProfileNotCovered,
                            phase: runenui_core::ProducerPhase::ApplicationUpdate,
                            component: runenui_core::BudgetComponent::ApplicationOutputs,
                            declared: 1,
                            attempted: Some(4)
                        }
                    )
                )
            );
            assert_eq!(
                updates.get(),
                1,
                "the underdeclared child is never accepted or run"
            );
            assert_eq!(
                app.trace()
                    .kinds()
                    .filter(|kind| matches!(kind, TraceRecordKind::ActionSubmissionAccepted))
                    .count(),
                1
            );
        }
    }
}
