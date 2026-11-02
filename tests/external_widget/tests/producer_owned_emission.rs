#![allow(clippy::unwrap_used, clippy::panic)]
#![forbid(unsafe_code)]

use runenui_core::{
    CommandOrigin, Element, EventContext, EventPhase, NoHostProtocol, ProducerBudget,
    SemanticCommand, UiApp, UiEvent, View, Widget, WidgetEventOutput, WidgetProducerBudget,
    WidgetProducerPhase,
};
use runenui_runtime::{
    AppRuntime, PumpBudget, RuntimeConfig, RuntimeStatus, RuntimeTerminalReason, TraceConfig,
    TraceRecordKind, TraceRoutedIntegrityFailure,
};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

// Deliberately neither Clone nor Send. Drop distinguishes the buffered first
// action from the over-budget attempted second action.
struct Payload {
    id: usize,
    dropped: Rc<[Cell<usize>; 2]>,
}
impl Drop for Payload {
    fn drop(&mut self) {
        self.dropped[self.id].set(self.dropped[self.id].get() + 1);
    }
}

#[derive(Clone, Debug)]
struct State {
    allowance: usize,
    mutation: Rc<Cell<usize>>,
    returned: Rc<Cell<bool>>,
    dropped: Rc<[Cell<usize>; 2]>,
    applied: Rc<RefCell<Vec<usize>>>,
    recovered: Rc<Cell<bool>>,
    mapped: bool,
    ignored: bool,
    armed: Rc<Cell<bool>>,
    skip_once: Rc<Cell<bool>>,
    maps: Rc<Cell<usize>>,
}
impl State {
    fn new(allowance: usize) -> Self {
        Self {
            allowance,
            recovered: Rc::new(Cell::new(false)),
            mapped: false,
            ignored: false,
            armed: Rc::new(Cell::new(true)),
            skip_once: Rc::new(Cell::new(false)),
            maps: Rc::new(Cell::new(0)),
            mutation: Rc::new(Cell::new(0)),
            returned: Rc::new(Cell::new(false)),
            dropped: Rc::new([Cell::new(0), Cell::new(0)]),
            applied: Rc::new(RefCell::new(Vec::new())),
        }
    }
}

#[derive(Debug)]
struct PublicWidget(State);
impl Widget<Payload> for PublicWidget {
    type State = ();
    fn create_state(&self) {}
    fn producer_budget(&self, phase: WidgetProducerPhase, (): &()) -> WidgetProducerBudget {
        WidgetProducerBudget::new(if phase == WidgetProducerPhase::Event {
            self.0.allowance
        } else {
            0
        })
    }
    fn event(
        &mut self,
        (): &mut (),
        event: &UiEvent,
        context: &mut EventContext<'_, Payload>,
    ) -> WidgetEventOutput {
        if self.0.armed.get()
            && context.phase() == EventPhase::Target
            && (event.as_keyboard().is_some()
                || event
                    .as_semantic_command()
                    .is_some_and(|e| e.command() == SemanticCommand::OpenMenu))
        {
            if self.0.skip_once.replace(false) {
                return WidgetEventOutput::default();
            }
            self.0.mutation.set(self.0.mutation.get() + 1);
            for id in 0..2 {
                let result = context.try_emit(Payload {
                    id,
                    dropped: self.0.dropped.clone(),
                });
                if self.0.ignored {
                    drop(result);
                    continue;
                }
                if let Err(error) = result {
                    let (attempted, cause) = error.into_parts();
                    assert_eq!(attempted.id, id);
                    assert!(Rc::ptr_eq(&attempted.dropped, &self.0.dropped));
                    assert_eq!(context.producer_violation(), Some(cause));
                    self.0.recovered.set(true);
                    drop(attempted);
                }
            }
            if self.0.ignored {
                let _ = context.try_emit(Payload {
                    id: 1,
                    dropped: self.0.dropped.clone(),
                });
            }
            // Arbitrary author code continues until callback return.
            self.0.returned.set(true);
        }
        WidgetEventOutput::default()
    }
}

struct App;
impl UiApp for App {
    type State = State;
    type Action = Payload;
    type HostProtocol = NoHostProtocol;
    fn root(s: &State) -> impl View<Payload> {
        let element = Element::new(PublicWidget(s.clone())).focusable(true);
        if s.mapped {
            let maps = s.maps.clone();
            element.map_action(move |a| {
                maps.set(maps.get() + 1);
                a
            })
        } else {
            element
        }
    }
    fn update(
        s: &mut State,
        a: Payload,
    ) -> impl runenui_core::IntoUpdateOutput<Payload, NoHostProtocol> {
        s.applied.borrow_mut().push(a.id);
    }
    fn initial_producer_budget(_: &State) -> ProducerBudget {
        ProducerBudget::new(1, 0).with_reconciliation(1, 0, 1)
    }
    fn action_producer_budget(_: &State, _: &Payload) -> ProducerBudget {
        ProducerBudget::new(1, 0).with_reconciliation(1, 1, 0)
    }
    fn interaction_producer_budget(_: &State) -> ProducerBudget {
        ProducerBudget::new(24, 16).with_reconciliation(1, 1, 1)
    }
}

