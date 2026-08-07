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
    AppRuntime, PumpBudget, RuntimeConfig, RuntimeLimits, RuntimeStatus, RuntimeTerminalReason,
    SendTaskCompletionError, SendTaskExecutor, SendTaskJob, SendTaskStartError,
    SendTaskStartOutcome, TimerFiringOutcome, TimerStartOutcome, TraceConfig, TraceRecordKind,
    TraceTimerTerminalOutcome, TraceWorkFamily, TraceWorkStartRefusal,
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
    ) -> impl IntoEffects<Self::Action, Self::HostProtocol> {
        if let Action::Label(label) = action {
            state.push(label);
        }
    }
}

#[test]
fn ready_local_task_reaches_update_only_through_canonical_envelopes() {
    let mut runtime = AppRuntime::<LocalTaskApp>::mount(Vec::new());
    let report = runtime.pump(PumpBudget::new(8, usize::MAX, usize::MAX, usize::MAX));
    assert_eq!(runtime.state(), &["ready"]);
    assert_eq!(report.processed_envelopes(), 3);
    assert!(report.is_quiescent());
}

struct PendingTaskApp;

impl UiApp for PendingTaskApp {
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
    ) -> impl IntoEffects<Self::Action, Self::HostProtocol> {
    }
}

#[test]
fn pending_local_task_does_not_prevent_quiescence() {
    let mut runtime = AppRuntime::<PendingTaskApp>::mount(());
    let report = runtime.pump(PumpBudget::new(8, usize::MAX, usize::MAX, usize::MAX));
    assert_eq!(report.processed_envelopes(), 2);
    assert!(report.is_quiescent());
}

struct TimerApp;

impl UiApp for TimerApp {
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
    ) -> impl IntoEffects<Self::Action, Self::HostProtocol> {
        if let Action::Label(label) = action {
            state.push(label);
        }
    }
}

#[test]
fn equal_deadline_timers_fire_in_creation_order() {
    let mut runtime = AppRuntime::<TimerApp>::mount(Vec::new());
    assert!(
        runtime
            .pump(PumpBudget::new(3, usize::MAX, usize::MAX, usize::MAX))
            .is_quiescent()
    );
    runtime
        .advance_time(Duration::from_millis(5))
        .unwrap_or_else(|_| unreachable!());
    assert!(
        runtime
            .pump(PumpBudget::new(8, usize::MAX, usize::MAX, usize::MAX))
            .is_quiescent()
    );
    assert_eq!(runtime.state(), &["first", "second"]);
}

struct RepeatingTimerApp;

impl UiApp for RepeatingTimerApp {
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
    ) -> impl IntoEffects<Self::Action, Self::HostProtocol> {
        *state += 1;
    }
}

#[test]
fn repeating_timer_coalesces_missed_ticks_from_logical_deadline() {
    let mut runtime = AppRuntime::<RepeatingTimerApp>::mount(0);
    runtime.pump(PumpBudget::new(2, usize::MAX, usize::MAX, usize::MAX));
    runtime
        .advance_time(Duration::from_millis(35))
        .unwrap_or_else(|_| unreachable!());
    runtime.pump(PumpBudget::new(8, usize::MAX, usize::MAX, usize::MAX));
    assert_eq!(*runtime.state(), 1);
    runtime
        .advance_time(Duration::from_millis(4))
        .unwrap_or_else(|_| unreachable!());
    assert_eq!(
        runtime
            .pump(PumpBudget::new(8, usize::MAX, usize::MAX, usize::MAX))
            .processed_envelopes(),
        0
    );
    runtime
        .advance_time(Duration::from_millis(1))
        .unwrap_or_else(|_| unreachable!());
    runtime.pump(PumpBudget::new(8, usize::MAX, usize::MAX, usize::MAX));
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
    ) -> impl IntoEffects<Self::Action, Self::HostProtocol> {
        *state += 1;
    }
}

#[test]
fn repeating_deadline_overflow_terminates_only_the_timer_after_current_firing() {
    let mut runtime = AppRuntime::<OverflowingRepeatingTimerApp>::mount(0);
    runtime.pump(PumpBudget::new(2, usize::MAX, usize::MAX, usize::MAX));
    runtime
        .advance_time(Duration::from_nanos(u64::MAX))
        .unwrap_or_else(|_| unreachable!());

    let report = runtime.pump(PumpBudget::new(8, usize::MAX, usize::MAX, usize::MAX));

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
            .processed_envelopes(),
        0
    );
}

struct ZeroTimerApp;

