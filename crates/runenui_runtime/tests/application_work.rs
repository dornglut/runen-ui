#![allow(refining_impl_trait)]

use core::task::Poll;
use std::{cell::Cell, rc::Rc};

use runenui_core::{
    Effects, Element, IntoEffects, NoHostProtocol, SubscriptionSet, UiApp, View, Widget,
    WidgetMountContext, WidgetUpdateContext, WorkKey, children, column, text,
};
use runenui_runtime::{
    AppRuntime, PumpBudget, PumpOutcome, RuntimeConfig, RuntimeLimits, RuntimeStatus,
    RuntimeTerminalReason,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Action {
    Initial,
    FollowUp,
}

struct OrderedWorkApp;

impl UiApp for OrderedWorkApp {
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
    ) -> impl IntoEffects<Self::Action, Self::HostProtocol> {
        state.push(action);
        match action {
            Action::Initial => Effects::action(Action::FollowUp),
            Action::FollowUp => Effects::none(),
        }
    }
}

#[test]
fn initial_and_update_effects_append_in_transaction_order() {
    let mut runtime = AppRuntime::<OrderedWorkApp>::mount(Vec::new());
    let first = runtime.pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX));
    assert!(runtime.state().is_empty());
    assert_eq!(first.remaining_queued_envelopes(), 1);
    assert_eq!(first.outcome(), PumpOutcome::BudgetExhausted);

    let second = runtime.pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX));
    assert_eq!(runtime.state(), &[Action::Initial]);
    assert_eq!(second.remaining_queued_envelopes(), 1);

    let third = runtime.pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX));
    assert_eq!(runtime.state(), &[Action::Initial, Action::FollowUp]);
    assert!(third.is_quiescent());
}

struct OverflowApp;

impl UiApp for OverflowApp {
    type State = usize;
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root(_: &Self::State) -> impl View<Self::Action> {
        text("overflow")
    }

    fn update(
        state: &mut Self::State,
        (): Self::Action,
    ) -> impl IntoEffects<Self::Action, Self::HostProtocol> {
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
    let mut runtime = AppRuntime::<OverflowApp>::mount_with_config(0, config);
    runtime.pump(PumpBudget::new(
        usize::MAX,
        usize::MAX,
        usize::MAX,
        usize::MAX,
    ));
    runtime.submit_action(()).unwrap_or_else(|_| unreachable!());
    let report = runtime.pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX));

    assert_eq!(*runtime.state(), 1);
    assert_eq!(
        runtime.status(),
        RuntimeStatus::Terminal(RuntimeTerminalReason::Poisoned)
    );
    assert_eq!(
        report.outcome(),
        PumpOutcome::Terminal(RuntimeTerminalReason::Poisoned)
    );
    assert_eq!(report.remaining_queued_envelopes(), 0);
}

#[derive(Debug)]
struct SubscriptionProbe(Rc<Cell<usize>>);

impl Widget<()> for SubscriptionProbe {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn update(&self, (): &mut Self::State, context: &mut WidgetUpdateContext) {
        context.invalidate_subscriptions();
    }

    fn subscriptions(&self, (): &Self::State, _: &mut SubscriptionSet<()>) {
        self.0.set(self.0.get() + 1);
    }
}

struct MountedSubscriptionApp;

impl UiApp for MountedSubscriptionApp {
    type State = Rc<Cell<usize>>;
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> impl View<Self::Action> {
        Element::new(SubscriptionProbe(Rc::clone(state))).key("probe")
    }

    fn update(
        _: &mut Self::State,
        (): Self::Action,
    ) -> impl IntoEffects<Self::Action, Self::HostProtocol> {
    }
}

#[test]
fn mounted_declaration_runs_after_mount_and_explicit_update_invalidation() {
    let calls = Rc::new(Cell::new(0));
    let mut runtime = AppRuntime::<MountedSubscriptionApp>::mount(Rc::clone(&calls));
    assert_eq!(calls.get(), 0);
    runtime.pump(PumpBudget::new(
        usize::MAX,
        usize::MAX,
        usize::MAX,
        usize::MAX,
    ));
    assert_eq!(calls.get(), 1);

    runtime.submit_action(()).unwrap_or_else(|_| unreachable!());
    let update = runtime.pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX));
    assert_eq!(calls.get(), 1);
    assert_eq!(update.remaining_queued_envelopes(), 1);
    runtime.pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX));
    assert_eq!(calls.get(), 2);
    assert!(
        runtime
            .pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX))
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
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn mount(&self, (): &mut Self::State, context: &mut WidgetMountContext<InitialAction>) {
        context.emit(self.mounted);
    }

    fn subscriptions(&self, (): &Self::State, _: &mut SubscriptionSet<InitialAction>) {
        self.declarations.borrow_mut().push(self.owner);
    }
}

struct InitialTransactionApp;

impl UiApp for InitialTransactionApp {
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
        subscriptions.local(
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
fn initial_transaction_assigns_every_group_atomically_in_canonical_order() {
    let declarations = Rc::new(std::cell::RefCell::new(Vec::new()));
    let mut runtime =
        AppRuntime::<InitialTransactionApp>::mount((Rc::clone(&declarations), Vec::new()));

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
        runtime.pump(PumpBudget::new(1, 0, 0, 0));
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

    runtime.pump(PumpBudget::new(3, 0, 0, 0));
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
    runtime.pump(PumpBudget::new(2, 0, 0, 0));
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
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn mount(&self, (): &mut Self::State, context: &mut WidgetMountContext<()>) {
        context.emit(());
    }
}

struct AtomicFailureApp;

impl UiApp for AtomicFailureApp {
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
fn assert_initial_plan_rejected(config: RuntimeConfig, reason: RuntimeTerminalReason) {
    let mut runtime = AppRuntime::<AtomicFailureApp>::mount_with_config(0, config);
    assert_eq!(runtime.status(), RuntimeStatus::Terminal(reason));
    assert_eq!(runtime.__live_work_record_count_for_test(), 0);
    let report = runtime.pump(PumpBudget::new(
        usize::MAX,
        usize::MAX,
        usize::MAX,
        usize::MAX,
    ));
    assert_eq!(report.remaining_queued_envelopes(), 0);
    assert_eq!(*runtime.state(), 0);
}

#[cfg(feature = "internal-test-seams")]
#[test]
fn initial_transaction_rejects_capacity_sequence_generation_and_aggregate_limits_without_partial_commit()
 {
    for capacity in [0, 1, 2] {
        let limits = RuntimeLimits::default().with_waiting_envelopes(capacity);
        assert_initial_plan_rejected(
            RuntimeConfig::default().with_limits(limits),
            RuntimeTerminalReason::Poisoned,
        );
    }

    let aggregate = RuntimeLimits::default().with_transaction_outputs(3);
    assert_initial_plan_rejected(
        RuntimeConfig::default().with_limits(aggregate),
        RuntimeTerminalReason::Poisoned,
    );

    assert_initial_plan_rejected(
        RuntimeConfig::default().__with_initial_next_work_sequence_for_test(u64::MAX),
        RuntimeTerminalReason::WorkSequenceExhausted,
    );
    assert_initial_plan_rejected(
        RuntimeConfig::default().__with_initial_next_work_generation_for_test(u64::MAX),
        RuntimeTerminalReason::WorkGenerationExhausted,
    );
}
