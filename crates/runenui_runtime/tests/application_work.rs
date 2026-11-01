#![allow(refining_impl_trait)]

use core::task::Poll;
use std::{cell::Cell, rc::Rc};

use runenui_core::{
    Effects, Element, IntoEffects, NoHostProtocol, SubscriptionSet, UiApp, View, Widget,
    WidgetMountContext, WidgetUpdateContext, WorkKey, children, column, text,
};
use runenui_runtime::{
    AppRuntime, InitialAdmissionErrorKind, PumpBudget, PumpOutcome, RuntimeConfig, RuntimeLimits,
    RuntimeStatus,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Action {
    Initial,
    FollowUp,
}

struct OrderedWorkApp;

impl UiApp for OrderedWorkApp {
    fn initial_producer_budget(_state: &Self::State) -> runenui_core::ProducerBudget {
        // One initial action and the single mounted text owner are resident;
        // no work starts or subscription declarations are produced.
        runenui_core::ProducerBudget::new(2, 1)
            .with_reconciliation(1, 0, 1)
            .with_subscriptions(0, 0)
            .with_work_starts(runenui_core::WorkStartBudget::new(0, 0, 0, 0, 0))
    }

    fn action_producer_budget(
        _state: &Self::State,
        _action: &Self::Action,
    ) -> runenui_core::ProducerBudget {
        runenui_core::ProducerBudget::new(2, 1)
            .with_reconciliation(1, 0, 1)
            .with_subscriptions(0, 0)
            .with_work_starts(runenui_core::WorkStartBudget::new(0, 0, 0, 0, 0))
    }

    fn interaction_producer_budget(_state: &Self::State) -> runenui_core::ProducerBudget {
        runenui_core::ProducerBudget::new(1, 0)
    }

    type State = Vec<Action>;
    type Action = Action;
    type HostProtocol = NoHostProtocol;

    fn root(_: &Self::State) -> impl View<Self::Action> {
        text("work")
    }

    fn initial_effects(_: &Self::State) -> impl IntoEffects<Self::Action, Self::HostProtocol> {
        Effects::action(Action::Initial)
    }

    fn update(
        state: &mut Self::State,
        action: Self::Action,
    ) -> impl runenui_core::IntoUpdateOutput<Self::Action, Self::HostProtocol> {
        state.push(action);
        match action {
            Action::Initial => Effects::action(Action::FollowUp),
            Action::FollowUp => Effects::none(),
        }
    }
}

#[test]
#[allow(clippy::assert_is_empty)]
fn initial_and_update_effects_append_in_transaction_order() {
    let mut runtime = AppRuntime::<OrderedWorkApp>::mount(Vec::new())
        .unwrap_or_else(|error| unreachable!("unexpected initial admission refusal: {error}"));
    let first = runtime
        .pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();
    assert!(runtime.state().is_empty());
    assert_eq!(first.remaining_queued_envelopes(), 1);
    assert_eq!(first.outcome(), PumpOutcome::BudgetExhausted);

    let second = runtime
        .pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();
    assert_eq!(runtime.state(), &[Action::Initial]);
    assert_eq!(second.remaining_queued_envelopes(), 1);

    let third = runtime
        .pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();
    assert_eq!(runtime.state(), &[Action::Initial, Action::FollowUp]);
    assert!(third.is_quiescent());
}

struct OverflowApp;

impl UiApp for OverflowApp {
    fn initial_producer_budget(_state: &Self::State) -> runenui_core::ProducerBudget {
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
            .with_subscriptions(0, 0)
            .with_work_starts(runenui_core::WorkStartBudget::new(1, 0, 0, 0, 0))
    }

    fn interaction_producer_budget(_state: &Self::State) -> runenui_core::ProducerBudget {
        runenui_core::ProducerBudget::new(1, 0)
    }

    type State = usize;
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root(_: &Self::State) -> impl View<Self::Action> {
        text("overflow")
    }

    fn update(
        state: &mut Self::State,
        (): Self::Action,
    ) -> impl runenui_core::IntoUpdateOutput<Self::Action, Self::HostProtocol> {
        *state += 1;
        Effects::keyed_local_task(
            WorkKey::new("overflow").unwrap_or_else(|_| unreachable!()),
            async { None },
        )
    }
}

#[test]
fn post_mutation_transaction_overflow_poisoning_is_explicit() {
    let limits = RuntimeLimits::default().with_transaction_outputs(0);
    let config = RuntimeConfig::default().with_limits(limits);
    let mut runtime = AppRuntime::<OverflowApp>::mount_with_config(0, config)
        .unwrap_or_else(|error| unreachable!("unexpected initial admission refusal: {error}"));
    let _ = runtime
        .pump(PumpBudget::new(
            usize::MAX,
            usize::MAX,
            usize::MAX,
            usize::MAX,
        ))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();
    runtime.submit_action(()).unwrap_or_else(|_| unreachable!());
    let report = runtime
        .pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();

    assert_eq!(*runtime.state(), 1);
    assert_eq!(
        runtime.status(),
        RuntimeStatus::Terminal(
            runenui_runtime::RuntimeTerminalReason::ProducerContractViolation(
                runenui_core::ProducerContractViolation {
                    kind: runenui_core::ProducerViolationKind::DeclarationExceeded,
                    phase: runenui_core::ProducerPhase::ApplicationUpdate,
                    component: runenui_core::BudgetComponent::ApplicationOutputs,
                    declared: 0,
                    attempted: Some(1)
                }
            )
        )
    );
    assert_eq!(
        report.outcome(),
        PumpOutcome::Terminal(
            runenui_runtime::RuntimeTerminalReason::ProducerContractViolation(
                runenui_core::ProducerContractViolation {
                    kind: runenui_core::ProducerViolationKind::DeclarationExceeded,
                    phase: runenui_core::ProducerPhase::ApplicationUpdate,
                    component: runenui_core::BudgetComponent::ApplicationOutputs,
                    declared: 0,
                    attempted: Some(1)
                }
            )
        )
    );
    assert_eq!(report.remaining_queued_envelopes(), 0);
}

#[derive(Debug)]
struct SubscriptionProbe {
    calls: Rc<Cell<usize>>,
    revision: usize,
}

impl Widget<()> for SubscriptionProbe {
    fn producer_budget(
        &self,
        phase: runenui_core::WidgetProducerPhase,
        _state: &Self::State,
    ) -> runenui_core::WidgetProducerBudget {
        match phase {
            runenui_core::WidgetProducerPhase::Activation => {
                runenui_core::WidgetProducerBudget::new(0)
            }
            runenui_core::WidgetProducerPhase::Subscriptions => {
                runenui_core::WidgetProducerBudget::new(0).with_subscriptions(0)
            }
            runenui_core::WidgetProducerPhase::Update => runenui_core::WidgetProducerBudget::new(1),
            _ => runenui_core::WidgetProducerBudget::new(0),
        }
    }

    type State = usize;

    fn create_state(&self) -> Self::State {
        self.revision
    }

    fn update(&self, revision: &mut Self::State, context: &mut WidgetUpdateContext) {
        if *revision != self.revision {
            *revision = self.revision;
            let _ = context.try_invalidate_subscriptions();
        }
    }

    fn subscriptions(&self, _: &Self::State, _: &mut SubscriptionSet<()>) {
        self.calls.set(self.calls.get() + 1);
    }
}

struct MountedSubscriptionState {
    calls: Rc<Cell<usize>>,
    revision: usize,
}

struct MountedSubscriptionApp;

impl UiApp for MountedSubscriptionApp {
    fn initial_producer_budget(_state: &Self::State) -> runenui_core::ProducerBudget {
        runenui_core::ProducerBudget::new(1, 0)
            .with_reconciliation(1, 0, 1)
            .with_subscriptions(0, 0)
            .with_work_starts(runenui_core::WorkStartBudget::new(0, 0, 0, 0, 0))
    }

    fn action_producer_budget(
        _state: &Self::State,
        _action: &Self::Action,
    ) -> runenui_core::ProducerBudget {
        runenui_core::ProducerBudget::new(2, 1)
            .with_reconciliation(1, 1, 1)
            .with_subscriptions(0, 0)
            .with_work_starts(runenui_core::WorkStartBudget::new(0, 0, 0, 0, 0))
    }

    fn interaction_producer_budget(_state: &Self::State) -> runenui_core::ProducerBudget {
        runenui_core::ProducerBudget::new(2, 1).with_reconciliation(1, 1, 1)
    }

    type State = MountedSubscriptionState;
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> impl View<Self::Action> {
        Element::new(SubscriptionProbe {
            calls: Rc::clone(&state.calls),
            revision: state.revision,
        })
        .key("probe")
    }

    fn update(
        state: &mut Self::State,
        (): Self::Action,
    ) -> impl runenui_core::IntoUpdateOutput<Self::Action, Self::HostProtocol> {
        state.revision += 1;
    }
}

#[test]
fn mounted_declaration_runs_after_mount_and_explicit_update_invalidation() {
    let calls = Rc::new(Cell::new(0));
    let mut runtime = AppRuntime::<MountedSubscriptionApp>::mount(MountedSubscriptionState {
        calls: Rc::clone(&calls),
        revision: 0,
    })
    .unwrap_or_else(|error| unreachable!("unexpected initial admission refusal: {error}"));
    assert_eq!(calls.get(), 0);
    let _ = runtime
        .pump(PumpBudget::new(
            usize::MAX,
            usize::MAX,
            usize::MAX,
            usize::MAX,
        ))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();
    assert_eq!(calls.get(), 1);

    runtime.submit_action(()).unwrap_or_else(|_| unreachable!());
    let update = runtime
        .pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();
    assert_eq!(calls.get(), 1);
    assert_eq!(
        runtime.status(),
        RuntimeStatus::Running,
        "trace: {:?}",
        runtime
            .trace()
            .records()
            .map(runenui_runtime::TraceRecord::kind)
            .collect::<Vec<_>>()
    );
    assert_eq!(update.remaining_queued_envelopes(), 1);
    let _ = runtime
        .pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();
    assert_eq!(calls.get(), 2);
    assert!(
        runtime
            .pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX))
            .unwrap_or_else(|_| unreachable!("pump observation"))
            .report()
            .to_owned()
            .is_quiescent()
    );
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum InitialAction {
    EffectOne,
    EffectTwo,
    MountedOne,
    MountedTwo,
}

