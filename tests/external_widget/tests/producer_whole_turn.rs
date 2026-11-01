use runenui_core::*;
use runenui_runtime::{AppRuntime, PumpBudget, RuntimeStatus, RuntimeTerminalReason};
use std::{
    cell::Cell,
    future::Future,
    pin::Pin,
    rc::Rc,
    task::{Context, Poll},
};

struct Token(Rc<Cell<usize>>);
impl Drop for Token {
    fn drop(&mut self) {
        self.0.set(self.0.get() + 1);
    }
}
struct PendingTask {
    _token: Token,
    polls: Rc<Cell<usize>>,
}
impl Future for PendingTask {
    type Output = Option<()>;
    fn poll(self: Pin<&mut Self>, _: &mut Context<'_>) -> Poll<Self::Output> {
        self.polls.set(self.polls.get() + 1);
        Poll::Pending
    }
}
#[derive(Debug)]
struct Owner {
    drops: Rc<Cell<usize>>,
    polls: Rc<Cell<usize>>,
}
impl Widget<()> for Owner {
    type State = ();
    fn create_state(&self) {}
    fn producer_budget(&self, phase: WidgetProducerPhase, (): &()) -> WidgetProducerBudget {
        if phase == WidgetProducerPhase::Event {
            WidgetProducerBudget::new(1).with_work_starts(WorkStartBudget::new(1, 0, 0, 0, 0))
        } else {
            WidgetProducerBudget::new(0)
        }
    }
    fn event(
        &mut self,
        (): &mut (),
        event: &UiEvent,
        cx: &mut EventContext<'_, ()>,
    ) -> WidgetEventOutput {
        if event
            .as_focus()
            .is_some_and(|focus| focus.kind() == FocusEventKind::Out)
        {
            let _ = cx.try_local_task(PendingTask {
                _token: Token(Rc::clone(&self.drops)),
                polls: Rc::clone(&self.polls),
            });
        }
        WidgetEventOutput::none()
    }
}
struct State {
    declared: usize,
    enabled: bool,
    drops: Rc<Cell<usize>>,
    polls: Rc<Cell<usize>>,
}
struct App;
impl UiApp for App {
    type State = State;
    type Action = ();
    type HostProtocol = NoHostProtocol;
    fn initial_producer_budget(_: &State) -> ProducerBudget {
        ProducerBudget::new(1, 0).with_reconciliation(1, 0, 1)
    }
    fn action_producer_budget(state: &State, (): &()) -> ProducerBudget {
        ProducerBudget::new(16, 8)
            .with_application_outputs(1)
            .with_reconciliation(0, 1, 0)
            .with_work_starts(WorkStartBudget::new(state.declared, 0, 0, 0, 0))
            .with_work_residency(WorkResidencyBudget::new(2, 0, 0, 0, 0, 0))
    }
    fn interaction_producer_budget(_: &State) -> ProducerBudget {
        ProducerBudget::new(40, 20)
            .with_application_outputs(0)
            .with_reconciliation(0, 1, 0)
            .with_work_starts(WorkStartBudget::new(5, 0, 0, 0, 0))
            .with_work_residency(WorkResidencyBudget::new(2, 0, 0, 0, 0, 0))
    }
    fn root(state: &State) -> impl View<()> {
        Element::new(Owner {
            drops: Rc::clone(&state.drops),
            polls: Rc::clone(&state.polls),
        })
        .key("owner")
        .focusable(state.enabled)
    }
    fn update(state: &mut State, (): ()) -> impl IntoUpdateOutput<(), NoHostProtocol> {
        state.enabled = false;
        Effects::local_task(PendingTask {
            _token: Token(Rc::clone(&state.drops)),
            polls: Rc::clone(&state.polls),
        })
    }
}
#[test]
fn application_and_retirement_work_share_the_whole_turn_family_limit() {
    for declared in [1, 2] {
        for trace in [0, 512] {
            let drops = Rc::new(Cell::new(0));
            let polls = Rc::new(Cell::new(0));
            let mut app = AppRuntime::<App>::mount_with_config(
                State {
                    declared,
                    enabled: true,
                    drops: Rc::clone(&drops),
                    polls: Rc::clone(&polls),
                },
                runenui_runtime::RuntimeConfig::default()
                    .with_trace_config(runenui_runtime::TraceConfig::new(trace)),
            )
            .unwrap_or_else(|_| unreachable!("one owner"));
            let pump = PumpBudget::new(16, 0, 0, 0);
            app.pump(pump).unwrap_or_else(|_| unreachable!());
            let target = app.index().nodes()[0].id().clone();
            app.submit_command(
                target,
                SemanticCommand::RequestFocus,
                CommandOrigin::programmatic(),
            )
            .unwrap_or_else(|_| unreachable!());
            app.pump(pump).unwrap_or_else(|_| unreachable!());
            app.submit_action(()).unwrap_or_else(|_| unreachable!());
            app.pump(PumpBudget::new(1, 0, 0, 0))
                .unwrap_or_else(|_| unreachable!());
            assert!(!app.state().enabled, "application state is not rolled back");
            if declared == 1 {
                assert!(
                    matches!(
                        app.status(),
                        RuntimeStatus::Terminal(RuntimeTerminalReason::ProducerContractViolation(
                            ProducerContractViolation {
                                component: BudgetComponent::WorkStarts(WorkFamily::LocalTask),
                                declared: 1,
                                attempted: Some(2),
                                ..
                            }
                        ))
                    ),
                    "actual status: {:?}",
                    app.status()
                );
                assert_eq!(drops.get(), 2);
            } else {
                assert_eq!(app.status(), RuntimeStatus::Running);
                assert_eq!(drops.get(), 0);
            }
            assert_eq!(polls.get(), 0);
            app.shutdown()
                .unwrap_or_else(|_| unreachable!("shutdown observation"));
            drop(app);
            assert_eq!(drops.get(), 2);
            assert_eq!(polls.get(), 0);
        }
    }
}
