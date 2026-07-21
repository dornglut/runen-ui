#![allow(refining_impl_trait)]

use std::{
    cell::Cell,
    future::Future,
    pin::Pin,
    rc::Rc,
    sync::{
        Arc, Barrier, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    task::{Context, Poll, Waker},
};

use runenui_core::{
    CommandOrigin, Effects, Element, IntoEffects, NoHostProtocol, SemanticCommand,
    SendSubscriptionSink, SendSubscriptionSinkError, SendSubscriptionSource,
    SendSubscriptionStartOutcome, SubscriptionSet, UiApp, View, Widget, WidgetActivation,
    WidgetActivationContext, WidgetActivationOutput, WidgetMountContext, WidgetUnmountContext,
    WorkFamily, WorkKey, column, text,
};
use runenui_runtime::{
    AppRuntime, PumpBudget, RuntimeConfig, SendTaskCompletionError, SendTaskExecutor, SendTaskJob,
    SendTaskStartError, TraceConfig, TraceRecordKind, TraceWorkFamily, TraceWorkOwner,
};

#[derive(Clone, Copy, Debug)]
enum Action {
    CancelThenStart,
    StartThenCancel,
    StartThenReplace,
    CancelTwice,
}

struct State {
    old: Rc<Cell<usize>>,
    first: Rc<Cell<usize>>,
    second: Rc<Cell<usize>>,
}

struct App;

impl UiApp for App {
    type State = State;
    type Action = Action;
    type HostProtocol = NoHostProtocol;

    fn root(_: &Self::State) -> impl View<Self::Action> {
        text("transactional cancellation")
    }

    fn initial_effects(state: &Self::State) -> impl IntoEffects<Self::Action, Self::HostProtocol> {
        Effects::keyed_local_task(key(), PollProbe(Rc::clone(&state.old)))
    }

    fn update(
        state: &mut Self::State,
        action: Self::Action,
    ) -> impl IntoEffects<Self::Action, Self::HostProtocol> {
        match action {
            Action::CancelThenStart => Effects::cancel(WorkFamily::LocalTask, key()).then(
                Effects::keyed_local_task(key(), PollProbe(Rc::clone(&state.first))),
            ),
            Action::StartThenCancel => {
                Effects::keyed_local_task(key(), PollProbe(Rc::clone(&state.first)))
                    .then(Effects::cancel(WorkFamily::LocalTask, key()))
            }
            Action::StartThenReplace => {
                Effects::keyed_local_task(key(), PollProbe(Rc::clone(&state.first))).then(
                    Effects::keyed_local_task(key(), PollProbe(Rc::clone(&state.second))),
                )
            }
            Action::CancelTwice => Effects::cancel(WorkFamily::LocalTask, key())
                .then(Effects::cancel(WorkFamily::LocalTask, key())),
        }
    }
}

struct PollProbe(Rc<Cell<usize>>);

impl Future for PollProbe {
    type Output = Option<Action>;

    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        self.0.set(self.0.get() + 1);
        Poll::Pending
    }
}

fn key() -> WorkKey {
    WorkKey::new("same.batch").unwrap_or_else(|_| unreachable!())
}