fn exercise(allowance: usize, trace: usize) -> (AppRuntime<App>, State) {
    let state = State::new(allowance);
    let mut runtime = AppRuntime::<App>::mount_with_config(
        state.clone(),
        RuntimeConfig::default().with_trace_config(TraceConfig::new(trace)),
    )
    .unwrap();
    let full = PumpBudget::new(usize::MAX, usize::MAX, usize::MAX, usize::MAX);
    runtime.pump(full).unwrap();
    let owner = runtime.index().nodes()[0].id().clone();
    runtime
        .submit_command(
            owner,
            SemanticCommand::OpenMenu,
            CommandOrigin::programmatic(),
        )
        .unwrap();
    runtime.pump(full).unwrap();
    (runtime, state)
}

#[test]
fn declared_callback_delivers_nonclone_outputs_once_in_fifo_order() {
    for trace in [0, 512] {
        let (runtime, state) = exercise(2, trace);
        assert_eq!(runtime.status(), RuntimeStatus::Running);
        assert_eq!(*state.applied.borrow(), [0, 1]);
        assert_eq!([state.dropped[0].get(), state.dropped[1].get()], [1, 1]);
        assert_eq!(state.mutation.get(), 1);
        assert!(state.returned.get());
    }
}

#[test]
fn underdeclared_callback_drops_attempted_and_uncommitted_payloads_and_poison_is_distinct() {
    for trace in [0, 512] {
        let (runtime, state) = exercise(1, trace);
        assert_eq!(
            runtime.status(),
            RuntimeStatus::Terminal(RuntimeTerminalReason::ProducerContractViolation(
                runenui_core::ProducerContractViolation {
                    kind: runenui_core::ProducerViolationKind::DeclarationExceeded,
                    phase: runenui_core::ProducerPhase::Widget(WidgetProducerPhase::Event),
                    component: runenui_core::BudgetComponent::Outputs,
                    declared: 1,
                    attempted: Some(2)
                }
            ))
        );
        assert!(state.applied.borrow().is_empty());
        assert_eq!([state.dropped[0].get(), state.dropped[1].get()], [1, 1]);
        assert_eq!(
            state.mutation.get(),
            1,
            "arbitrary callback mutation is not rolled back"
        );
        assert!(
            state.returned.get(),
            "void emit does not notify callback author"
        );
        if trace != 0 {
            assert!(runtime.trace().kinds().any(|k| matches!(
                k,
                TraceRecordKind::RoutedIntegrityFailed {
                    failure: TraceRoutedIntegrityFailure::ProducerViolation(_),
                    ..
                }
            )));
        }
    }
}

#[test]
fn ordinary_full_preserves_exact_nonclone_payload_without_mutation_or_acceptance() {
    let state = State::new(2);
    let mut runtime = AppRuntime::<App>::mount_with_config(
        state.clone(),
        RuntimeConfig::default()
            .with_queue_capacity(1)
            .with_trace_config(TraceConfig::new(512)),
    )
    .unwrap();
    let full = PumpBudget::new(usize::MAX, usize::MAX, usize::MAX, usize::MAX);
    runtime.pump(full).unwrap();
    let first = runtime
        .submit_action(Payload {
            id: 0,
            dropped: state.dropped.clone(),
        })
        .unwrap();
    let accepted_count = || {
        runtime
            .trace()
            .kinds()
            .filter(|kind| matches!(kind, TraceRecordKind::ActionSubmissionAccepted))
            .count()
    };
    let before_accepted = accepted_count();
    let refused = runtime
        .submit_action(Payload {
            id: 1,
            dropped: state.dropped.clone(),
        })
        .unwrap_err();
    assert_eq!(refused.kind(), runenui_runtime::SubmitActionErrorKind::Full);
    assert_eq!(
        runtime
            .trace()
            .kinds()
            .filter(|kind| matches!(kind, TraceRecordKind::ActionSubmissionAccepted))
            .count(),
        before_accepted
    );
    assert!(
        runtime
            .trace()
            .kinds()
            .any(|kind| matches!(kind, TraceRecordKind::ActionSubmissionRejectedFull))
    );
    assert_eq!(state.mutation.get(), 0);
    assert!(state.applied.borrow().is_empty());
    assert_eq!([state.dropped[0].get(), state.dropped[1].get()], [0, 0]);
    let returned = refused.into_action();
    assert_eq!(returned.id, 1);
    assert!(Rc::ptr_eq(&returned.dropped, &state.dropped));
    runtime.pump(full).unwrap();
    let next = runtime.submit_action(returned).unwrap();
    assert_eq!(
        next.get(),
        first.get() + 1,
        "refusal consumes no work sequence"
    );
    runtime.pump(full).unwrap();
    assert_eq!(runtime.status(), RuntimeStatus::Running);
    assert_eq!(*state.applied.borrow(), [0, 1]);
    assert_eq!([state.dropped[0].get(), state.dropped[1].get()], [1, 1]);
}

