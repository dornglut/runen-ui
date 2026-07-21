#![allow(refining_impl_trait)]

use std::{
    future::Future,
    pin::Pin,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
    task::{Context, Poll, Waker},
};

use runenui_core::{Effects, IntoEffects, NoHostProtocol, UiApp, View, text};
use runenui_runtime::{
    AppRuntime, PumpBudget, SendTaskCompletionError, SendTaskExecutor, SendTaskJob,
    SendTaskStartError,
};

struct App;

impl UiApp for App {
    type State = usize;
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root(_: &Self::State) -> impl View<Self::Action> {
        text("wake redraw")
    }
    fn initial_effects(_: &Self::State) -> impl IntoEffects<Self::Action, Self::HostProtocol> {
        Effects::send_task(async { 1_u8 }, |_| ())
    }
    fn update(
        state: &mut Self::State,
        (): Self::Action,
    ) -> impl IntoEffects<Self::Action, Self::HostProtocol> {
        *state += 1;
        Effects::redraw()
    }
}

#[derive(Clone)]
struct HoldingExecutor(Arc<Mutex<Vec<SendTaskJob>>>);

impl SendTaskExecutor for HoldingExecutor {
    fn start(&mut self, job: SendTaskJob) -> Result<(), SendTaskStartError> {
        self.0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push(job);
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

#[test]
fn pump_acknowledgment_and_rearm_do_not_strand_work() {
    let jobs = Arc::new(Mutex::new(Vec::new()));
    let wake_count = Arc::new(AtomicUsize::new(0));
    let mut runtime = AppRuntime::<App>::mount(0);
    runtime.set_send_task_executor(HoldingExecutor(Arc::clone(&jobs)));
    let count = Arc::clone(&wake_count);
    runtime.set_wake_transport(move || {
        count.fetch_add(1, Ordering::SeqCst);
    });
    assert_eq!(wake_count.load(Ordering::SeqCst), 1);
    runtime.pump(PumpBudget::new(2, 0, 0, 0));
    assert_eq!(wake_count.load(Ordering::SeqCst), 1);

    let job = jobs
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .pop()
        .unwrap_or_else(|| unreachable!());
    run_ready(job).unwrap_or_else(|_| unreachable!());
    assert_eq!(wake_count.load(Ordering::SeqCst), 2);
    let blocked = runtime.pump(PumpBudget::new(0, 0, 0, 0));
    assert!(blocked.completion_imports_pending());
    assert!(blocked.exhausted_budgets().completion_imports());
    assert_eq!(wake_count.load(Ordering::SeqCst), 3);
    runtime.pump(PumpBudget::new(3, 1, 0, 0));
    assert_eq!(*runtime.state(), 1);
    assert_eq!(wake_count.load(Ordering::SeqCst), 3);
}

#[cfg(feature = "internal-test-seams")]
#[test]
fn terminal_transition_closes_retained_producers_without_external_wake() {
    let jobs = Arc::new(Mutex::new(Vec::new()));
    let wake_count = Arc::new(AtomicUsize::new(0));
    let mut runtime = AppRuntime::<App>::mount(0);
    runtime.set_send_task_executor(HoldingExecutor(Arc::clone(&jobs)));
    let count = Arc::clone(&wake_count);
    runtime.set_wake_transport(move || {
        count.fetch_add(1, Ordering::SeqCst);
    });
    runtime.pump(PumpBudget::new(2, 0, 0, 0));
    let job = jobs
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
        .pop()
        .unwrap_or_else(|| unreachable!());
    let wakes_before_terminal = wake_count.load(Ordering::SeqCst);

    runtime.__seed_next_work_sequence_for_test(0);
    assert!(runtime.submit_action(()).is_err());
    assert_eq!(wake_count.load(Ordering::SeqCst), wakes_before_terminal);
    assert!(matches!(
        run_ready(job),
        Err(SendTaskCompletionError::Closed(_))
    ));
}

struct RedrawApp;

impl UiApp for RedrawApp {
    type State = usize;
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root(_: &Self::State) -> impl View<Self::Action> {
        text("redraw")
    }
    fn update(
        state: &mut Self::State,
        (): Self::Action,
    ) -> impl IntoEffects<Self::Action, Self::HostProtocol> {
        *state += 1;
        Effects::redraw()
    }
}

#[test]
fn redraw_acknowledgment_retains_a_newer_dirty_revision() {
    let mut runtime = AppRuntime::<RedrawApp>::mount(0);
    let initial = runtime
        .take_redraw_request()
        .unwrap_or_else(|| unreachable!());
    runtime
        .acknowledge_redraw(&initial)
        .unwrap_or_else(|_| unreachable!());
    assert!(runtime.take_redraw_request().is_none());
    runtime.pump(PumpBudget::new(1, 0, 0, 0));

    runtime.submit_action(()).unwrap_or_else(|_| unreachable!());
    runtime.pump(PumpBudget::new(1, 0, 0, 0));
    let older = runtime
        .take_redraw_request()
        .unwrap_or_else(|| unreachable!());
    runtime.submit_action(()).unwrap_or_else(|_| unreachable!());
    runtime.pump(PumpBudget::new(1, 0, 0, 0));
    runtime
        .acknowledge_redraw(&older)
        .unwrap_or_else(|_| unreachable!());
    let newer = runtime
        .take_redraw_request()
        .unwrap_or_else(|| unreachable!());
    assert!(newer.revision() > older.revision());
}