#[derive(Debug)]
struct InitialWidget {
    owner: usize,
    declarations: Rc<std::cell::RefCell<Vec<usize>>>,
    mounted: InitialAction,
}

impl Widget<InitialAction> for InitialWidget {
    fn producer_budget(
        &self,
        phase: runenui_core::WidgetProducerPhase,
        _state: &Self::State,
    ) -> runenui_core::WidgetProducerBudget {
        match phase {
            runenui_core::WidgetProducerPhase::Activation => {
                runenui_core::WidgetProducerBudget::new(0)
            }
            runenui_core::WidgetProducerPhase::Mount => runenui_core::WidgetProducerBudget::new(1),
            runenui_core::WidgetProducerPhase::Subscriptions => {
                runenui_core::WidgetProducerBudget::new(0).with_subscriptions(1)
            }
            _ => runenui_core::WidgetProducerBudget::new(0),
        }
    }

    type State = ();

    fn create_state(&self) -> Self::State {}

    fn mount(&self, (): &mut Self::State, context: &mut WidgetMountContext<InitialAction>) {
        let _ = context.try_emit(self.mounted);
    }

    fn subscriptions(&self, (): &Self::State, _: &mut SubscriptionSet<InitialAction>) {
        self.declarations.borrow_mut().push(self.owner);
    }
}