struct RunResult {
    polls: (usize, usize, usize),
    requested: Vec<u64>,
    cancelled: Vec<u64>,
    semantic: Vec<(&'static str, u64)>,
    provisional_cancellation_parent_is_commit: bool,
}

fn run(action: Action) -> RunResult {
    let old = Rc::new(Cell::new(0));
    let first = Rc::new(Cell::new(0));
    let second = Rc::new(Cell::new(0));
    let mut runtime = AppRuntime::<App>::mount(State {
        old: Rc::clone(&old),
        first: Rc::clone(&first),
        second: Rc::clone(&second),
    });
    runtime.pump(PumpBudget::new(2, 0, 1, 0));
    old.set(0);
    runtime
        .submit_action(action)
        .unwrap_or_else(|_| unreachable!());
    runtime.pump(PumpBudget::new(1, 0, 1, 0));
    runtime.pump(PumpBudget::new(16, 0, 16, 0));
    let trace_generations = |kind: fn(&TraceRecordKind) -> bool| {
        runtime
            .trace()
            .records()
            .filter(|record| kind(record.kind()))
            .map(|record| {
                let identity = record.work().unwrap_or_else(|| unreachable!());
                assert_eq!(identity.owner(), &TraceWorkOwner::Application);
                assert_eq!(identity.family(), TraceWorkFamily::LocalTask);
                assert_eq!(identity.key(), Some(&key()));
                identity.generation()
            })
            .collect()
    };
    let records: Vec<_> = runtime.trace().records().collect();
    let semantic = records
        .iter()
        .filter_map(|record| {
            let label = match record.kind() {
                TraceRecordKind::WorkRequested => "requested",
                TraceRecordKind::WorkGenerationCommitted => "committed",
                TraceRecordKind::WorkCancellationBound => "bound",
                TraceRecordKind::WorkLogicallyInvalidated => "invalidated",
                _ => return None,
            };
            Some((
                label,
                record
                    .work()
                    .unwrap_or_else(|| unreachable!("work fact has identity"))
                    .generation(),
            ))
        })
        .collect();
    let provisional_cancellation_parent_is_commit = records.iter().all(|bound| {
        if !matches!(bound.kind(), TraceRecordKind::WorkCancellationBound)
            || bound
                .work()
                .is_none_or(|identity| identity.generation() == 1)
        {
            return true;
        }
        let generation = bound.work().unwrap_or_else(|| unreachable!()).generation();
        records.iter().any(|committed| {
            matches!(committed.kind(), TraceRecordKind::WorkGenerationCommitted)
                && committed
                    .work()
                    .is_some_and(|identity| identity.generation() == generation)
                && bound.causal_parent() == Some(committed.sequence())
        })
    });
    RunResult {
        polls: (old.get(), first.get(), second.get()),
        requested: trace_generations(|kind| matches!(kind, TraceRecordKind::WorkRequested)),
        cancelled: trace_generations(|kind| matches!(kind, TraceRecordKind::WorkCancellationBound)),
        semantic,
        provisional_cancellation_parent_is_commit,
    }
}

#[test]
fn all_same_batch_keyed_cases_bind_and_invalidate_at_commit() {
    let cancel_then_start = run(Action::CancelThenStart);
    assert_eq!(cancel_then_start.polls, (0, 1, 0));
    assert_eq!(cancel_then_start.requested, [1, 2]);
    assert_eq!(cancel_then_start.cancelled, [1]);
    assert_eq!(
        &cancel_then_start.semantic[2..],
        [
            ("bound", 1),
            ("invalidated", 1),
            ("requested", 2),
            ("committed", 2),
        ]
    );

    let start_then_cancel = run(Action::StartThenCancel);
    assert_eq!(start_then_cancel.polls, (0, 0, 0));
    assert_eq!(start_then_cancel.requested, [1, 2]);
    assert_eq!(start_then_cancel.cancelled, [1, 2]);
    assert_eq!(
        &start_then_cancel.semantic[2..],
        [
            ("bound", 1),
            ("invalidated", 1),
            ("requested", 2),
            ("committed", 2),
            ("bound", 2),
            ("invalidated", 2),
        ]
    );
    assert!(start_then_cancel.provisional_cancellation_parent_is_commit);

    let start_then_replace = run(Action::StartThenReplace);
    assert_eq!(start_then_replace.polls, (0, 0, 1));
    assert_eq!(start_then_replace.requested, [1, 2, 3]);
    assert_eq!(start_then_replace.cancelled, [1, 2]);
    assert_eq!(
        &start_then_replace.semantic[2..],
        [
            ("bound", 1),
            ("invalidated", 1),
            ("requested", 2),
            ("committed", 2),
            ("bound", 2),
            ("invalidated", 2),
            ("requested", 3),
            ("committed", 3),
        ]
    );
    assert!(start_then_replace.provisional_cancellation_parent_is_commit);

    let cancel_twice = run(Action::CancelTwice);
    assert_eq!(cancel_twice.polls, (0, 0, 0));
    assert_eq!(cancel_twice.requested, [1]);
    assert_eq!(cancel_twice.cancelled, [1]);
    assert_eq!(
        &cancel_twice.semantic[2..],
        [("bound", 1), ("invalidated", 1)]
    );
}

#[derive(Debug)]
struct MountedCancellationWidget {
    mode: Action,
    old: Rc<Cell<usize>>,
    first: Rc<Cell<usize>>,
    second: Rc<Cell<usize>>,
}

impl Widget<()> for MountedCancellationWidget {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn mount(&self, (): &mut Self::State, context: &mut WidgetMountContext<()>) {
        context.keyed_local_task(key(), MountedPollProbe(Rc::clone(&self.old)));
    }