impl UiApp for ZeroTimerApp {
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
    ) -> impl IntoEffects<Self::Action, Self::HostProtocol> {
    }
}

#[test]
fn zero_repeating_interval_is_explicit_and_non_poisoning() {
    let mut runtime = AppRuntime::<ZeroTimerApp>::mount(());
    assert!(
        runtime
            .pump(PumpBudget::new(2, usize::MAX, usize::MAX, usize::MAX))
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
    ) -> impl IntoEffects<Self::Action, Self::HostProtocol> {
        if let Action::NonSend(value) = action {
            *state += Rc::strong_count(&value);
        }
    }
}

#[test]
fn send_task_transports_output_without_requiring_action_send() {
    let jobs = Rc::new(RefCell::new(Vec::new()));
    let mut runtime = AppRuntime::<SendTaskApp>::mount(0);
    runtime.set_send_task_executor(HoldingExecutor(Rc::clone(&jobs)));
    runtime.pump(PumpBudget::new(2, usize::MAX, usize::MAX, usize::MAX));
    assert_eq!(
        runtime.last_send_task_start_outcome(),
        Some(SendTaskStartOutcome::Started)
    );
    let job = jobs.borrow_mut().pop().unwrap_or_else(|| unreachable!());
    run_ready(job).unwrap_or_else(|_| unreachable!());
    runtime.pump(PumpBudget::new(8, usize::MAX, usize::MAX, usize::MAX));
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
    ) -> impl IntoEffects<Self::Action, Self::HostProtocol> {
        *state = Some(action);
    }
}

#[test]
fn executor_refusal_is_terminal_for_work_but_recoverable_for_runtime() {
    let mut runtime = AppRuntime::<RefusalApp>::mount(None);
    assert!(
        runtime
            .pump(PumpBudget::new(8, usize::MAX, usize::MAX, usize::MAX))
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
    let mut runtime = AppRuntime::<RefusalApp>::mount(None);
    runtime.set_send_task_executor(RefusingExecutor {
        kind,
        calls: Rc::clone(&calls),
    });

    assert!(
        runtime
            .pump(PumpBudget::new(8, usize::MAX, usize::MAX, usize::MAX))
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
    );
    runtime.set_send_task_executor(HoldingExecutor(Rc::clone(&jobs)));
    runtime.pump(PumpBudget::new(2, usize::MAX, usize::MAX, usize::MAX));
    let job = jobs.borrow_mut().pop().unwrap_or_else(|| unreachable!());
    let Err(SendTaskCompletionError::Full(completion)) = run_ready(job) else {
        unreachable!()
    };
    runtime.shutdown();
    assert!(matches!(
        completion.submit(),
        Err(SendTaskCompletionError::Closed(_))
    ));
}

#[cfg(feature = "internal-test-seams")]
struct SendMapperIntegrityApp;

#[cfg(feature = "internal-test-seams")]
impl UiApp for SendMapperIntegrityApp {
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
    ) -> impl IntoEffects<Self::Action, Self::HostProtocol> {
    }
}