struct InitialTransactionApp;

impl UiApp for InitialTransactionApp {
    fn initial_producer_budget(_state: &Self::State) -> runenui_core::ProducerBudget {
        // Four initial actions, one application subscription start, its
        // outcome/cancellation pair, and three mounted reconciliation owners.
        runenui_core::ProducerBudget::new(10, 4)
            .with_reconciliation(3, 0, 3)
            .with_subscriptions(1, 1)
            .with_work_starts(runenui_core::WorkStartBudget::new(1, 0, 0, 0, 0))
            // Only the application subscription retains work; mounted
            // subscription callbacks emit no declarations.
            .with_work_residency(runenui_core::WorkResidencyBudget::new(0, 0, 0, 1, 0, 0))
    }

    fn action_producer_budget(
        _state: &Self::State,
        _action: &Self::Action,
    ) -> runenui_core::ProducerBudget {
        runenui_core::ProducerBudget::new(1, 0)
            .with_subscriptions(1, 0)
            .with_work_starts(runenui_core::WorkStartBudget::new(0, 0, 0, 0, 0))
    }

    fn interaction_producer_budget(_state: &Self::State) -> runenui_core::ProducerBudget {
        runenui_core::ProducerBudget::new(1, 0)
    }

    type State = (Rc<std::cell::RefCell<Vec<usize>>>, Vec<InitialAction>);
    type Action = InitialAction;
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> impl View<Self::Action> {
        column(children![
            Element::new(InitialWidget {
                owner: 1,
                declarations: Rc::clone(&state.0),
                mounted: InitialAction::MountedOne,
            })
            .key("initial.one"),
            Element::new(InitialWidget {
                owner: 2,
                declarations: Rc::clone(&state.0),
                mounted: InitialAction::MountedTwo,
            })
            .key("initial.two"),
        ])
    }

