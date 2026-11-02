#![allow(refining_impl_trait)]

use core::{
    pin::Pin,
    task::{Context, Poll},
};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::{cell::Cell, rc::Rc};

use runenui_core::{
    Element, NoHostProtocol, UiApp, View, Widget, WidgetMountContext, WidgetUpdateContext, text,
};
use runenui_runtime::{
    AppRuntime, InitialAdmissionErrorKind, PumpBudget, PumpOutcome, RuntimeConfig, RuntimeStatus,
    RuntimeTerminalReason, SubmitActionError, SubmitActionErrorKind,
};

#[derive(Debug, Eq, PartialEq)]
struct Action(u32);

struct OrderedApp;

impl UiApp for OrderedApp {
    fn initial_producer_budget(_state: &Self::State) -> runenui_core::ProducerBudget {
        // One mounted root owner emits one mandatory initial subscription-reconcile envelope.
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
            .with_work_starts(runenui_core::WorkStartBudget::new(0, 0, 0, 0, 0))
    }

    fn interaction_producer_budget(_state: &Self::State) -> runenui_core::ProducerBudget {
        runenui_core::ProducerBudget::new(1, 0)
    }

    type State = Vec<u32>;
    type Action = Action;
    type HostProtocol = NoHostProtocol;

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
    let mut runtime = AppRuntime::<OrderedApp>::mount_with_config(Vec::new(), config)
        .unwrap_or_else(|error| unreachable!("unexpected initial admission refusal: {error}"));
    let _ = runtime
        .pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();
    let first = runtime
        .submit_action(Action(10))
        .unwrap_or_else(|_| unreachable!());
    let second = runtime
        .submit_action(Action(20))
        .unwrap_or_else(|_| unreachable!());
    assert_eq!(first.get(), 2);
    assert_eq!(second.get(), 3);
    let Err(rejected) = runtime.submit_action(Action(30)) else {
        unreachable!()
    };
    assert_eq!(rejected.kind(), SubmitActionErrorKind::Full);
    assert_eq!(rejected.into_action(), Action(30));
    assert_eq!(runtime.state(), &[] as &[u32]);

    let first_pump = runtime
        .pump(PumpBudget::new(2, usize::MAX, usize::MAX, usize::MAX))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();
    assert_eq!(first_pump.processed_envelopes(), 2);
    assert_eq!(first_pump.remaining_queued_envelopes(), 0);
    assert_eq!(first_pump.outcome(), PumpOutcome::Quiescent);
    assert_eq!(runtime.state(), &[10, 20]);
    assert_eq!(runtime.reconciliation_report().generation().get(), 3);

    let third = runtime
        .submit_action(rejected_action())
        .unwrap_or_else(|_| unreachable!());
    assert_eq!(third.get(), 4, "full rejection consumed no sequence");
    let final_pump = runtime
        .pump(PumpBudget::new(8, usize::MAX, usize::MAX, usize::MAX))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();
    assert_eq!(final_pump.processed_envelopes(), 1);
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
    let mut runtime = AppRuntime::<OrderedApp>::mount(Vec::new())
        .unwrap_or_else(|error| unreachable!("unexpected initial admission refusal: {error}"));
    let _ = runtime
        .pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();
    for value in 0..5 {
        runtime
            .submit_action(Action(value))
            .unwrap_or_else(|_| unreachable!());
    }
    let zero = runtime
        .pump(PumpBudget::new(0, usize::MAX, usize::MAX, usize::MAX))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();
    assert_eq!(zero.processed_envelopes(), 0);
    assert_eq!(zero.remaining_queued_envelopes(), 5);
    assert_eq!(zero.outcome(), PumpOutcome::BudgetExhausted);
    let two = runtime
        .pump(PumpBudget::new(2, usize::MAX, usize::MAX, usize::MAX))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();
    assert_eq!(two.processed_envelopes(), 2);
    assert_eq!(runtime.state(), &[0, 1]);
    let rest = runtime
        .pump(PumpBudget::new(
            usize::MAX,
            usize::MAX,
            usize::MAX,
            usize::MAX,
        ))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();
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
            _ => runenui_core::WidgetProducerBudget::new(0),
        }
    }

    type State = ();

    fn create_state(&self) -> Self::State {}

    fn mount(&self, (): &mut Self::State, _: &mut WidgetMountContext<TransactionAction>) {
        self.mounted_value.set(self.value);
    }

    fn update(&self, (): &mut Self::State, _: &mut WidgetUpdateContext<TransactionAction>) {
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
            .with_work_starts(runenui_core::WorkStartBudget::new(0, 0, 0, 0, 0))
    }

    fn interaction_producer_budget(_state: &Self::State) -> runenui_core::ProducerBudget {
        runenui_core::ProducerBudget::new(1, 0)
    }

    type State = TransactionState;
    type Action = TransactionAction;
    type HostProtocol = NoHostProtocol;

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
    let mut runtime = AppRuntime::<TransactionApp>::mount(state)
        .unwrap_or_else(|error| unreachable!("unexpected initial admission refusal: {error}"));
    let _ = runtime
        .pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();
    runtime
        .submit_action(TransactionAction(1))
        .unwrap_or_else(|_| unreachable!());
    runtime
        .submit_action(TransactionAction(2))
        .unwrap_or_else(|_| unreachable!());

    assert_eq!(
        runtime
            .pump(PumpBudget::new(2, usize::MAX, usize::MAX, usize::MAX))
            .unwrap_or_else(|_| unreachable!("pump observation"))
            .report()
            .to_owned()
            .processed_envelopes(),
        2
    );
    assert_eq!(runtime.state().mounted_values_seen_by_update, [0, 1]);
    assert_eq!(mounted_value.get(), 2);
    assert_eq!(runtime.reconciliation_report().generation().get(), 3);
}