#[cfg(feature = "internal-test-seams")]
#[test]
fn work_sequence_exhaustion_prevents_send_completion_mapper() {
    let calls = Rc::new(Cell::new(0));
    let jobs = Rc::new(RefCell::new(Vec::new()));
    let mut runtime = AppRuntime::<SendMapperIntegrityApp>::mount(Rc::clone(&calls));
    runtime.set_send_task_executor(HoldingExecutor(Rc::clone(&jobs)));
    runtime.pump(PumpBudget::new(2, 0, 0, 0));
    run_ready(jobs.borrow_mut().pop().unwrap_or_else(|| unreachable!()))
        .unwrap_or_else(|_| unreachable!());
    runtime.__seed_next_work_sequence_for_test(0);

    runtime.pump(PumpBudget::new(0, 1, 0, 0));

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
fn one_remaining_sequence_is_consumed_only_by_each_final_scheduler_action() {
    let mut local = AppRuntime::<LocalTaskApp>::mount(Vec::new());
    local.pump(PumpBudget::new(2, 0, 0, 0));
    local.__seed_next_work_sequence_for_test(u64::MAX);
    local.pump(PumpBudget::new(0, 0, 1, 0));
    assert_eq!(accepted_last_sequence(&local), 1);
    assert_eq!(local.status(), RuntimeStatus::Running);
    assert!(local.state().is_empty());

    let jobs = Rc::new(RefCell::new(Vec::new()));
    let mut send = AppRuntime::<SendTaskApp>::mount(0);
    send.set_send_task_executor(HoldingExecutor(Rc::clone(&jobs)));
    send.pump(PumpBudget::new(2, 0, 0, 0));
    run_ready(jobs.borrow_mut().pop().unwrap_or_else(|| unreachable!()))
        .unwrap_or_else(|_| unreachable!());
    send.__seed_next_work_sequence_for_test(u64::MAX);
    send.pump(PumpBudget::new(0, 1, 0, 0));
    assert_eq!(accepted_last_sequence(&send), 1);
    assert_eq!(send.status(), RuntimeStatus::Running);
    assert_eq!(send.state(), &0);

    let mut timer = AppRuntime::<RepeatingTimerApp>::mount(0);
    timer.pump(PumpBudget::new(2, 0, 0, 0));
    timer
        .advance_time(Duration::from_millis(10))
        .unwrap_or_else(|_| unreachable!());
    timer.pump(PumpBudget::new(0, 0, 0, 1));
    timer.__seed_next_work_sequence_for_test(u64::MAX);
    timer.pump(PumpBudget::new(1, 0, 0, 0));
    assert_eq!(accepted_last_sequence(&timer), 1);
    assert_eq!(timer.status(), RuntimeStatus::Running);
    assert_eq!(timer.state(), &0);

    let refusal_calls = Rc::new(Cell::new(0));
    let mut refusal = AppRuntime::<RefusalApp>::mount(None);
    refusal.set_send_task_executor(RefusingExecutor {
        kind: RefusalKind::Unavailable,
        calls: Rc::clone(&refusal_calls),
    });
    refusal.pump(PumpBudget::new(1, 0, 0, 0));
    refusal.__seed_next_work_sequence_for_test(u64::MAX);
    refusal.pump(PumpBudget::new(1, 0, 0, 0));
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
    ) -> impl IntoEffects<Self::Action, Self::HostProtocol> {
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
    });
    runtime.set_send_task_executor(HoldingExecutor(Rc::clone(&jobs)));
    runtime.pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX));
    runtime
        .submit_action(false)
        .unwrap_or_else(|_| unreachable!());
    runtime.pump(PumpBudget::new(2, usize::MAX, usize::MAX, usize::MAX));

    let job = jobs.borrow_mut().pop().unwrap_or_else(|| unreachable!());
    assert!(matches!(
        run_ready(job),
        Err(SendTaskCompletionError::Stale(_))
    ));
    assert!(
        runtime
            .pump(PumpBudget::new(8, usize::MAX, usize::MAX, usize::MAX))
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
    });
    runtime.set_send_task_executor(HoldingExecutor(Rc::clone(&jobs)));
    runtime.pump(PumpBudget::new(8, usize::MAX, usize::MAX, usize::MAX));
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
    runtime.pump(PumpBudget::new(0, 1, 0, 0));

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
    runtime.pump(PumpBudget::new(0, 1, 0, 0));

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
fn disabled_trace_changes_no_send_completion_behavior() {
    let mapper_calls = Rc::new(Cell::new(0));
    let jobs = Rc::new(RefCell::new(Vec::new()));
    let mut runtime = AppRuntime::<SendCancelApp>::mount_with_config(
        SendCancelState {
            mapper_calls: Rc::clone(&mapper_calls),
            completed_updates: 0,
        },
        RuntimeConfig::default().with_trace_config(TraceConfig::new(0)),
    );
    runtime.set_send_task_executor(HoldingExecutor(Rc::clone(&jobs)));
    runtime.pump(PumpBudget::new(8, usize::MAX, usize::MAX, usize::MAX));
    let job = jobs
        .borrow_mut()
        .pop()
        .unwrap_or_else(|| unreachable!("executor retained one exact job"));
    run_ready(job).unwrap_or_else(|_| unreachable!("live completion enters ingress"));
    runtime.pump(PumpBudget::new(0, 1, 0, 0));

    assert_eq!(mapper_calls.get(), 1);
    assert_eq!(runtime.status(), RuntimeStatus::Running);
    assert!(runtime.trace().is_empty());
}

struct CancelTimerApp;

impl UiApp for CancelTimerApp {
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
    ) -> impl IntoEffects<Self::Action, Self::HostProtocol> {
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
    let mut runtime = AppRuntime::<CancelTimerApp>::mount(0);
    runtime.pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX));
    runtime
        .submit_action(false)
        .unwrap_or_else(|_| unreachable!());
    runtime.pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX));
    runtime
        .advance_time(Duration::from_millis(5))
        .unwrap_or_else(|_| unreachable!());
    runtime.pump(PumpBudget::new(8, usize::MAX, usize::MAX, usize::MAX));
    assert_eq!(*runtime.state(), 0);
}
