#![allow(refining_impl_trait)]

use core::{
    pin::Pin,
    task::{Context, Poll, Waker},
};
use std::{
    cell::Cell,
    rc::Rc,
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicUsize, Ordering},
    },
};

use runenui_core::{
    LocalSubscriptionSource, SendSubscriptionSink, SendSubscriptionSinkError,
    SendSubscriptionSource, SendSubscriptionStartOutcome, SubscriptionSet, UiApp, View, WorkKey,
    text,
};
use runenui_runtime::{
    AppRuntime, PumpBudget, RuntimeConfig, RuntimeLimits, TraceRecordKind, TraceWorkFamily,
    TraceWorkStartRefusal,
};

fn subscription_fact<'a, App: UiApp>(
    runtime: &'a AppRuntime<App>,
    key: &WorkKey,
    predicate: impl Fn(&TraceRecordKind) -> bool,
) -> &'a runenui_runtime::TraceRecord {
    runtime
        .trace()
        .records()
        .find(|record| {
            predicate(record.kind())
                && record.work().is_some_and(|identity| {
                    identity.family() == TraceWorkFamily::Subscription
                        && identity.key() == Some(key)
                })
        })
        .unwrap_or_else(|| unreachable!("subscription scheduler fact is present"))
}

fn assert_subscription_final_action<App: UiApp>(
    runtime: &AppRuntime<App>,
    terminal: &runenui_runtime::TraceRecord,
) {
    let action = runtime
        .trace()
        .records()
        .find(|record| {
            matches!(record.kind(), TraceRecordKind::ActionSubmissionAccepted)
                && record.causal_parent() == Some(terminal.sequence())
        })
        .unwrap_or_else(|| unreachable!("subscription accepts one final action"));
    let transaction = runtime
        .trace()
        .records()
        .find(|record| {
            matches!(
                record.kind(),
                TraceRecordKind::ApplicationActionTransactionStarted
            ) && record.work_sequence() == action.work_sequence()
        })
        .unwrap_or_else(|| unreachable!("subscription action enters application update"));
    assert_eq!(transaction.causal_parent(), Some(action.sequence()));
}

fn key(value: &str) -> WorkKey {
    WorkKey::new(value).unwrap_or_else(|_| unreachable!())
}