#[test]
fn capacity_zero_returns_state_and_config_before_widget_mutation() {
    let config = RuntimeConfig::default().with_queue_capacity(0);
    let mounted_value = Rc::new(Cell::new(u32::MAX));
    let state = TransactionState {
        value: 0,
        mounted_value: Rc::clone(&mounted_value),
        mounted_values_seen_by_update: Vec::new(),
    };
    let Err(error) = AppRuntime::<TransactionApp>::mount_with_config(state, config) else {
        unreachable!()
    };
    assert_eq!(error.kind(), InitialAdmissionErrorKind::Capacity);
    let (state, config) = error.into_parts();
    assert_eq!(state.value, 0);
    assert_eq!(state.mounted_values_seen_by_update, Vec::<u32>::new());
    assert_eq!(mounted_value.get(), u32::MAX);
    assert_eq!(config.queue_capacity(), 0);
}

static INVALID_PROFILE_ROOT_CALLS: AtomicUsize = AtomicUsize::new(0);

struct InvalidInitialProfileApp;

impl UiApp for InvalidInitialProfileApp {
    fn initial_producer_budget(_state: &Self::State) -> runenui_core::ProducerBudget {
        runenui_core::ProducerBudget::new(0, 0).with_reconciliation(1, 0, 1)
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

    type State = usize;
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root(_state: &Self::State) -> Element<Self::Action> {
        INVALID_PROFILE_ROOT_CALLS.fetch_add(1, Ordering::SeqCst);
        text("invalid initial profile").into_element()
    }

    fn update(_state: &mut Self::State, (): Self::Action) {}
}

#[test]
fn invalid_initial_profile_is_refused_before_root_callback() {
    INVALID_PROFILE_ROOT_CALLS.store(0, Ordering::SeqCst);
    let Err(error) =
        AppRuntime::<InvalidInitialProfileApp>::mount_with_config(41, RuntimeConfig::default())
    else {
        unreachable!()
    };
    assert_eq!(
        error.kind(),
        InitialAdmissionErrorKind::InvalidProducerProfile
    );
    let (state, config) = error.into_parts();
    assert_eq!(state, 41);
    assert_eq!(
        config.queue_capacity(),
        RuntimeConfig::default().queue_capacity()
    );
    assert_eq!(INVALID_PROFILE_ROOT_CALLS.load(Ordering::SeqCst), 0);
}

struct SequenceAdmissionApp;

impl UiApp for SequenceAdmissionApp {
    fn initial_producer_budget(_state: &Self::State) -> runenui_core::ProducerBudget {
        runenui_core::ProducerBudget::new(2, 0).with_reconciliation(1, 0, 1)
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

    type State = usize;
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root(_state: &Self::State) -> Element<Self::Action> {
        text("sequence exhaustion").into_element()
    }

    fn update(_state: &mut Self::State, (): Self::Action) {}
}

#[test]
fn initial_sequence_exhaustion_returns_unconsumed_state_and_config() {
    let config = RuntimeConfig::default().__with_initial_next_work_sequence_for_test(u64::MAX);
    let Err(error) = AppRuntime::<SequenceAdmissionApp>::mount_with_config(73, config) else {
        unreachable!()
    };
    assert_eq!(error.kind(), InitialAdmissionErrorKind::SequenceExhausted);
    let (state, returned_config) = error.into_parts();
    assert_eq!(state, 73);
    assert_eq!(returned_config, config);
}

#[test]
fn logical_queue_capacity_does_not_eagerly_reserve() {
    let config = RuntimeConfig::default().with_queue_capacity(usize::MAX);
    let mut runtime = AppRuntime::<OrderedApp>::mount_with_config(Vec::new(), config)
        .unwrap_or_else(|error| unreachable!("unexpected initial admission refusal: {error}"));
    let _ = runtime
        .pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();

    runtime
        .submit_action(Action(1))
        .unwrap_or_else(|_| unreachable!());
    let report = runtime
        .pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();

    assert_eq!(report.processed_envelopes(), 1);
    assert_eq!(report.remaining_queued_envelopes(), 0);
    assert_eq!(report.outcome(), PumpOutcome::Quiescent);
    assert_eq!(runtime.state(), &[1]);
}

#[test]
fn a_large_queue_is_processed_iteratively() {
    const COUNT: usize = 10_000;
    let config = RuntimeConfig::default().with_queue_capacity(COUNT);
    let mut runtime = AppRuntime::<OrderedApp>::mount_with_config(Vec::new(), config)
        .unwrap_or_else(|error| unreachable!("unexpected initial admission refusal: {error}"));
    let _ = runtime
        .pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();
    for value in 0..COUNT {
        runtime
            .submit_action(Action(
                u32::try_from(value).unwrap_or_else(|_| unreachable!()),
            ))
            .unwrap_or_else(|_| unreachable!());
    }
    let report = runtime
        .pump(PumpBudget::new(COUNT, usize::MAX, usize::MAX, usize::MAX))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();
    assert_eq!(report.processed_envelopes(), COUNT);
    assert_eq!(report.outcome(), PumpOutcome::Quiescent);
    assert_eq!(runtime.state().len(), COUNT);
    assert_eq!(runtime.state()[0], 0);
    assert_eq!(
        runtime.state()[COUNT - 1],
        u32::try_from(COUNT - 1).unwrap_or_else(|_| unreachable!())
    );
}

#[derive(Clone, Copy)]
struct SubscriptionAction;

struct PendingMountedSubscription;

impl runenui_core::LocalSubscriptionSource<SubscriptionAction> for PendingMountedSubscription {
    fn poll_next(
        self: Pin<&mut Self>,
        _context: &mut Context<'_>,
    ) -> Poll<Option<SubscriptionAction>> {
        Poll::Pending
    }
}

#[derive(Debug)]
struct MountedSubscriptionOwner {
    active: bool,
}

impl Widget<SubscriptionAction> for MountedSubscriptionOwner {
    type State = ();

