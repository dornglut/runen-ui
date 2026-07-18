#![allow(refining_impl_trait)]

use core::task::Poll;

use runenui_core::{
    Effects, Element, IntoEffects, NoHostProtocol, SubscriptionSet, UiApp, View, Widget,
    WidgetUpdateContext, WorkKey, text,
};
use runenui_runtime::{
    AppRuntime, PumpBudget, TraceRecordKind, TraceWorkFamily, TraceWorkOwner, WorkSequence,
};

struct TraceApp;

impl UiApp for TraceApp {
    type State = usize;
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root(_: &Self::State) -> impl View<Self::Action> {
        text("trace")
    }
    fn initial_effects(_: &Self::State) -> impl IntoEffects<Self::Action, Self::HostProtocol> {
        Effects::local_task(async { Some(()) })
    }
    fn update(state: &mut Self::State, (): Self::Action) {
        *state += 1;
    }
}

#[test]
fn scheduler_trace_covers_effect_checkpoint_update_and_redraw_transitions() {
    let mut runtime = AppRuntime::<TraceApp>::mount(0);
    runtime.pump(PumpBudget::new(3, 0, 1, 0));
    let kinds: Vec<_> = runtime.trace().kinds().collect();
    assert!(
        kinds
            .iter()
            .any(|kind| matches!(kind, TraceRecordKind::InitialEffectsCommitted { count: 1 }))
    );
    let accepted = runtime
        .trace()
        .records()
        .find(|record| matches!(record.kind(), TraceRecordKind::WorkStartAccepted))
        .unwrap_or_else(|| unreachable!());
    let identity = accepted.work().unwrap_or_else(|| unreachable!());
    assert_eq!(identity.owner(), &TraceWorkOwner::Application);
    assert_eq!(identity.family(), TraceWorkFamily::LocalTask);
    assert_eq!(identity.generation(), 1);
    assert_eq!(identity.key(), None);
    assert!(
        kinds
            .iter()
            .any(|kind| matches!(kind, TraceRecordKind::WorkStartAttempted))
    );
    assert!(
        kinds
            .iter()
            .any(|kind| matches!(kind, TraceRecordKind::LocalWorkPolled))
    );
    assert!(
        kinds
            .iter()
            .any(|kind| matches!(kind, TraceRecordKind::LocalWorkReady))
    );
    assert!(kinds.iter().any(|kind| matches!(
        kind,
        TraceRecordKind::ReadinessCheckpoint {
            polled_local_work: 1,
            ..
        }
    )));
    assert!(
        kinds
            .iter()
            .any(|kind| matches!(kind, TraceRecordKind::UpdateEffectsCommitted { count: 0 }))
    );
    assert!(
        kinds
            .iter()
            .any(|kind| matches!(kind, TraceRecordKind::RedrawRequested { .. }))
    );

    assert_local_task_trace_chain(&runtime);
}

fn assert_local_task_trace_chain(runtime: &AppRuntime<TraceApp>) {
    let records: Vec<_> = runtime.trace().records().collect();
    let requested = records
        .iter()
        .find(|record| matches!(record.kind(), TraceRecordKind::WorkRequested))
        .unwrap_or_else(|| unreachable!());
    let initial_transaction = records
        .iter()
        .find(|record| {
            matches!(
                record.kind(),
                TraceRecordKind::InitialApplicationTransactionStarted
            )
        })
        .unwrap_or_else(|| unreachable!());
    assert_eq!(
        requested.causal_parent(),
        Some(initial_transaction.sequence())
    );
    let committed = records
        .iter()
        .find(|record| {
            matches!(record.kind(), TraceRecordKind::WorkGenerationCommitted)
                && record.work() == requested.work()
        })
        .unwrap_or_else(|| unreachable!());
    assert_eq!(committed.causal_parent(), Some(requested.sequence()));
    let attempted = records
        .iter()
        .find(|record| {
            matches!(record.kind(), TraceRecordKind::WorkStartAttempted)
                && record.work() == requested.work()
        })
        .unwrap_or_else(|| unreachable!());
    assert_eq!(attempted.work_sequence().map(WorkSequence::get), Some(2));
    assert_eq!(attempted.causal_parent(), Some(committed.sequence()));
    let accepted = records
        .iter()
        .find(|record| {
            matches!(record.kind(), TraceRecordKind::WorkStartAccepted)
                && record.work() == requested.work()
        })
        .unwrap_or_else(|| unreachable!());
    assert_eq!(accepted.work_sequence(), attempted.work_sequence());
    assert_eq!(accepted.causal_parent(), Some(attempted.sequence()));
    let polled = records
        .iter()
        .find(|record| {
            matches!(record.kind(), TraceRecordKind::LocalWorkPolled)
                && record.work() == requested.work()
        })
        .unwrap_or_else(|| unreachable!());
    assert_eq!(polled.causal_parent(), Some(accepted.sequence()));
    let ready = records
        .iter()
        .find(|record| {
            matches!(record.kind(), TraceRecordKind::LocalWorkReady)
                && record.work() == requested.work()
        })
        .unwrap_or_else(|| unreachable!());
    assert_eq!(ready.causal_parent(), Some(polled.sequence()));
    let produced_action = records
        .iter()
        .find(|record| {
            matches!(record.kind(), TraceRecordKind::ActionSubmissionAccepted)
                && record.causal_parent() == Some(ready.sequence())
        })
        .unwrap_or_else(|| unreachable!());
    assert_eq!(
        produced_action.work_sequence().map(WorkSequence::get),
        Some(3)
    );
    let application_transaction = records
        .iter()
        .find(|record| {
            matches!(
                record.kind(),
                TraceRecordKind::ApplicationActionTransactionStarted
            ) && record.work_sequence() == produced_action.work_sequence()
        })
        .unwrap_or_else(|| unreachable!());
    assert_eq!(
        application_transaction.causal_parent(),
        Some(produced_action.sequence())
    );
}