    fn activation(&self, (): &Self::State) -> WidgetActivation {
        WidgetActivation::actionable(true)
    }

    fn activate(
        &mut self,
        (): &mut Self::State,
        context: &mut WidgetActivationContext<()>,
    ) -> WidgetActivationOutput<()> {
        match self.mode {
            Action::CancelThenStart => {
                context.cancel(WorkFamily::LocalTask, key());
                context.keyed_local_task(key(), MountedPollProbe(Rc::clone(&self.first)));
            }
            Action::StartThenCancel => {
                context.keyed_local_task(key(), MountedPollProbe(Rc::clone(&self.first)));
                context.cancel(WorkFamily::LocalTask, key());
            }
            Action::StartThenReplace => {
                context.keyed_local_task(key(), MountedPollProbe(Rc::clone(&self.first)));
                context.keyed_local_task(key(), MountedPollProbe(Rc::clone(&self.second)));
            }
            Action::CancelTwice => {
                context.cancel(WorkFamily::LocalTask, key());
                context.cancel(WorkFamily::LocalTask, key());
            }
        }
        WidgetActivationOutput::none()
    }
}

struct MountedPollProbe(Rc<Cell<usize>>);

impl Future for MountedPollProbe {
    type Output = Option<()>;

    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        self.0.set(self.0.get() + 1);
        Poll::Pending
    }
}

struct MountedCancellationApp;

impl UiApp for MountedCancellationApp {
    type State = MountedCancellationWidget;
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> Element<Self::Action> {
        Element::new(MountedCancellationWidget {
            mode: state.mode,
            old: Rc::clone(&state.old),
            first: Rc::clone(&state.first),
            second: Rc::clone(&state.second),
        })
        .key("mounted.same.batch")
    }

    fn update(_: &mut Self::State, (): Self::Action) {}
}