    fn producer_budget(
        &self,
        phase: runenui_core::WidgetProducerPhase,
        _state: &Self::State,
    ) -> runenui_core::WidgetProducerBudget {
        match phase {
            runenui_core::WidgetProducerPhase::Subscriptions => {
                runenui_core::WidgetProducerBudget::new(0)
                    .with_subscriptions(1)
                    .with_work_starts(runenui_core::WorkStartBudget::new(1, 0, 0, 0, 0))
            }
            runenui_core::WidgetProducerPhase::Update => runenui_core::WidgetProducerBudget::new(1),
            _ => runenui_core::WidgetProducerBudget::new(0),
        }
    }

    fn create_state(&self) -> Self::State {}

    fn update(&self, (): &mut Self::State, context: &mut WidgetUpdateContext<SubscriptionAction>) {
        let _ = context.try_invalidate_subscriptions();
    }

    fn subscriptions(
        &self,
        (): &Self::State,
        subscriptions: &mut runenui_core::SubscriptionSet<SubscriptionAction>,
    ) {
        if self.active {
            let _ = subscriptions.try_local(
                runenui_core::WorkKey::new("mounted.live").unwrap_or_else(|_| unreachable!()),
                0,
                PendingMountedSubscription,
            );
        }
    }
}

struct MountedSubscriptionApp;

impl UiApp for MountedSubscriptionApp {
    type State = bool;
    type Action = SubscriptionAction;
    type HostProtocol = NoHostProtocol;

    fn initial_producer_budget(_state: &Self::State) -> runenui_core::ProducerBudget {
        runenui_core::ProducerBudget::new(1, 0)
            .with_reconciliation(1, 0, 1)
            .with_subscriptions(0, 1)
            .with_work_starts(runenui_core::WorkStartBudget::new(1, 0, 0, 0, 0))
            .with_work_residency(runenui_core::WorkResidencyBudget::new(0, 0, 0, 1, 0, 0))
    }