#[derive(Clone, Copy)]
enum OrderedAction {
    Trigger,
    UpdateOutput,
    MountedOutput,
}

#[derive(Debug)]
struct OrderedWidget {
    revision: u64,
}

impl Widget<OrderedAction> for OrderedWidget {
    type State = u64;

    fn create_state(&self) -> Self::State {
        self.revision
    }

    fn update(&self, state: &mut Self::State, context: &mut WidgetUpdateContext<OrderedAction>) {
        if *state != self.revision {
            *state = self.revision;
            context.invalidate_subscriptions();
            context.emit(OrderedAction::MountedOutput);
        }
    }

    fn subscriptions(&self, _: &Self::State, subscriptions: &mut SubscriptionSet<OrderedAction>) {
        subscriptions.local(
            WorkKey::new("ordered.mounted").unwrap_or_else(|_| unreachable!()),
            self.revision,
            |_: &mut core::task::Context<'_>| Poll::Pending,
        );
    }
}

struct OrderedState {
    revision: u64,
}

struct OrderedTransactionApp;

impl UiApp for OrderedTransactionApp {
    type State = OrderedState;
    type Action = OrderedAction;
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> impl View<Self::Action> {
        Element::new(OrderedWidget {
            revision: state.revision,
        })
        .key("ordered-widget")
    }

    fn update(
        state: &mut Self::State,
        action: Self::Action,
    ) -> impl IntoEffects<Self::Action, Self::HostProtocol> {
        if matches!(action, OrderedAction::Trigger) {
            state.revision += 1;
            return Effects::action(OrderedAction::UpdateOutput)
                .then(Effects::local_task(async { None::<OrderedAction> }));
        }
        Effects::none()
    }

    fn subscriptions(state: &Self::State, subscriptions: &mut SubscriptionSet<Self::Action>) {
        subscriptions.local(
            WorkKey::new("ordered.application").unwrap_or_else(|_| unreachable!()),
            state.revision,
            |_: &mut core::task::Context<'_>| Poll::Pending,
        );
    }
}

fn sequence_value(sequence: Option<WorkSequence>) -> u64 {
    sequence
        .unwrap_or_else(|| unreachable!("scheduler record carries its queue sequence"))
        .get()
}

#[test]
fn application_transaction_assigns_the_global_adr_order_exactly() {
    let mut runtime = AppRuntime::<OrderedTransactionApp>::mount(OrderedState { revision: 0 });
    runtime.pump(PumpBudget::new(
        usize::MAX,
        usize::MAX,
        usize::MAX,
        usize::MAX,
    ));
    let trigger_sequence = runtime
        .submit_action(OrderedAction::Trigger)
        .unwrap_or_else(|_| unreachable!());
    runtime.pump(PumpBudget::new(
        usize::MAX,
        usize::MAX,
        usize::MAX,
        usize::MAX,
    ));

    let records: Vec<_> = runtime.trace().records().collect();
    let transaction = records
        .iter()
        .find(|record| {
            matches!(
                record.kind(),
                TraceRecordKind::ApplicationActionTransactionStarted
            ) && record.work_sequence() == Some(trigger_sequence)
        })
        .unwrap_or_else(|| unreachable!());
    let transaction_sequence = transaction.sequence();
    let application_cancellation = records
        .iter()
        .find(|record| {
            matches!(record.kind(), TraceRecordKind::WorkCleanupProcessed)
                && record.work().is_some_and(|work| {
                    work.owner() == &TraceWorkOwner::Application
                        && work.family() == TraceWorkFamily::Subscription
                })
                && record.sequence().get() > transaction_sequence.get()
        })
        .unwrap_or_else(|| unreachable!());
    let mounted_reconciliation = records
        .iter()
        .find(|record| {
            matches!(
                record.kind(),
                TraceRecordKind::SubscriptionDiffCommitted { .. }
            ) && record.work_sequence().is_some()
                && record.sequence().get() > transaction_sequence.get()
        })
        .unwrap_or_else(|| unreachable!());
    let transaction_actions: Vec<_> = records
        .iter()
        .filter(|record| {
            matches!(record.kind(), TraceRecordKind::ActionSubmissionAccepted)
                && record.causal_parent() == Some(transaction_sequence)
        })
        .collect();
    assert_eq!(transaction_actions.len(), 2);
    let task_start = records
        .iter()
        .find(|record| {
            matches!(record.kind(), TraceRecordKind::WorkStartAttempted)
                && record.work().is_some_and(|work| {
                    work.owner() == &TraceWorkOwner::Application
                        && work.family() == TraceWorkFamily::LocalTask
                })
                && record.sequence().get() > transaction_sequence.get()
        })
        .unwrap_or_else(|| unreachable!());
    let application_subscription_start = records
        .iter()
        .rfind(|record| {
            matches!(record.kind(), TraceRecordKind::WorkStartAttempted)
                && record.work().is_some_and(|work| {
                    work.owner() == &TraceWorkOwner::Application
                        && work.family() == TraceWorkFamily::Subscription
                })
        })
        .unwrap_or_else(|| unreachable!());

    let ordered = [
        sequence_value(application_cancellation.work_sequence()),
        sequence_value(mounted_reconciliation.work_sequence()),
        sequence_value(transaction_actions[0].work_sequence()),
        sequence_value(task_start.work_sequence()),
        sequence_value(application_subscription_start.work_sequence()),
        sequence_value(transaction_actions[1].work_sequence()),
    ];
    assert!(ordered.windows(2).all(|pair| pair[1] == pair[0] + 1));
}