fn run_mounted(action: Action) -> (Vec<(&'static str, u64)>, Vec<u64>, u64) {
    let old = Rc::new(Cell::new(0));
    let first = Rc::new(Cell::new(0));
    let second = Rc::new(Cell::new(0));
    let mut runtime = AppRuntime::<MountedCancellationApp>::mount(MountedCancellationWidget {
        mode: action,
        old: Rc::clone(&old),
        first,
        second,
    });
    runtime.pump(PumpBudget::new(2, 0, 1, 0));
    old.set(0);
    let target = runtime.index().nodes()[0].id().clone();
    runtime
        .submit_command(
            target,
            SemanticCommand::Activate,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("the exact live target is accepted"));
    runtime.pump(PumpBudget::new(1, 0, 0, 0));
    let routed_start_sequence = runtime
        .trace()
        .records()
        .filter(|record| matches!(record.kind(), TraceRecordKind::RoutedEventStarted))
        .last()
        .map_or_else(
            || unreachable!("routed transaction start is retained"),
            |record| record.sequence().get(),
        );
    runtime.pump(PumpBudget::new(16, 0, 16, 0));
    let semantic = runtime
        .trace()
        .records()
        .filter(|record| record.sequence().get() > routed_start_sequence)
        .filter_map(|record| {
            let label = match record.kind() {
                TraceRecordKind::WorkRequested => "requested",
                TraceRecordKind::WorkGenerationCommitted => "committed",
                TraceRecordKind::WorkCancellationBound => "bound",
                TraceRecordKind::WorkLogicallyInvalidated => "invalidated",
                _ => return None,
            };
            let identity = record
                .work()
                .unwrap_or_else(|| unreachable!("semantic work fact has identity"));
            assert!(matches!(identity.owner(), TraceWorkOwner::Mounted(_)));
            Some((label, identity.generation()))
        })
        .collect();
    let cleanup_sequences: Vec<u64> = runtime
        .trace()
        .records()
        .filter(|record| matches!(record.kind(), TraceRecordKind::WorkCleanupProcessed))
        .filter_map(runenui_runtime::TraceRecord::work_sequence)
        .map(runenui_runtime::WorkSequence::get)
        .collect();
    assert_eq!(old.get(), 0);
    let first = cleanup_sequences
        .first()
        .copied()
        .unwrap_or_else(|| unreachable!("cancellation queues cleanup"));
    (semantic, cleanup_sequences, first)
}

#[test]
fn mounted_same_batch_semantics_preserve_collector_order_and_cleanup_sequences() {
    let (semantic, cleanup, first) = run_mounted(Action::CancelThenStart);
    assert_eq!(
        semantic,
        [
            ("bound", 1),
            ("invalidated", 1),
            ("requested", 2),
            ("committed", 2),
        ]
    );
    assert_eq!(cleanup, [first]);

    let (semantic, cleanup, first) = run_mounted(Action::StartThenCancel);
    assert_eq!(
        semantic,
        [
            ("bound", 1),
            ("invalidated", 1),
            ("requested", 2),
            ("committed", 2),
            ("bound", 2),
            ("invalidated", 2),
        ]
    );
    assert_eq!(cleanup, [first, first + 1]);

    let (semantic, cleanup, first) = run_mounted(Action::StartThenReplace);
    assert_eq!(
        semantic,
        [
            ("bound", 1),
            ("invalidated", 1),
            ("requested", 2),
            ("committed", 2),
            ("bound", 2),
            ("invalidated", 2),
            ("requested", 3),
            ("committed", 3),
        ]
    );
    assert_eq!(cleanup, [first, first + 1]);

    let (semantic, cleanup, first) = run_mounted(Action::CancelTwice);
    assert_eq!(semantic, [("bound", 1), ("invalidated", 1)]);
    assert_eq!(cleanup, [first]);
}

#[test]
fn trace_retention_eviction_does_not_change_live_cancellation_authority() {
    let old = Rc::new(Cell::new(0));
    let first = Rc::new(Cell::new(0));
    let second = Rc::new(Cell::new(0));
    let config = RuntimeConfig::default().with_trace_config(TraceConfig::new(2));
    let mut runtime = AppRuntime::<App>::mount_with_config(
        State {
            old: Rc::clone(&old),
            first,
            second,
        },
        config,
    );
    runtime.pump(PumpBudget::new(2, 0, 1, 0));
    old.set(0);
    runtime
        .submit_action(Action::CancelTwice)
        .unwrap_or_else(|_| unreachable!());
    runtime.pump(PumpBudget::new(16, 0, 16, 0));

    assert_eq!(old.get(), 0);
    assert!(runtime.trace().dropped_before_sequence().is_some());
}

#[derive(Clone, Copy, Debug)]
enum UnmountAction {
    Remove,
    Item,
}

struct BarrierWidget {
    barrier: Arc<Barrier>,
    sink: Arc<Mutex<Option<SendSubscriptionSink<Arc<u8>>>>>,
    mapped: Arc<AtomicUsize>,
}

impl core::fmt::Debug for BarrierWidget {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter.write_str("BarrierWidget")
    }
}

struct RetainedSinkSource {
    sink: Arc<Mutex<Option<SendSubscriptionSink<Arc<u8>>>>>,
}

impl SendSubscriptionSource<Arc<u8>> for RetainedSinkSource {
    fn start(self: Box<Self>, sink: SendSubscriptionSink<Arc<u8>>) -> SendSubscriptionStartOutcome {
        *self
            .sink
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(sink);
        SendSubscriptionStartOutcome::Started
    }
}