    fn initial_effects(_: &Self::State) -> impl IntoEffects<Self::Action, Self::HostProtocol> {
        Effects::action(InitialAction::EffectOne).then(Effects::action(InitialAction::EffectTwo))
    }

    fn subscriptions(_: &Self::State, subscriptions: &mut SubscriptionSet<Self::Action>) {
        let _ = subscriptions.try_local(
            WorkKey::new("initial.application").unwrap_or_else(|_| unreachable!()),
            0,
            |_: &mut core::task::Context<'_>| Poll::Pending,
        );
    }

    fn update(state: &mut Self::State, action: Self::Action) {
        state.1.push(action);
    }
}

#[test]
#[allow(clippy::assert_is_empty)]
fn initial_transaction_assigns_every_group_atomically_in_canonical_order() {
    let declarations = Rc::new(std::cell::RefCell::new(Vec::new()));
    let mut runtime =
        AppRuntime::<InitialTransactionApp>::mount((Rc::clone(&declarations), Vec::new()))
            .unwrap_or_else(|error| unreachable!("unexpected initial admission refusal: {error}"));

    let accepted_sequences: Vec<_> = runtime
        .trace()
        .records()
        .filter(|record| {
            matches!(
                record.kind(),
                runenui_runtime::TraceRecordKind::ActionSubmissionAccepted
            )
        })
        .filter_map(|record| {
            record
                .work_sequence()
                .map(runenui_runtime::WorkSequence::get)
        })
        .collect();
    assert_eq!(accepted_sequences, [4, 5, 7, 8]);

    for expected_sequence in 1..=3 {
        let _ = runtime
            .pump(PumpBudget::new(1, 0, 0, 0))
            .unwrap_or_else(|_| unreachable!("pump observation"))
            .report()
            .to_owned();
        let sequence = runtime
            .trace()
            .records()
            .filter(|record| {
                matches!(
                    record.kind(),
                    runenui_runtime::TraceRecordKind::SubscriptionDiffCommitted { .. }
                ) && record.work_sequence().is_some()
            })
            .last()
            .and_then(runenui_runtime::TraceRecord::work_sequence)
            .map_or_else(
                || unreachable!("mounted declaration retains its envelope sequence"),
                runenui_runtime::WorkSequence::get,
            );
        assert_eq!(sequence, expected_sequence);
    }
    assert_eq!(&*declarations.borrow(), &[1, 2]);
    assert!(runtime.state().1.is_empty());

    let _ = runtime
        .pump(PumpBudget::new(3, 0, 0, 0))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();
    let application_subscription_start = runtime
        .trace()
        .records()
        .find(|record| {
            matches!(
                record.kind(),
                runenui_runtime::TraceRecordKind::WorkStartAttempted
            ) && record.work().is_some_and(|identity| {
                identity.owner() == &runenui_runtime::TraceWorkOwner::Application
                    && identity.family() == runenui_runtime::TraceWorkFamily::Subscription
            })
        })
        .and_then(runenui_runtime::TraceRecord::work_sequence)
        .map(runenui_runtime::WorkSequence::get);
    assert_eq!(application_subscription_start, Some(6));
    let _ = runtime
        .pump(PumpBudget::new(2, 0, 0, 0))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();
    assert_eq!(
        runtime.state().1,
        [
            InitialAction::EffectOne,
            InitialAction::EffectTwo,
            InitialAction::MountedOne,
            InitialAction::MountedTwo,
        ]
    );
}