    fn action_producer_budget(
        _state: &Self::State,
        _action: &Self::Action,
    ) -> runenui_core::ProducerBudget {
        runenui_core::ProducerBudget::new(2, 0)
            .with_reconciliation(0, 1, 1)
            .with_subscriptions(0, 1)
            .with_work_starts(runenui_core::WorkStartBudget::new(0, 0, 0, 0, 0))
    }

    fn interaction_producer_budget(_state: &Self::State) -> runenui_core::ProducerBudget {
        runenui_core::ProducerBudget::new(1, 0)
    }

    fn root(state: &Self::State) -> Element<Self::Action> {
        Element::new(MountedSubscriptionOwner { active: *state }).key("owner")
    }

    fn update(state: &mut Self::State, _: Self::Action) {
        *state = false;
    }
}

#[test]
fn mounted_subscription_reconciliation_and_cancellation_retain_the_action_root() {
    let config = RuntimeConfig::default().with_queue_capacity(3);
    let mut runtime = AppRuntime::<MountedSubscriptionApp>::mount_with_config(true, config)
        .unwrap_or_else(|error| unreachable!("unexpected initial admission refusal: {error}"));

    runtime
        .pump(PumpBudget::new(2, 0, 0, 0))
        .unwrap_or_else(|_| unreachable!("initial mounted subscription work is funded"));
    assert_eq!(
        runtime.status(),
        RuntimeStatus::Running,
        "trace: {:#?}",
        runtime
            .trace()
            .records()
            .map(runenui_runtime::TraceRecord::kind)
            .collect::<Vec<_>>()
    );
    assert!(runtime.trace().records().any(|record| matches!(
        record.kind(),
        runenui_runtime::TraceRecordKind::WorkStartAccepted
    )));

    runtime
        .submit_action(SubscriptionAction)
        .unwrap_or_else(|_| unreachable!("two-credit action root fits beside the live lease"));
    let action = runtime
        .pump(PumpBudget::new(1, 0, 0, 0))
        .unwrap_or_else(|_| unreachable!("application transaction remains admitted"));
    assert_eq!(action.report().processed_envelopes(), 1);
    assert_eq!(
        runtime.status(),
        RuntimeStatus::Running,
        "trace: {:#?}",
        runtime
            .trace()
            .records()
            .map(runenui_runtime::TraceRecord::kind)
            .collect::<Vec<_>>()
    );

    let reconciliation = runtime
        .pump(PumpBudget::new(1, 0, 0, 0))
        .unwrap_or_else(|_| unreachable!("mounted subscription diff uses its accepted root"));
    assert_eq!(reconciliation.report().processed_envelopes(), 1);
    assert_eq!(
        runtime.status(),
        RuntimeStatus::Running,
        "trace: {:#?}",
        runtime
            .trace()
            .records()
            .map(runenui_runtime::TraceRecord::kind)
            .collect::<Vec<_>>()
    );

    let cancellation = runtime
        .pump(PumpBudget::new(1, 0, 0, 0))
        .unwrap_or_else(|_| unreachable!("subscription retirement uses the same root"));
    assert_eq!(cancellation.report().processed_envelopes(), 1);
    assert_eq!(runtime.status(), RuntimeStatus::Running);
    assert_eq!(runtime.state(), &false);
}

#[test]
fn capacity_two_refuses_the_causal_root_before_action_acceptance() {
    let config = RuntimeConfig::default().with_queue_capacity(2);
    let mut runtime = AppRuntime::<MountedSubscriptionApp>::mount_with_config(true, config)
        .unwrap_or_else(|error| unreachable!("initial subscription work is admitted: {error}"));
    runtime
        .pump(PumpBudget::new(2, 0, 0, 0))
        .unwrap_or_else(|_| unreachable!("initial mounted subscription work is funded"));
    let accepted_before = runtime
        .trace()
        .records()
        .filter(|record| {
            matches!(
                record.kind(),
                runenui_runtime::TraceRecordKind::ActionSubmissionAccepted
            )
        })
        .count();
    let Err(error) = runtime.submit_action(SubscriptionAction) else {
        unreachable!("capacity two cannot reserve a two-credit root beside the live lease")
    };
    assert_eq!(error.kind(), runenui_runtime::SubmitActionErrorKind::Full);
    assert_eq!(
        runtime
            .trace()
            .records()
            .filter(|record| matches!(
                record.kind(),
                runenui_runtime::TraceRecordKind::ActionSubmissionAccepted
            ))
            .count(),
        accepted_before
    );
    assert_eq!(runtime.state(), &true);
    assert_eq!(runtime.status(), RuntimeStatus::Running);
}

#[test]
fn queued_subscription_descendants_hold_capacity_until_exact_cancellation_retires() {
    for trace_capacity in [0, 256] {
        let mut runtime = AppRuntime::<MountedSubscriptionApp>::mount_with_config(
            true,
            RuntimeConfig::default()
                .with_queue_capacity(3)
                .with_trace_config(runenui_runtime::TraceConfig::new(trace_capacity)),
        )
        .unwrap_or_else(|error| unreachable!("initial subscription fits: {error}"));
        runtime
            .pump(PumpBudget::new(2, 0, 0, 0))
            .unwrap_or_else(|_| unreachable!());
        let original = runtime
            .submit_action(SubscriptionAction)
            .unwrap_or_else(|_| unreachable!());
        // The app action, mounted declaration and cancellation are separate
        // FIFO envelopes. Completing the app action must not release its root.
        for step in 0..3 {
            let zero = runtime
                .pump(PumpBudget::new(0, 0, 0, 0))
                .unwrap_or_else(|_| unreachable!());
            assert_eq!(zero.report().processed_envelopes(), 0);
            assert!(!zero.report().is_quiescent());
            let Err(refused) = runtime.submit_action(SubscriptionAction) else {
                unreachable!("step {step}: mandatory queued descendants still own their frontier");
            };
            assert_eq!(refused.kind(), SubmitActionErrorKind::Full);
            let progress = runtime
                .pump(PumpBudget::new(1, 0, 0, 0))
                .unwrap_or_else(|_| unreachable!());
            assert_eq!(progress.report().processed_envelopes(), 1);
            assert_eq!(runtime.status(), RuntimeStatus::Running);
        }
        let retried = runtime
            .submit_action(SubscriptionAction)
            .unwrap_or_else(|_| unreachable!("exact retirement releases both roots"));
        assert_eq!(
            retried.get(),
            original.get() + 3,
            "refusals allocate no sequence"
        );
        let done = runtime
            .pump(PumpBudget::new(8, 0, 0, 0))
            .unwrap_or_else(|_| unreachable!());
        assert!(done.report().is_quiescent());
        assert_eq!(runtime.status(), RuntimeStatus::Running);
    }
}

#[derive(Eq, PartialEq)]
struct LocalOnlyAction(Rc<()>);

struct LocalOnlyApp;

impl UiApp for LocalOnlyApp {
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
            .with_work_starts(runenui_core::WorkStartBudget::new(0, 0, 0, 0, 0))
    }

