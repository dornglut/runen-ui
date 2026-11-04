#![allow(clippy::unwrap_used, clippy::panic)]
use runenui_core::*;
use runenui_runtime::{
    AppRuntime, PumpBudget, RuntimeConfig, RuntimeStatus, RuntimeTerminalReason, TraceConfig,
};
use std::{
    cell::Cell,
    future::Future,
    pin::Pin,
    rc::Rc,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
    task::{Context, Poll},
    time::Duration,
};
#[derive(Clone, Debug)]
struct State {
    mode: u8,
    returned: Rc<Cell<bool>>,
    polls: Arc<AtomicUsize>,
    drops: Arc<AtomicUsize>,
    maps: Rc<Cell<usize>>,
}
struct Token(Arc<AtomicUsize>);
impl Drop for Token {
    fn drop(&mut self) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }
}
struct Never {
    token: Token,
    polls: Arc<AtomicUsize>,
}
impl Future for Never {
    type Output = String;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<String> {
        self.polls.fetch_add(1, Ordering::SeqCst);
        Poll::Pending
    }
}
#[derive(Debug)]
struct Probe(State);
impl Widget<()> for Probe {
    type State = ();
    fn create_state(&self) {}
    fn producer_budget(&self, _: WidgetProducerPhase, (): &()) -> WidgetProducerBudget {
        WidgetProducerBudget::new(0)
    }
    fn event(
        &mut self,
        (): &mut (),
        event: &UiEvent,
        cx: &mut EventContext<'_, ()>,
    ) -> WidgetEventOutput {
        if cx.phase() != EventPhase::Target
            || event
                .as_semantic_command()
                .is_none_or(|e| e.command() != SemanticCommand::OpenMenu)
        {
            return WidgetEventOutput::default();
        }
        let s = &self.0;
        match s.mode {
            0 => {
                let token = Token(s.drops.clone());
                let polls = s.polls.clone();
                let future = async move {
                    let _keep = token;
                    polls.fetch_add(1, Ordering::SeqCst);
                    Some(())
                };
                let (future, cause) = cx.try_local_task(future).unwrap_err().into_parts();
                assert_eq!(
                    cause.component,
                    BudgetComponent::WorkStarts(WorkFamily::LocalTask)
                );
                assert_eq!(s.drops.load(Ordering::SeqCst), 0);
                drop(future);
            }
            1 => {
                let future = Never {
                    token: Token(s.drops.clone()),
                    polls: s.polls.clone(),
                };
                let maps = s.maps.clone();
                let mapper = move |_: String| {
                    maps.set(maps.get() + 1);
                };
                let ((future, mapper), cause) =
                    cx.try_send_task(future, mapper).unwrap_err().into_parts();
                assert_eq!(
                    cause.component,
                    BudgetComponent::WorkStarts(WorkFamily::SendTask)
                );
                assert!(Arc::ptr_eq(&future.token.0, &s.drops));
                assert_eq!(s.maps.get(), 0);
                drop(future);
                drop(mapper);
            }
            2 => {
                let token = Token(s.drops.clone());
                let timer = TimerEffect::once(Duration::ZERO, move || {
                    let _keep = &token;
                });
                let (timer, cause) = cx.try_timer(timer).unwrap_err().into_parts();
                assert_eq!(
                    cause.component,
                    BudgetComponent::WorkStarts(WorkFamily::Timer)
                );
                assert_eq!(s.drops.load(Ordering::SeqCst), 0);
                drop(timer);
            }
            3 => {
                let (command, cause) = cx
                    .try_emit_command(SemanticCommand::OpenMenu)
                    .unwrap_err()
                    .into_parts();
                assert_eq!(command, SemanticCommand::OpenMenu);
                assert_eq!(cause.component, BudgetComponent::Outputs);
            }
            _ => {
                let key = WorkKey::new("probe").unwrap();
                let ((family, key), _) = cx
                    .try_cancel(WorkFamily::LocalTask, key)
                    .unwrap_err()
                    .into_parts();
                assert_eq!(family, WorkFamily::LocalTask);
                assert_eq!(key, WorkKey::new("probe").unwrap());
            }
        }
        let first = cx.producer_violation().unwrap();
        let ((), later) = cx.try_emit(()).unwrap_err().into_parts();
        assert_eq!(first, later);
        s.returned.set(true);
        WidgetEventOutput::default()
    }
}
struct App;
impl UiApp for App {
    type State = State;
    type Action = ();
    type HostProtocol = NoHostProtocol;
    fn root(s: &State) -> impl View<()> {
        Element::new(Probe(s.clone()))
    }
    fn update(_: &mut State, (): ()) -> impl IntoUpdateOutput<(), NoHostProtocol> {}
    fn initial_producer_budget(_: &State) -> ProducerBudget {
        ProducerBudget::new(1, 0).with_reconciliation(1, 0, 1)
    }
    fn action_producer_budget(_: &State, (): &()) -> ProducerBudget {
        ProducerBudget::new(1, 0).with_reconciliation(1, 1, 0)
    }
    fn interaction_producer_budget(_: &State) -> ProducerBudget {
        ProducerBudget::new(16, 8)
            .with_reconciliation(1, 1, 1)
            .with_work_starts(WorkStartBudget::new(1, 1, 1, 0, 0))
            .with_work_residency(WorkResidencyBudget::new(1, 1, 1, 0, 0, 0))
    }
}
#[test]
fn task_timer_command_and_cancel_failures_return_owned_requests_before_start() {
    let budget = PumpBudget::new(usize::MAX, usize::MAX, usize::MAX, usize::MAX);
    for mode in 0..5 {
        for trace in [0, 512] {
            let state = State {
                mode,
                returned: Rc::new(Cell::new(false)),
                polls: Arc::new(AtomicUsize::new(0)),
                drops: Arc::new(AtomicUsize::new(0)),
                maps: Rc::new(Cell::new(0)),
            };
            let mut app = AppRuntime::<App>::mount_with_config(
                state.clone(),
                RuntimeConfig::default().with_trace_config(TraceConfig::new(trace)),
            )
            .unwrap();
            app.pump(budget).unwrap();
            let owner = app.index().nodes()[0].id().clone();
            app.submit_command(
                owner,
                SemanticCommand::OpenMenu,
                CommandOrigin::programmatic(),
            )
            .unwrap();
            app.pump(budget).unwrap();
            assert!(state.returned.get());
            assert_eq!(state.polls.load(Ordering::SeqCst), 0);
            assert_eq!(state.maps.get(), 0);
            assert_eq!(state.drops.load(Ordering::SeqCst), usize::from(mode < 3));
            assert!(matches!(
                app.status(),
                RuntimeStatus::Terminal(RuntimeTerminalReason::ProducerContractViolation(_))
            ));
        }
    }
}