#[derive(Debug)]
struct EmittingMount;

impl Widget<()> for EmittingMount {
    fn producer_budget(
        &self,
        phase: runenui_core::WidgetProducerPhase,
        _state: &Self::State,
    ) -> runenui_core::WidgetProducerBudget {
        match phase {
            runenui_core::WidgetProducerPhase::Activation => {
                runenui_core::WidgetProducerBudget::new(0)
            }
            runenui_core::WidgetProducerPhase::Mount => runenui_core::WidgetProducerBudget::new(1),
            runenui_core::WidgetProducerPhase::Subscriptions => {
                runenui_core::WidgetProducerBudget::new(0).with_subscriptions(0)
            }
            _ => runenui_core::WidgetProducerBudget::new(0),
        }
    }

    type State = ();

    fn create_state(&self) -> Self::State {}

    fn mount(&self, (): &mut Self::State, context: &mut WidgetMountContext<()>) {
        let _ = context.try_emit(());
    }
}

struct AtomicFailureApp;

impl UiApp for AtomicFailureApp {
    fn initial_producer_budget(_state: &Self::State) -> runenui_core::ProducerBudget {
        // Two mount actions plus two local starts (each with an outcome and
        // cancellation credit) share the three mounted-owner obligations.
        runenui_core::ProducerBudget::new(11, 4)
            .with_reconciliation(3, 0, 3)
            .with_subscriptions(0, 0)
            .with_work_starts(runenui_core::WorkStartBudget::new(2, 0, 0, 0, 0))
            .with_work_residency(runenui_core::WorkResidencyBudget::new(2, 0, 0, 0, 0, 0))
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
        column(children![
            Element::new(EmittingMount).key("failure.one"),
            Element::new(EmittingMount).key("failure.two"),
        ])
    }

    fn initial_effects(_: &Self::State) -> impl IntoEffects<Self::Action, Self::HostProtocol> {
        Effects::local_task(async { None }).then(Effects::local_task(async { None }))
    }

    fn update(state: &mut Self::State, (): Self::Action) {
        *state += 1;
    }
}