    fn interaction_producer_budget(_state: &Self::State) -> runenui_core::ProducerBudget {
        runenui_core::ProducerBudget::new(1, 0)
    }

    type State = usize;
    type Action = LocalOnlyAction;
    type HostProtocol = NoHostProtocol;

    fn root(_: &Self::State) -> Element<Self::Action> {
        text("local").into_element()
    }

    fn update(state: &mut Self::State, _: Self::Action) {
        *state += 1;
    }
}

#[test]
fn actions_need_neither_clone_send_nor_debug() {
    let mut runtime = AppRuntime::<LocalOnlyApp>::mount(0)
        .unwrap_or_else(|error| unreachable!("unexpected initial admission refusal: {error}"));
    let _ = runtime
        .pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();
    let action = LocalOnlyAction(Rc::new(()));
    assert!(runtime.submit_action(action).is_ok());
    let _ = runtime
        .pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();
    assert_eq!(runtime.state(), &1);
}

#[test]
fn shutdown_is_idempotent_cancels_waiting_actions_and_closes_submission() {
    let mut runtime = AppRuntime::<OrderedApp>::mount(Vec::new())
        .unwrap_or_else(|error| unreachable!("unexpected initial admission refusal: {error}"));
    let _ = runtime
        .pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned();
    runtime
        .submit_action(Action(1))
        .unwrap_or_else(|_| unreachable!());
    runtime
        .submit_action(Action(2))
        .unwrap_or_else(|_| unreachable!());
    let first = runtime
        .shutdown()
        .unwrap_or_else(|_| unreachable!("shutdown observation"))
        .report()
        .to_owned();
    assert!(!first.already_complete());
    assert_eq!(first.cancelled_queued_envelopes(), 2);
    assert_eq!(first.unmounted_lifetimes(), 1);
    assert_eq!(runtime.status(), RuntimeStatus::Closed);
    assert_eq!(
        runtime
            .pump(PumpBudget::new(8, usize::MAX, usize::MAX, usize::MAX))
            .unwrap_or_else(|_| unreachable!("pump observation"))
            .report()
            .to_owned()
            .outcome(),
        PumpOutcome::Closed
    );
    assert_eq!(runtime.state(), &[] as &[u32]);
    let Err(error) = runtime.submit_action(Action(3)) else {
        unreachable!()
    };
    assert_eq!(error.kind(), SubmitActionErrorKind::Closed);
    assert_eq!(error.into_action(), Action(3));
    let second = runtime
        .shutdown()
        .unwrap_or_else(|_| unreachable!("shutdown observation"))
        .report()
        .to_owned();
    assert!(second.already_complete());
    assert_eq!(second.cancelled_queued_envelopes(), 0);
    assert_eq!(second.unmounted_lifetimes(), 0);
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum CausalAction {
    Unary(u8),
    FanA,
    FanB,
    Follower,
    Grow,
    UnbudgetedStart,
}

struct CausalQueueApp;

impl UiApp for CausalQueueApp {
    fn initial_producer_budget(_state: &Self::State) -> runenui_core::ProducerBudget {
        runenui_core::ProducerBudget::new(1, 0)
            .with_reconciliation(1, 0, 1)
            .with_subscriptions(0, 0)
            .with_work_starts(runenui_core::WorkStartBudget::new(0, 0, 0, 0, 0))
    }

    fn action_producer_budget(
        _state: &Self::State,
        action: &Self::Action,
    ) -> runenui_core::ProducerBudget {
        let (credits, outputs) = match action {
            CausalAction::Unary(_) => (1, 1),
            CausalAction::FanA => (3, 2),
            CausalAction::FanB | CausalAction::Follower => (1, 0),
            CausalAction::Grow | CausalAction::UnbudgetedStart => (2, 2),
        };
        runenui_core::ProducerBudget::new(credits, outputs)
            .with_subscriptions(0, 0)
            .with_work_starts(runenui_core::WorkStartBudget::new(0, 0, 0, 0, 0))
    }

    fn interaction_producer_budget(_state: &Self::State) -> runenui_core::ProducerBudget {
        runenui_core::ProducerBudget::new(3, 2)
    }

    type State = Vec<CausalAction>;
    type Action = CausalAction;
    type HostProtocol = NoHostProtocol;

    fn root(_state: &Self::State) -> Element<Self::Action> {
        text("causal queue").into_element()
    }

    fn update(
        state: &mut Self::State,
        action: Self::Action,
    ) -> impl runenui_core::IntoUpdateOutput<Self::Action, Self::HostProtocol> {
        state.push(action);
        match action {
            CausalAction::Unary(remaining) if remaining > 0 => {
                runenui_core::Effects::action(CausalAction::Unary(remaining - 1))
            }
            CausalAction::FanA => runenui_core::Effects::action(CausalAction::FanA)
                .then(runenui_core::Effects::action(CausalAction::FanB)),
            CausalAction::Grow => runenui_core::Effects::action(CausalAction::Grow)
                .then(runenui_core::Effects::action(CausalAction::Grow)),
            CausalAction::UnbudgetedStart => runenui_core::Effects::local_task(async { None })
                .then(runenui_core::Effects::action(CausalAction::Follower)),
            CausalAction::Unary(_) | CausalAction::FanB | CausalAction::Follower => {
                runenui_core::Effects::none()
            }
        }
    }
}

#[test]
fn producer_work_start_overrun_is_rejected_before_external_start_or_child_append() {
    static START_POLLS: AtomicUsize = AtomicUsize::new(0);
    struct StartOverrunApp;

    impl UiApp for StartOverrunApp {
        fn initial_producer_budget(_state: &Self::State) -> runenui_core::ProducerBudget {
            runenui_core::ProducerBudget::new(1, 0)
                .with_reconciliation(1, 0, 1)
                .with_subscriptions(0, 0)
                .with_work_starts(runenui_core::WorkStartBudget::new(0, 0, 0, 0, 0))
        }

        fn action_producer_budget(
            _state: &Self::State,
            action: &Self::Action,
        ) -> runenui_core::ProducerBudget {
            let credits = match action {
                CausalAction::UnbudgetedStart => 2,
                _ => 1,
            };
            runenui_core::ProducerBudget::new(credits, 2)
                .with_subscriptions(0, 0)
                .with_work_starts(runenui_core::WorkStartBudget::new(0, 0, 0, 0, 0))
        }

        fn interaction_producer_budget(_state: &Self::State) -> runenui_core::ProducerBudget {
            runenui_core::ProducerBudget::new(1, 0)
        }

        type State = Vec<CausalAction>;
        type Action = CausalAction;
        type HostProtocol = NoHostProtocol;

        fn root(_state: &Self::State) -> Element<Self::Action> {
            text("start overrun").into_element()
        }

        fn update(
            state: &mut Self::State,
            action: Self::Action,
        ) -> impl runenui_core::IntoUpdateOutput<Self::Action, Self::HostProtocol> {
            state.push(action);
            match action {
                CausalAction::UnbudgetedStart => runenui_core::Effects::local_task(async {
                    START_POLLS.fetch_add(1, Ordering::SeqCst);
                    None
                })
                .then(runenui_core::Effects::action(CausalAction::Follower)),
                _ => runenui_core::Effects::none(),
            }
        }
    }

    START_POLLS.store(0, Ordering::SeqCst);

    let mut runtime = AppRuntime::<StartOverrunApp>::mount_with_config(
        Vec::new(),
        RuntimeConfig::default().with_queue_capacity(4),
    )
    .unwrap_or_else(|error| unreachable!("unexpected initial admission refusal: {error}"));
    let _ = pump_actions(&mut runtime, 1);
    runtime
        .submit_action(CausalAction::UnbudgetedStart)
        .unwrap_or_else(|_| unreachable!());
    let report = pump_actions(&mut runtime, 1);

    assert_eq!(runtime.state(), &[CausalAction::UnbudgetedStart]);
    assert_eq!(START_POLLS.load(Ordering::SeqCst), 0);
    assert_eq!(
        runtime.status(),
        RuntimeStatus::Terminal(
            runenui_runtime::RuntimeTerminalReason::ProducerContractViolation(
                runenui_core::ProducerContractViolation {
                    kind: runenui_core::ProducerViolationKind::DeclarationExceeded,
                    phase: runenui_core::ProducerPhase::ApplicationUpdate,
                    component: runenui_core::BudgetComponent::WorkStarts(
                        runenui_core::WorkFamily::LocalTask
                    ),
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
                    component: runenui_core::BudgetComponent::WorkStarts(
                        runenui_core::WorkFamily::LocalTask
                    ),
                    declared: 0,
                    attempted: Some(1)
                }
            )
        )
    );
    assert_eq!(report.remaining_queued_envelopes(), 0);
}

fn pump_actions<A: UiApp>(
    runtime: &mut AppRuntime<A>,
    budget: usize,
) -> runenui_runtime::PumpReport {
    runtime
        .pump(PumpBudget::new(budget, usize::MAX, usize::MAX, usize::MAX))
        .unwrap_or_else(|_| unreachable!("pump observation"))
        .report()
        .to_owned()
}

#[test]
fn causal_root_quota_renews_for_a_bounded_unary_chain() {
    let mut runtime = AppRuntime::<CausalQueueApp>::mount_with_config(
        Vec::new(),
        RuntimeConfig::default().with_queue_capacity(1),
    )
    .unwrap_or_else(|error| unreachable!("unexpected initial admission refusal: {error}"));
    let _ = pump_actions(&mut runtime, 1);
    runtime
        .submit_action(CausalAction::Unary(63))
        .unwrap_or_else(|_| unreachable!());

    for _ in 0..63 {
        let report = pump_actions(&mut runtime, 1);
        assert_eq!(report.processed_envelopes(), 1);
        assert_eq!(report.remaining_queued_envelopes(), 1);
        assert_eq!(report.outcome(), PumpOutcome::BudgetExhausted);
    }
    let final_report = pump_actions(&mut runtime, 1);
    assert_eq!(final_report.processed_envelopes(), 1);
    assert_eq!(final_report.remaining_queued_envelopes(), 0);
    assert_eq!(final_report.outcome(), PumpOutcome::Quiescent);
    assert_eq!(runtime.state().len(), 64);
    assert!(matches!(runtime.status(), RuntimeStatus::Running));
    runtime
        .submit_action(CausalAction::Unary(0))
        .unwrap_or_else(|_| unreachable!("completed causal root released its queue credit"));
    assert_eq!(
        pump_actions(&mut runtime, 1).outcome(),
        PumpOutcome::Quiescent
    );
    assert_eq!(runtime.state().len(), 65);
}

#[test]
fn fanout_children_keep_fifo_order_behind_unrelated_admitted_work() {
    let mut runtime = AppRuntime::<CausalQueueApp>::mount_with_config(
        Vec::new(),
        RuntimeConfig::default().with_queue_capacity(4),
    )
    .unwrap_or_else(|error| unreachable!("unexpected initial admission refusal: {error}"));
    let _ = pump_actions(&mut runtime, 1);
    runtime
        .submit_action(CausalAction::FanA)
        .unwrap_or_else(|_| unreachable!());
    runtime
        .submit_action(CausalAction::Follower)
        .unwrap_or_else(|_| unreachable!());

    let zero = pump_actions(&mut runtime, 0);
    assert_eq!(zero.processed_envelopes(), 0);
    assert_eq!(zero.remaining_queued_envelopes(), 2);
    assert_eq!(runtime.state().len(), 0);

    assert_eq!(pump_actions(&mut runtime, 1).processed_envelopes(), 1);
    assert_eq!(runtime.state(), &[CausalAction::FanA]);
    assert_eq!(pump_actions(&mut runtime, 1).processed_envelopes(), 1);
    assert_eq!(
        runtime.state(),
        &[CausalAction::FanA, CausalAction::Follower]
    );
    assert_eq!(pump_actions(&mut runtime, 1).processed_envelopes(), 1);
    assert_eq!(
        runtime.state(),
        &[
            CausalAction::FanA,
            CausalAction::Follower,
            CausalAction::FanA
        ]
    );
    assert_eq!(pump_actions(&mut runtime, 1).processed_envelopes(), 1);
    assert_eq!(
        runtime.state(),
        &[
            CausalAction::FanA,
            CausalAction::Follower,
            CausalAction::FanA,
            CausalAction::FanB,
        ]
    );
    assert!(matches!(runtime.status(), RuntimeStatus::Running));
}

#[test]
fn growing_causal_frontier_shortage_is_typed_atomic_and_clears_root_work() {
    let mut runtime = AppRuntime::<CausalQueueApp>::mount_with_config(
        Vec::new(),
        RuntimeConfig::default().with_queue_capacity(8),
    )
    .unwrap_or_else(|error| unreachable!("unexpected initial admission refusal: {error}"));
    let _ = pump_actions(&mut runtime, 1);
    runtime
        .submit_action(CausalAction::Grow)
        .unwrap_or_else(|_| unreachable!());
    let report = pump_actions(&mut runtime, 1);
    assert_eq!(report.remaining_queued_envelopes(), 2);

    let violation = pump_actions(&mut runtime, 1);
    assert_eq!(runtime.state(), &[CausalAction::Grow, CausalAction::Grow]);
    let RuntimeStatus::Terminal(RuntimeTerminalReason::ProducerContractViolation(cause)) =
        runtime.status()
    else {
        unreachable!("finite authored root frontier shortage is typed");
    };
    assert_eq!(
        cause.kind,
        runenui_core::ProducerViolationKind::DeclarationExceeded
    );
    assert_eq!(cause.phase, runenui_core::ProducerPhase::ApplicationUpdate);
    assert_eq!(
        cause.component,
        runenui_core::BudgetComponent::QueueFrontier
    );
    assert_eq!(cause.declared, 2);
    assert_eq!(cause.attempted, Some(3));
    assert_eq!(
        violation.outcome(),
        PumpOutcome::Terminal(RuntimeTerminalReason::ProducerContractViolation(cause))
    );
    assert_eq!(violation.remaining_queued_envelopes(), 0);
    let accepted_sequences: Vec<_> = runtime
        .trace()
        .records()
        .filter(|record| {
            matches!(
                record.kind(),
                runenui_runtime::TraceRecordKind::ActionSubmissionAccepted
            )
        })
        .filter_map(runenui_runtime::TraceRecord::work_sequence)
        .map(runenui_core::WorkSequence::get)
        .collect();
    assert_eq!(accepted_sequences, [2, 3, 4]);
}

#[test]
fn error_debugging_does_not_require_action_debug() {
    let config = RuntimeConfig::default().with_queue_capacity(1);
    let mut runtime = AppRuntime::<LocalOnlyApp>::mount_with_config(0, config)
        .unwrap_or_else(|error| unreachable!("unexpected initial admission refusal: {error}"));
    let Err(error) = runtime.submit_action(LocalOnlyAction(Rc::new(()))) else {
        unreachable!()
    };
    assert_eq!(format!("{error:?}"), "SubmitActionError { kind: Full, .. }");
    assert!(matches!(error, SubmitActionError::Full(_)));
}