fn lock<T>(value: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    value
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

#[derive(Clone, Copy)]
enum LocalAction {
    Disable,
}

struct LocalControl {
    polls: AtomicUsize,
    ready: AtomicBool,
    waker: Mutex<Option<Waker>>,
    order: Arc<Mutex<Vec<u8>>>,
}

impl LocalControl {
    const fn new(order: Arc<Mutex<Vec<u8>>>) -> Self {
        Self {
            polls: AtomicUsize::new(0),
            ready: AtomicBool::new(false),
            waker: Mutex::new(None),
            order,
        }
    }

    fn wake(&self) {
        self.ready.store(true, Ordering::Release);
        let waker = lock(&self.waker).take();
        if let Some(waker) = waker {
            waker.wake();
        }
    }
}

struct WakeSource {
    id: u8,
    control: Arc<LocalControl>,
}

impl LocalSubscriptionSource<LocalAction> for WakeSource {
    fn poll_next(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Option<LocalAction>> {
        let this = self.get_mut();
        this.control.polls.fetch_add(1, Ordering::Relaxed);
        lock(&this.control.order).push(this.id);
        if this.control.ready.swap(false, Ordering::AcqRel) {
            Poll::Ready(Some(LocalAction::Disable))
        } else {
            *lock(&this.control.waker) = Some(context.waker().clone());
            Poll::Pending
        }
    }
}

struct LocalState {
    active: bool,
    controls: [Arc<LocalControl>; 2],
    updates: usize,
}

struct LocalApp;

impl UiApp for LocalApp {
    type State = LocalState;
    type Action = LocalAction;
    type HostProtocol = runenui_core::NoHostProtocol;

    fn root(_: &Self::State) -> impl View<Self::Action> {
        text("wake-aware subscriptions")
    }

    fn update(state: &mut Self::State, LocalAction::Disable: Self::Action) {
        state.active = false;
        state.updates += 1;
    }

    fn subscriptions(state: &Self::State, subscriptions: &mut SubscriptionSet<Self::Action>) {
        if !state.active {
            return;
        }
        for (index, control) in state.controls.iter().enumerate() {
            subscriptions.local(
                key(if index == 0 {
                    "local.first"
                } else {
                    "local.second"
                }),
                0,
                WakeSource {
                    id: u8::try_from(index + 1).unwrap_or_else(|_| unreachable!()),
                    control: Arc::clone(control),
                },
            );
        }
    }
}

#[test]
fn local_sources_share_budget_sleep_wake_in_order_and_stop_after_cancellation() {
    let order = Arc::new(Mutex::new(Vec::new()));
    let first = Arc::new(LocalControl::new(Arc::clone(&order)));
    let second = Arc::new(LocalControl::new(Arc::clone(&order)));
    let mut runtime = AppRuntime::<LocalApp>::mount(LocalState {
        active: true,
        controls: [Arc::clone(&first), Arc::clone(&second)],
        updates: 0,
    });

    let first_report = runtime.pump(PumpBudget::new(16, usize::MAX, 1, usize::MAX));
    assert_eq!(first_report.polled_local_work(), 1);
    assert_eq!(&*lock(&order), &[1]);

    let second_report = runtime.pump(PumpBudget::new(16, usize::MAX, 1, usize::MAX));
    assert_eq!(second_report.polled_local_work(), 1);
    assert_eq!(&*lock(&order), &[1, 2]);

    let sleeping = runtime.pump(PumpBudget::new(16, usize::MAX, 8, usize::MAX));
    assert!(sleeping.is_quiescent());
    assert_eq!(sleeping.polled_local_work(), 0);

    second.wake();
    runtime.pump(PumpBudget::new(32, usize::MAX, 1, usize::MAX));
    assert_eq!(runtime.state().updates, 1);
    assert!(!runtime.state().active);
    let polls_after_cancel = second.polls.load(Ordering::Relaxed);

    second.wake();
    let cancelled = runtime.pump(PumpBudget::new(16, usize::MAX, 8, usize::MAX));
    assert!(cancelled.is_quiescent());
    assert_eq!(second.polls.load(Ordering::Relaxed), polls_after_cancel);

    let identity_key = key("local.second");
    let requested = subscription_fact(&runtime, &identity_key, |kind| {
        matches!(kind, TraceRecordKind::WorkRequested)
    });
    let declared = subscription_fact(&runtime, &identity_key, |kind| {
        matches!(kind, TraceRecordKind::SubscriptionDeclared)
    });
    let committed = subscription_fact(&runtime, &identity_key, |kind| {
        matches!(kind, TraceRecordKind::WorkGenerationCommitted)
    });
    let attempted = subscription_fact(&runtime, &identity_key, |kind| {
        matches!(kind, TraceRecordKind::WorkStartAttempted)
    });
    let accepted = subscription_fact(&runtime, &identity_key, |kind| {
        matches!(kind, TraceRecordKind::WorkStartAccepted)
    });
    assert_eq!(declared.causal_parent(), Some(requested.sequence()));
    assert_eq!(committed.causal_parent(), Some(declared.sequence()));
    assert_eq!(attempted.causal_parent(), Some(committed.sequence()));
    assert_eq!(accepted.causal_parent(), Some(attempted.sequence()));
    let polls: Vec<_> = runtime
        .trace()
        .records()
        .filter(|record| {
            matches!(record.kind(), TraceRecordKind::LocalWorkPolled)
                && record
                    .work()
                    .is_some_and(|identity| identity.key() == Some(&identity_key))
        })
        .collect();
    assert_eq!(polls[0].causal_parent(), Some(accepted.sequence()));
    assert!(
        polls
            .windows(2)
            .all(|pair| pair[1].causal_parent() == Some(pair[0].sequence()))
    );
    let ready = subscription_fact(&runtime, &identity_key, |kind| {
        matches!(kind, TraceRecordKind::LocalWorkReady)
    });
    assert_eq!(
        ready.causal_parent(),
        polls.last().map(|record| record.sequence())
    );
    assert_subscription_final_action(&runtime, ready);
}

#[derive(Debug)]
struct SendItem(u8);

struct SendControl {
    starts: AtomicUsize,
    sink: Mutex<Option<SendSubscriptionSink<Arc<SendItem>>>>,
}

struct RecoveringSendSource {
    control: Arc<SendControl>,
}

impl SendSubscriptionSource<Arc<SendItem>> for RecoveringSendSource {
    fn start(
        self: Box<Self>,
        sink: SendSubscriptionSink<Arc<SendItem>>,
    ) -> SendSubscriptionStartOutcome {
        self.control.starts.fetch_add(1, Ordering::Relaxed);
        *lock(&self.control.sink) = Some(sink);
        SendSubscriptionStartOutcome::Started
    }
}

struct SendState {
    control: Arc<SendControl>,
    mapped: Rc<AtomicBool>,
}

struct SendApp;

impl UiApp for SendApp {
    type State = SendState;
    type Action = Rc<()>;
    type HostProtocol = runenui_core::NoHostProtocol;

    fn root(_: &Self::State) -> impl View<Self::Action> {
        text("bounded send subscription")
    }

    fn update(_: &mut Self::State, _: Self::Action) {}

    fn subscriptions(state: &Self::State, subscriptions: &mut SubscriptionSet<Self::Action>) {
        let mapped = Rc::clone(&state.mapped);
        subscriptions.send(
            key("send.bounded"),
            0,
            RecoveringSendSource {
                control: Arc::clone(&state.control),
            },
            move |item| {
                let _ = item.0;
                mapped.store(true, Ordering::Relaxed);
                Rc::new(())
            },
        );
    }
}

#[test]
fn send_source_starts_once_and_full_or_closed_sink_returns_the_exact_item() {
    let first = Arc::new(SendItem(1));
    let control = Arc::new(SendControl {
        starts: AtomicUsize::new(0),
        sink: Mutex::new(None),
    });
    let mapped = Rc::new(AtomicBool::new(false));
    let limits = RuntimeLimits::default().with_completion_ingress(0);
    let mut runtime = AppRuntime::<SendApp>::mount_with_config(
        SendState {
            control: Arc::clone(&control),
            mapped: Rc::clone(&mapped),
        },
        RuntimeConfig::default().with_limits(limits),
    );

    runtime.pump(PumpBudget::new(16, usize::MAX, usize::MAX, usize::MAX));
    assert_eq!(control.starts.load(Ordering::Relaxed), 1);
    let full_sink = lock(&control.sink)
        .as_ref()
        .cloned()
        .unwrap_or_else(|| unreachable!("source retained its exact sink"));
    let submitted = Arc::clone(&first);
    let recovered = std::thread::spawn(move || full_sink.try_send(submitted))
        .join()
        .unwrap_or_else(|_| unreachable!("producer thread remains deterministic"));
    let Err(SendSubscriptionSinkError::Full(recovered)) = recovered else {
        unreachable!("capacity zero returns the item");
    };
    assert!(Arc::ptr_eq(&recovered, &first));
    assert!(!mapped.load(Ordering::Relaxed));

    runtime.pump(PumpBudget::new(16, usize::MAX, usize::MAX, usize::MAX));
    assert_eq!(control.starts.load(Ordering::Relaxed), 1);
    runtime.shutdown();

    let second = Arc::new(SendItem(2));
    let sink = lock(&control.sink)
        .take()
        .unwrap_or_else(|| unreachable!("source retained its exact sink"));
    let Err(SendSubscriptionSinkError::Closed(closed)) = sink.try_send(Arc::clone(&second)) else {
        unreachable!("closed ingress returns the item");
    };
    assert!(Arc::ptr_eq(&closed, &second));
}

#[derive(Clone, Copy)]
enum StartAction {
    Item,
    Retry,
    Disable,
}

struct StartControl {
    starts: AtomicUsize,
    mapped: AtomicUsize,
    startup_not_started: AtomicUsize,
    sink: Mutex<Option<SendSubscriptionSink<Arc<SendItem>>>>,
}

struct OutcomeSource {
    control: Arc<StartControl>,
    outcome: SendSubscriptionStartOutcome,
}

impl SendSubscriptionSource<Arc<SendItem>> for OutcomeSource {
    fn start(
        self: Box<Self>,
        sink: SendSubscriptionSink<Arc<SendItem>>,
    ) -> SendSubscriptionStartOutcome {
        self.control.starts.fetch_add(1, Ordering::Relaxed);
        *lock(&self.control.sink) = Some(sink.clone());
        let item = Arc::new(SendItem(9));
        let Err(SendSubscriptionSinkError::NotStarted(recovered)) =
            sink.try_send(Arc::clone(&item))
        else {
            unreachable!("a source cannot submit before startup commits")
        };
        assert!(Arc::ptr_eq(&recovered, &item));
        self.control
            .startup_not_started
            .fetch_add(1, Ordering::Relaxed);
        self.outcome
    }
}

struct StartState {
    control: Arc<StartControl>,
    outcome: SendSubscriptionStartOutcome,
    revision: u64,
    active: bool,
}

struct StartOutcomeApp;

impl UiApp for StartOutcomeApp {
    type State = StartState;
    type Action = StartAction;
    type HostProtocol = runenui_core::NoHostProtocol;

    fn root(_: &Self::State) -> impl View<Self::Action> {
        text("send subscription start outcomes")
    }

    fn update(state: &mut Self::State, action: Self::Action) {
        match action {
            StartAction::Item => {}
            StartAction::Retry => {
                state.outcome = SendSubscriptionStartOutcome::Started;
                state.revision += 1;
            }
            StartAction::Disable => state.active = false,
        }
    }

    fn subscriptions(state: &Self::State, subscriptions: &mut SubscriptionSet<Self::Action>) {
        if !state.active {
            return;
        }
        let control = Arc::clone(&state.control);
        subscriptions.send(
            key("send.start-outcome"),
            state.revision,
            OutcomeSource {
                control: Arc::clone(&control),
                outcome: state.outcome,
            },
            move |item| {
                let _ = item.0;
                control.mapped.fetch_add(1, Ordering::Relaxed);
                StartAction::Item
            },
        );
    }
}

fn start_runtime(
    outcome: SendSubscriptionStartOutcome,
) -> (AppRuntime<StartOutcomeApp>, Arc<StartControl>) {
    let control = Arc::new(StartControl {
        starts: AtomicUsize::new(0),
        mapped: AtomicUsize::new(0),
        startup_not_started: AtomicUsize::new(0),
        sink: Mutex::new(None),
    });
    let runtime = AppRuntime::<StartOutcomeApp>::mount(StartState {
        control: Arc::clone(&control),
        outcome,
        revision: 0,
        active: true,
    });
    (runtime, control)
}

#[test]
fn send_subscription_start_outcomes_are_once_only_reclaimed_and_explicitly_retryable() {
    for (outcome, refusal) in [
        (
            SendSubscriptionStartOutcome::Unavailable,
            TraceWorkStartRefusal::SubscriptionUnavailable,
        ),
        (
            SendSubscriptionStartOutcome::Full,
            TraceWorkStartRefusal::SubscriptionFull,
        ),
        (
            SendSubscriptionStartOutcome::Closed,
            TraceWorkStartRefusal::SubscriptionClosed,
        ),
        (
            SendSubscriptionStartOutcome::Rejected,
            TraceWorkStartRefusal::SubscriptionRejected,
        ),
    ] {
        let (mut runtime, control) = start_runtime(outcome);
        runtime.pump(PumpBudget::new(32, usize::MAX, usize::MAX, usize::MAX));
        assert_eq!(control.starts.load(Ordering::Relaxed), 1);
        assert_eq!(control.startup_not_started.load(Ordering::Relaxed), 1);
        assert_eq!(control.mapped.load(Ordering::Relaxed), 0);
        assert_eq!(runtime.__live_work_record_count_for_test(), 0);
        runtime.pump(PumpBudget::new(32, usize::MAX, usize::MAX, usize::MAX));
        assert_eq!(control.starts.load(Ordering::Relaxed), 1);
        assert!(runtime.trace().records().any(|record| matches!(
            record.kind(),
            TraceRecordKind::WorkStartRefused { outcome } if *outcome == refusal
        )));

        let stale_item = Arc::new(SendItem(10));
        let sink = lock(&control.sink)
            .as_ref()
            .cloned()
            .unwrap_or_else(|| unreachable!("start attempt retained its sink"));
        let Err(SendSubscriptionSinkError::Stale(recovered)) =
            sink.try_send(Arc::clone(&stale_item))
        else {
            unreachable!("refused generation has stale sink authority")
        };
        assert!(Arc::ptr_eq(&recovered, &stale_item));

        runtime
            .submit_action(StartAction::Retry)
            .unwrap_or_else(|_| unreachable!());
        runtime.pump(PumpBudget::new(64, usize::MAX, usize::MAX, usize::MAX));
        assert_eq!(control.starts.load(Ordering::Relaxed), 2);
        let sink = lock(&control.sink)
            .as_ref()
            .cloned()
            .unwrap_or_else(|| unreachable!("retry retained its started sink"));
        sink.try_send(Arc::new(SendItem(12)))
            .unwrap_or_else(|_| unreachable!("post-start item is accepted"));
        runtime.pump(PumpBudget::new(64, usize::MAX, usize::MAX, usize::MAX));
        assert_eq!(control.mapped.load(Ordering::Relaxed), 1);
        assert_eq!(runtime.__live_work_record_count_for_test(), 1);
    }
}

#[test]
fn started_send_subscription_starts_once_and_maps_accepted_items() {
    let (mut runtime, control) = start_runtime(SendSubscriptionStartOutcome::Started);
    runtime.pump(PumpBudget::new(32, usize::MAX, usize::MAX, usize::MAX));
    assert_eq!(control.starts.load(Ordering::Relaxed), 1);
    assert_eq!(control.startup_not_started.load(Ordering::Relaxed), 1);
    assert_eq!(control.mapped.load(Ordering::Relaxed), 0);
    let sink = lock(&control.sink)
        .as_ref()
        .cloned()
        .unwrap_or_else(|| unreachable!("started source retained its sink"));
    sink.try_send(Arc::new(SendItem(13)))
        .unwrap_or_else(|_| unreachable!("post-start item is accepted"));
    runtime.pump(PumpBudget::new(32, usize::MAX, usize::MAX, usize::MAX));
    assert_eq!(control.mapped.load(Ordering::Relaxed), 1);
    assert_eq!(runtime.__live_work_record_count_for_test(), 1);
    assert!(
        runtime
            .trace()
            .records()
            .any(|record| matches!(record.kind(), TraceRecordKind::WorkStartAccepted))
    );
    let identity_key = key("send.start-outcome");
    let requested = subscription_fact(&runtime, &identity_key, |kind| {
        matches!(kind, TraceRecordKind::WorkRequested)
    });
    let declared = subscription_fact(&runtime, &identity_key, |kind| {
        matches!(kind, TraceRecordKind::SubscriptionDeclared)
    });
    let committed = subscription_fact(&runtime, &identity_key, |kind| {
        matches!(kind, TraceRecordKind::WorkGenerationCommitted)
    });
    let attempted = subscription_fact(&runtime, &identity_key, |kind| {
        matches!(kind, TraceRecordKind::WorkStartAttempted)
    });
    let accepted = subscription_fact(&runtime, &identity_key, |kind| {
        matches!(kind, TraceRecordKind::WorkStartAccepted)
    });
    let imported = subscription_fact(&runtime, &identity_key, |kind| {
        matches!(kind, TraceRecordKind::WorkCompletionImported)
    });
    let mapped = subscription_fact(&runtime, &identity_key, |kind| {
        matches!(kind, TraceRecordKind::WorkCompletionMapped)
    });
    assert_eq!(declared.causal_parent(), Some(requested.sequence()));
    assert_eq!(committed.causal_parent(), Some(declared.sequence()));
    assert_eq!(attempted.causal_parent(), Some(committed.sequence()));
    assert_eq!(accepted.causal_parent(), Some(attempted.sequence()));
    assert_eq!(imported.causal_parent(), Some(accepted.sequence()));
    assert_eq!(mapped.causal_parent(), Some(imported.sequence()));
    assert_subscription_final_action(&runtime, mapped);
}

#[cfg(feature = "internal-test-seams")]
fn trace_boundary_send_subscription() -> (AppRuntime<StartOutcomeApp>, Arc<StartControl>) {
    let (mut runtime, control) = start_runtime(SendSubscriptionStartOutcome::Started);
    runtime.pump(PumpBudget::new(8, usize::MAX, usize::MAX, usize::MAX));
    (runtime, control)
}

#[cfg(feature = "internal-test-seams")]
#[test]
fn send_subscription_item_admits_its_exact_three_record_plan_beside_publication_authority() {
    let (mut runtime, control) = trace_boundary_send_subscription();
    let sink = lock(&control.sink)
        .as_ref()
        .cloned()
        .unwrap_or_else(|| unreachable!("started source retained its sink"));
    sink.try_send(Arc::new(SendItem(21)))
        .unwrap_or_else(|_| unreachable!("post-start item enters ingress"));
    assert!(runtime.__surface_publication_trace_reserved_for_test());
    runtime.__seed_next_trace_sequence_for_test(u64::MAX - 3);
    runtime.pump(PumpBudget::new(0, 1, 0, 0));

    assert_eq!(control.mapped.load(Ordering::Relaxed), 1);
    assert_eq!(runtime.status(), runenui_runtime::RuntimeStatus::Running);
    assert_eq!(runtime.__subscription_slot_count_for_test(), 1);
    let kinds: Vec<_> = runtime.trace().kinds().collect();
    let tail = &kinds[kinds.len() - 3..];
    assert!(matches!(tail[0], TraceRecordKind::WorkCompletionImported));
    assert!(matches!(tail[1], TraceRecordKind::WorkCompletionMapped));
    assert!(matches!(tail[2], TraceRecordKind::ActionSubmissionAccepted));
}

#[cfg(feature = "internal-test-seams")]
#[test]
fn send_subscription_item_with_only_two_unreserved_records_never_runs_mapper() {
    let (mut runtime, control) = trace_boundary_send_subscription();
    let sink = lock(&control.sink)
        .as_ref()
        .cloned()
        .unwrap_or_else(|| unreachable!("started source retained its sink"));
    sink.try_send(Arc::new(SendItem(22)))
        .unwrap_or_else(|_| unreachable!("post-start item enters ingress"));
    assert!(runtime.__surface_publication_trace_reserved_for_test());
    runtime.__seed_next_trace_sequence_for_test(u64::MAX - 2);
    runtime.pump(PumpBudget::new(0, 1, 0, 0));

    assert_eq!(control.mapped.load(Ordering::Relaxed), 0);
    assert_eq!(
        runtime.status(),
        runenui_runtime::RuntimeStatus::Terminal(
            runenui_runtime::RuntimeTerminalReason::TraceSequenceExhausted
        )
    );
    assert_eq!(runtime.__subscription_slot_count_for_test(), 0);
    assert_eq!(runtime.__completion_payload_count_for_test(), 0);
}

#[test]
fn cancelled_send_subscription_sink_returns_the_exact_stale_item() {
    let (mut runtime, control) = start_runtime(SendSubscriptionStartOutcome::Started);
    runtime.pump(PumpBudget::new(32, usize::MAX, usize::MAX, usize::MAX));
    let sink = lock(&control.sink)
        .as_ref()
        .cloned()
        .unwrap_or_else(|| unreachable!("started source retained its sink"));
    runtime
        .submit_action(StartAction::Disable)
        .unwrap_or_else(|_| unreachable!());
    runtime.pump(PumpBudget::new(32, usize::MAX, usize::MAX, usize::MAX));
    let item = Arc::new(SendItem(11));
    let Err(SendSubscriptionSinkError::Stale(recovered)) = sink.try_send(Arc::clone(&item)) else {
        unreachable!("cancelled generation is stale while runtime remains open")
    };
    assert!(Arc::ptr_eq(&recovered, &item));
    assert_eq!(control.mapped.load(Ordering::Relaxed), 0);
}

struct InitialReplacementState {
    revision: u64,
    old_starts: Arc<AtomicUsize>,
    new_starts: Arc<AtomicUsize>,
    old_maps: Rc<Cell<usize>>,
    new_maps: Rc<Cell<usize>>,
}

#[derive(Clone, Copy)]
enum InitialReplacementAction {
    Advance,
    Item,
}

struct CountingSource {
    starts: Arc<AtomicUsize>,
}

impl SendSubscriptionSource<u8> for CountingSource {
    fn start(self: Box<Self>, sink: SendSubscriptionSink<u8>) -> SendSubscriptionStartOutcome {
        self.starts.fetch_add(1, Ordering::Relaxed);
        let _ = sink.try_send(1);
        SendSubscriptionStartOutcome::Started
    }
}

struct InitialReplacementApp;

impl UiApp for InitialReplacementApp {
    type State = InitialReplacementState;
    type Action = InitialReplacementAction;
    type HostProtocol = runenui_core::NoHostProtocol;

    fn root(_: &Self::State) -> impl View<Self::Action> {
        text("initial replacement")
    }

    fn initial_effects(
        _: &Self::State,
    ) -> impl runenui_core::IntoEffects<Self::Action, Self::HostProtocol> {
        runenui_core::Effects::action(InitialReplacementAction::Advance)
    }

    fn update(state: &mut Self::State, action: Self::Action) {
        if matches!(action, InitialReplacementAction::Advance) {
            state.revision = 1;
        }
    }

    fn subscriptions(state: &Self::State, subscriptions: &mut SubscriptionSet<Self::Action>) {
        let old = state.revision == 0;
        let starts = if old {
            Arc::clone(&state.old_starts)
        } else {
            Arc::clone(&state.new_starts)
        };
        let maps = if old {
            Rc::clone(&state.old_maps)
        } else {
            Rc::clone(&state.new_maps)
        };
        subscriptions.send(
            key("initial.replacement"),
            state.revision,
            CountingSource { starts },
            move |_| {
                maps.set(maps.get() + 1);
                InitialReplacementAction::Item
            },
        );
    }
}

#[test]
fn initial_effect_action_replaces_the_old_subscription_before_its_start_callback() {
    let old_starts = Arc::new(AtomicUsize::new(0));
    let new_starts = Arc::new(AtomicUsize::new(0));
    let old_maps = Rc::new(Cell::new(0));
    let new_maps = Rc::new(Cell::new(0));
    let mut runtime = AppRuntime::<InitialReplacementApp>::mount(InitialReplacementState {
        revision: 0,
        old_starts: Arc::clone(&old_starts),
        new_starts: Arc::clone(&new_starts),
        old_maps: Rc::clone(&old_maps),
        new_maps: Rc::clone(&new_maps),
    });

    runtime.pump(PumpBudget::new(32, usize::MAX, usize::MAX, usize::MAX));
    assert_eq!(runtime.state().revision, 1);
    assert_eq!(old_starts.load(Ordering::Relaxed), 0);
    assert_eq!(old_maps.get(), 0);
    assert_eq!(new_starts.load(Ordering::Relaxed), 1);
    assert_eq!(new_maps.get(), 0);
}

struct ExactLocalSubscriptionApp;

impl UiApp for ExactLocalSubscriptionApp {
    type State = Rc<Cell<usize>>;
    type Action = ();
    type HostProtocol = runenui_core::NoHostProtocol;

    fn root(_: &Self::State) -> impl View<Self::Action> {
        text("exact local subscription")
    }

    fn update(_: &mut Self::State, (): Self::Action) {}

    fn subscriptions(state: &Self::State, subscriptions: &mut SubscriptionSet<Self::Action>) {
        let calls = Rc::clone(state);
        subscriptions.local(
            key("exact.local"),
            0,
            move |_: &mut core::task::Context<'_>| {
                calls.set(calls.get() + 1);
                Poll::Ready(Some(()))
            },
        );
    }
}

#[cfg(feature = "internal-test-seams")]
#[test]
fn one_remaining_sequence_is_the_final_local_or_send_subscription_action() {
    let local_calls = Rc::new(Cell::new(0));
    let mut local = AppRuntime::<ExactLocalSubscriptionApp>::mount(Rc::clone(&local_calls));
    local.pump(PumpBudget::new(2, 0, 0, 0));
    local.__seed_next_work_sequence_for_test(u64::MAX);
    local.pump(PumpBudget::new(0, 0, 1, 0));
    assert_eq!(local_calls.get(), 1);
    assert!(local.trace().records().any(|record| {
        matches!(record.kind(), TraceRecordKind::ActionSubmissionAccepted)
            && record
                .work_sequence()
                .is_some_and(|sequence| sequence.get() == u64::MAX)
    }));
    assert_eq!(local.status(), runenui_runtime::RuntimeStatus::Running);

    let (mut send, control) = start_runtime(SendSubscriptionStartOutcome::Started);
    send.pump(PumpBudget::new(2, 0, 0, 0));
    assert_eq!(control.mapped.load(Ordering::Relaxed), 0);
    let sink = lock(&control.sink)
        .as_ref()
        .cloned()
        .unwrap_or_else(|| unreachable!("started source retained its sink"));
    sink.try_send(Arc::new(SendItem(14)))
        .unwrap_or_else(|_| unreachable!("post-start item is accepted"));
    send.__seed_next_work_sequence_for_test(u64::MAX);
    send.pump(PumpBudget::new(0, 1, 0, 0));
    assert_eq!(control.mapped.load(Ordering::Relaxed), 1);
    assert!(send.trace().records().any(|record| {
        matches!(record.kind(), TraceRecordKind::ActionSubmissionAccepted)
            && record
                .work_sequence()
                .is_some_and(|sequence| sequence.get() == u64::MAX)
    }));
    assert_eq!(send.status(), runenui_runtime::RuntimeStatus::Running);
}