#[test]
fn mapped_overrun_returns_child_value_before_mapper() {
    for trace in [0, 512] {
        let mut state = State::new(1);
        state.mapped = true;
        let mut app = AppRuntime::<App>::mount_with_config(
            state.clone(),
            RuntimeConfig::default().with_trace_config(TraceConfig::new(trace)),
        )
        .unwrap();
        app.pump(FULL).unwrap();
        let owner = app.index().nodes()[0].id().clone();
        app.submit_command(
            owner,
            SemanticCommand::OpenMenu,
            CommandOrigin::programmatic(),
        )
        .unwrap();
        app.pump(FULL).unwrap();
        assert!(state.recovered.get());
        assert_eq!(state.maps.get(), 0);
        assert!(state.applied.borrow().is_empty());
        assert!(matches!(
            app.status(),
            RuntimeStatus::Terminal(RuntimeTerminalReason::ProducerContractViolation(_))
        ));
        assert_eq!(state.dropped[0].get(), 1);
        assert_eq!(state.dropped[1].get(), 1);
    }
}
const FULL: PumpBudget = PumpBudget::new(usize::MAX, usize::MAX, usize::MAX, usize::MAX);
#[test]
fn ignored_fallible_error_has_sticky_terminal_status_without_trace() {
    let mut state = State::new(1);
    state.ignored = true;
    let mut app = AppRuntime::<App>::mount_with_config(
        state.clone(),
        RuntimeConfig::default().with_trace_config(TraceConfig::new(0)),
    )
    .unwrap();
    app.pump(FULL).unwrap();
    let owner = app.index().nodes()[0].id().clone();
    app.submit_command(
        owner,
        SemanticCommand::OpenMenu,
        CommandOrigin::programmatic(),
    )
    .unwrap();
    app.pump(FULL).unwrap();
    assert!(state.returned.get());
    assert!(!state.recovered.get());
    assert!(state.applied.borrow().is_empty());
    assert_eq!(state.dropped[0].get(), 1);
    assert_eq!(state.dropped[1].get(), 2);
    assert!(matches!(
        app.status(),
        RuntimeStatus::Terminal(RuntimeTerminalReason::ProducerContractViolation(_))
    ));
}

#[test]
fn real_input_abort_preserves_earlier_settlement_and_retires_scope_once() {
    use runenui_core::{
        KeyLocation, KeyModifiers, KeyboardCompositionState, KeyboardEvent, KeyboardPhase,
        LogicalKey, PhysicalKey,
    };
    use runenui_runtime::{InputArbitrationRecord, UiInputFinality};
    for trace in [0, 512] {
        let state = State::new(1);
        let mut app = AppRuntime::<App>::mount_with_config(
            state.clone(),
            RuntimeConfig::default().with_trace_config(TraceConfig::new(trace)),
        )
        .unwrap();
        app.pump(FULL).unwrap();
        let owner = app.index().nodes()[0].id().clone();
        app.submit_command(
            owner,
            SemanticCommand::RequestFocus,
            CommandOrigin::programmatic(),
        )
        .unwrap();
        app.pump(FULL).unwrap();
        state.skip_once.set(true);
        let key = |code: &str| {
            KeyboardEvent::new(
                KeyboardPhase::Down,
                PhysicalKey::Code(code.into()),
                LogicalKey::Character("a".into()),
                KeyModifiers::NONE,
                false,
                KeyLocation::Standard,
                KeyboardCompositionState::Inactive,
                None,
            )
        };
        let first = app.submit_keyboard(key("KeyA")).unwrap().sequence();
        let second = app.submit_keyboard(key("KeyB")).unwrap().sequence();
        let batch = app.pump(FULL).unwrap();
        let settled: Vec<_> = batch
            .ordered_records()
            .iter()
            .filter_map(|r| {
                if let InputArbitrationRecord::InputSettled(s) = r {
                    Some(s)
                } else {
                    None
                }
            })
            .collect();
        assert_eq!(settled.len(), 2);
        assert_eq!(settled[0].sequence(), first);
        assert_eq!(settled[1].sequence(), second);
        assert!(matches!(
            settled[0].finality(),
            UiInputFinality::Committed(_)
        ));
        assert!(matches!(settled[1].finality(), UiInputFinality::Aborted(_)));
        assert!(matches!(
            batch.final_ownership().status(),
            RuntimeStatus::Terminal(RuntimeTerminalReason::ProducerContractViolation(_))
        ));
        assert_eq!(
            batch
                .ordered_records()
                .iter()
                .filter(|r| matches!(r, InputArbitrationRecord::ScopeRetired(_)))
                .count(),
            1
        );
        let closed = app.shutdown().unwrap();
        assert_eq!(closed.final_ownership().status(), RuntimeStatus::Closed);
        assert!(
            !closed
                .ordered_records()
                .iter()
                .any(|r| matches!(r, InputArbitrationRecord::ScopeRetired(_)))
        );
        assert_eq!(state.dropped[0].get(), 1);
        assert_eq!(state.dropped[1].get(), 1);
    }
}