impl Widget<UnmountAction> for BarrierWidget {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn subscriptions(&self, (): &Self::State, subscriptions: &mut SubscriptionSet<UnmountAction>) {
        let mapped = Arc::clone(&self.mapped);
        subscriptions.send(
            WorkKey::new("unmount.barrier").unwrap_or_else(|_| unreachable!()),
            0,
            RetainedSinkSource {
                sink: Arc::clone(&self.sink),
            },
            move |_| {
                mapped.fetch_add(1, Ordering::SeqCst);
                UnmountAction::Item
            },
        );
    }

    fn unmount(&self, (): &mut Self::State, _: &mut WidgetUnmountContext) {
        self.barrier.wait();
        self.barrier.wait();
    }
}

struct UnmountState {
    mounted: bool,
    replace: bool,
    barrier: Arc<Barrier>,
    sink: Arc<Mutex<Option<SendSubscriptionSink<Arc<u8>>>>>,
    mapped: Arc<AtomicUsize>,
}

struct UnmountApp;

impl UiApp for UnmountApp {
    type State = UnmountState;
    type Action = UnmountAction;
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> Element<Self::Action> {
        let children = if state.mounted {
            vec![
                Element::new(BarrierWidget {
                    barrier: Arc::clone(&state.barrier),
                    sink: Arc::clone(&state.sink),
                    mapped: Arc::clone(&state.mapped),
                })
                .key("barrier"),
            ]
        } else if state.replace {
            vec![text("replacement").key("barrier").into_element()]
        } else {
            Vec::new()
        };
        column(children).key("unmount.root").into_element()
    }

    fn update(state: &mut Self::State, action: Self::Action) {
        if matches!(action, UnmountAction::Remove) {
            state.mounted = false;
        }
    }
}

#[cfg(feature = "internal-test-seams")]
fn assert_mounted_subscription_authority_is_stale_before_unmount(replace: bool) {
    let barrier = Arc::new(Barrier::new(2));
    let sink = Arc::new(Mutex::new(None));
    let mapped = Arc::new(AtomicUsize::new(0));
    let mut runtime = AppRuntime::<UnmountApp>::mount(UnmountState {
        mounted: true,
        replace,
        barrier: Arc::clone(&barrier),
        sink: Arc::clone(&sink),
        mapped: Arc::clone(&mapped),
    });
    runtime.pump(PumpBudget::new(16, usize::MAX, usize::MAX, usize::MAX));
    let retained = sink
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .as_ref()
        .cloned()
        .unwrap_or_else(|| unreachable!("mounted source retained its sink"));
    let item = Arc::new(7_u8);
    let submitted = Arc::clone(&item);
    let worker_barrier = Arc::clone(&barrier);
    let worker = std::thread::spawn(move || {
        worker_barrier.wait();
        let result = retained.try_send(submitted);
        worker_barrier.wait();
        result
    });
    runtime
        .submit_action(UnmountAction::Remove)
        .unwrap_or_else(|_| unreachable!("removal action is accepted"));
    runtime.pump(PumpBudget::new(16, usize::MAX, usize::MAX, usize::MAX));
    let Err(SendSubscriptionSinkError::Stale(recovered)) = worker
        .join()
        .unwrap_or_else(|_| unreachable!("producer thread remains deterministic"))
    else {
        unreachable!("authority is stale before the unmount hook")
    };
    assert!(Arc::ptr_eq(&recovered, &item));
    assert_eq!(mapped.load(Ordering::SeqCst), 0);
    assert_eq!(runtime.__live_work_record_count_for_test(), 0);
    assert_eq!(runtime.__subscription_slot_count_for_test(), 0);
    assert_eq!(runtime.__completion_payload_count_for_test(), 0);
}

#[cfg(feature = "internal-test-seams")]
#[test]
fn mounted_subscription_authority_is_stale_before_removal_unmount_callback_runs() {
    assert_mounted_subscription_authority_is_stale_before_unmount(false);
}

#[cfg(feature = "internal-test-seams")]
#[test]
fn mounted_subscription_authority_is_stale_before_keyed_replacement_unmount_callback_runs() {
    assert_mounted_subscription_authority_is_stale_before_unmount(true);
}