#[cfg(feature = "internal-test-seams")]
#[test]
fn initial_transaction_rejects_capacity_sequence_generation_and_aggregate_limits_without_partial_commit()
 {
    for capacity in [0, 1, 2] {
        let limits = RuntimeLimits::default().with_waiting_envelopes(capacity);
        let config = RuntimeConfig::default().with_limits(limits);
        let Err(error) = AppRuntime::<AtomicFailureApp>::mount_with_config(0, config) else {
            unreachable!("the source-derived initial frontier exceeds these capacities")
        };
        assert_eq!(error.kind(), InitialAdmissionErrorKind::Capacity);
        assert_eq!(error.state(), &0);
    }

    let aggregate = RuntimeLimits::default().with_transaction_outputs(3);
    let Err(error) = AppRuntime::<AtomicFailureApp>::mount_with_config(
        0,
        RuntimeConfig::default().with_limits(aggregate),
    ) else {
        unreachable!("the four-output initial application component cannot fit three");
    };
    assert_eq!(
        error.kind(),
        InitialAdmissionErrorKind::InvalidProducerProfile
    );
    assert_eq!(error.state(), &0);
    assert_eq!(error.config().limits().transaction_outputs(), 3);

    let sequence_config =
        RuntimeConfig::default().__with_initial_next_work_sequence_for_test(u64::MAX);
    let Err(error) = AppRuntime::<AtomicFailureApp>::mount_with_config(0, sequence_config) else {
        unreachable!("the initial frontier cannot reserve a sequence at u64::MAX")
    };
    assert_eq!(error.kind(), InitialAdmissionErrorKind::SequenceExhausted);
    assert_eq!(error.state(), &0);
    let config = RuntimeConfig::default().__with_initial_next_work_generation_for_test(u64::MAX);
    let Err(error) = AppRuntime::<AtomicFailureApp>::mount_with_config(0, config) else {
        unreachable!("initial generation exhaustion is refused before construction")
    };
    assert_eq!(
        error.kind(),
        InitialAdmissionErrorKind::WorkGenerationExhausted
    );
    assert_eq!(error.into_parts(), (0, config));
}

struct InitialAdmissionProbeState(Rc<Cell<usize>>);

struct InitialAdmissionProbeApp;

impl UiApp for InitialAdmissionProbeApp {
    fn initial_producer_budget(_state: &Self::State) -> runenui_core::ProducerBudget {
        // One text node has one mounted reconciliation owner and no output or
        // work-start obligations: queue credit one is the exact initial bound.
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
    }

    fn interaction_producer_budget(_state: &Self::State) -> runenui_core::ProducerBudget {
        runenui_core::ProducerBudget::new(1, 0)
    }

    type State = InitialAdmissionProbeState;
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> impl View<Self::Action> {
        state.0.set(state.0.get() + 1);
        text("admission probe")
    }

    fn update(_: &mut Self::State, (): Self::Action) {}
}

#[test]
fn exact_minimum_initial_profile_admits_and_capacity_refusal_precedes_root_factory() {
    let calls = Rc::new(Cell::new(0));
    let Err(error) = AppRuntime::<InitialAdmissionProbeApp>::mount_with_config(
        InitialAdmissionProbeState(Rc::clone(&calls)),
        RuntimeConfig::default().with_queue_capacity(0),
    ) else {
        unreachable!("zero capacity cannot admit the one-credit initial frontier")
    };
    assert_eq!(error.kind(), InitialAdmissionErrorKind::Capacity);
    assert_eq!(calls.get(), 0);
    let (state, _config) = error.into_parts();
    assert!(Rc::ptr_eq(&state.0, &calls));

    let runtime = AppRuntime::<InitialAdmissionProbeApp>::mount(InitialAdmissionProbeState(
        Rc::clone(&calls),
    ))
    .unwrap_or_else(|error| unreachable!("the exact one-credit initial profile admits: {error}"));
    assert_eq!(calls.get(), 1);
    assert_eq!(runtime.status(), RuntimeStatus::Running);
}