#[derive(Debug)]
struct TaskBarrierWidget {
    barrier: Arc<Barrier>,
    mapped: Arc<AtomicUsize>,
}

impl Widget<UnmountAction> for TaskBarrierWidget {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn mount(&self, (): &mut Self::State, context: &mut WidgetMountContext<UnmountAction>) {
        let mapped = Arc::clone(&self.mapped);
        context.send_task(async { 7_u8 }, move |_| {
            mapped.fetch_add(1, Ordering::SeqCst);
            UnmountAction::Item
        });
    }

    fn unmount(&self, (): &mut Self::State, _: &mut WidgetUnmountContext) {
        self.barrier.wait();
        self.barrier.wait();
    }
}

struct TaskUnmountState {
    mounted: bool,
    barrier: Arc<Barrier>,
    mapped: Arc<AtomicUsize>,
}

struct TaskUnmountApp;

impl UiApp for TaskUnmountApp {
    type State = TaskUnmountState;
    type Action = UnmountAction;
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> Element<Self::Action> {
        if state.mounted {
            Element::new(TaskBarrierWidget {
                barrier: Arc::clone(&state.barrier),
                mapped: Arc::clone(&state.mapped),
            })
            .key("task.barrier")
        } else {
            text("removed").into_element()
        }
    }

    fn update(state: &mut Self::State, action: Self::Action) {
        if matches!(action, UnmountAction::Remove) {
            state.mounted = false;
        }
    }
}

#[derive(Clone)]
struct RetainingSendTaskExecutor(Arc<Mutex<Vec<SendTaskJob>>>);

impl SendTaskExecutor for RetainingSendTaskExecutor {
    fn start(&mut self, job: SendTaskJob) -> Result<(), SendTaskStartError> {
        self.0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push(job);
        Ok(())
    }
}

fn run_ready_send_task(job: SendTaskJob) -> Result<(), SendTaskCompletionError> {
    let mut future = Box::pin(job.run());
    let mut context = Context::from_waker(Waker::noop());
    match Pin::new(&mut future).poll(&mut context) {
        Poll::Ready(result) => result,
        Poll::Pending => unreachable!("test send task is immediately ready"),
    }
}

#[cfg(feature = "internal-test-seams")]
#[test]
fn mounted_send_task_completion_is_stale_during_unmount_callback() {
    let barrier = Arc::new(Barrier::new(2));
    let mapped = Arc::new(AtomicUsize::new(0));
    let jobs = Arc::new(Mutex::new(Vec::new()));
    let mut runtime = AppRuntime::<TaskUnmountApp>::mount(TaskUnmountState {
        mounted: true,
        barrier: Arc::clone(&barrier),
        mapped: Arc::clone(&mapped),
    });
    runtime.set_send_task_executor(RetainingSendTaskExecutor(Arc::clone(&jobs)));
    runtime.pump(PumpBudget::new(16, usize::MAX, usize::MAX, usize::MAX));
    let job = jobs
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .pop()
        .unwrap_or_else(|| unreachable!("mounted send task was accepted exactly once"));
    let worker_barrier = Arc::clone(&barrier);
    let worker = std::thread::spawn(move || {
        worker_barrier.wait();
        let result = run_ready_send_task(job);
        worker_barrier.wait();
        result
    });
    runtime
        .submit_action(UnmountAction::Remove)
        .unwrap_or_else(|_| unreachable!("removal action is accepted"));
    runtime.pump(PumpBudget::new(16, usize::MAX, usize::MAX, usize::MAX));
    assert!(matches!(
        worker
            .join()
            .unwrap_or_else(|_| unreachable!("producer thread remains deterministic")),
        Err(SendTaskCompletionError::Stale(_))
    ));
    assert_eq!(mapped.load(Ordering::SeqCst), 0);
    assert_eq!(runtime.__live_work_record_count_for_test(), 0);
    assert_eq!(runtime.__send_task_slot_count_for_test(), 0);
    assert_eq!(runtime.__send_task_mapper_count_for_test(), 0);
    assert_eq!(runtime.__completion_payload_count_for_test(), 0);
}
