#![allow(refining_impl_trait)]

use std::{cell::Cell, cell::RefCell, rc::Rc};

use runenui_core::{
    CommandOrigin, Element, ElementId, EventContext, EventPhase, KeyLocation, KeyModifiers,
    KeyboardCompositionState, KeyboardEvent, KeyboardPhase, LogicalKey, NoHostProtocol,
    PhysicalKey, SemanticCommand, UiApp, UiEvent, View, Widget, WidgetEventOutput,
};
use runenui_runtime::{
    AppRuntime, InputArbitrationRecord, PumpBudget, RuntimeConfig, RuntimeStatus,
    SubmitActionErrorKind, SubmitKeyboardErrorKind, TraceConfig, UiInputFinality,
};

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum Action {
    EnableEventBudget,
    Noop,
}

#[derive(Debug)]
struct State {
    callback_log: Rc<RefCell<Vec<&'static str>>>,
    event_budget_enabled: Rc<Cell<bool>>,
    emit_output: bool,
}

#[derive(Debug)]
struct FocusLeaf {
    callback_log: Rc<RefCell<Vec<&'static str>>>,
    event_budget_enabled: Rc<Cell<bool>>,
    emit_output: bool,
}

impl Widget<Action> for FocusLeaf {
    type State = ();

    fn producer_budget(
        &self,
        phase: runenui_core::WidgetProducerPhase,
        _state: &Self::State,
    ) -> runenui_core::WidgetProducerBudget {
        match phase {
            runenui_core::WidgetProducerPhase::Event => runenui_core::WidgetProducerBudget::new(
                usize::from(self.event_budget_enabled.get()),
            ),
            runenui_core::WidgetProducerPhase::Subscriptions => {
                runenui_core::WidgetProducerBudget::new(0).with_subscriptions(0)
            }
            _ => runenui_core::WidgetProducerBudget::new(0),
        }
    }

    fn create_state(&self) -> Self::State {}

    fn event(
        &mut self,
        (): &mut Self::State,
        event: &UiEvent,
        context: &mut EventContext<'_, Action>,
    ) -> WidgetEventOutput {
        if context.phase() == EventPhase::Target && matches!(event, UiEvent::Keyboard(_)) {
            self.callback_log.borrow_mut().push("keyboard");
            if self.emit_output {
                let _ = context.try_emit(Action::Noop);
            }
        }
        WidgetEventOutput::none()
    }

    fn activation(&self, (): &Self::State) -> runenui_core::WidgetActivation {
        runenui_core::WidgetActivation::actionable(true)
    }
}

struct App;

impl UiApp for App {
    type State = State;
    type Action = Action;
    type HostProtocol = NoHostProtocol;

    fn initial_producer_budget(_state: &Self::State) -> runenui_core::ProducerBudget {
        runenui_core::ProducerBudget::new(1, 0)
            .with_reconciliation(1, 0, 1)
            .with_subscriptions(0, 0)
    }

    fn action_producer_budget(
        _state: &Self::State,
        _action: &Self::Action,
    ) -> runenui_core::ProducerBudget {
        runenui_core::ProducerBudget::new(1, 0)
            .with_reconciliation(0, 1, 0)
            .with_subscriptions(0, 0)
    }

    fn interaction_producer_budget(_state: &Self::State) -> runenui_core::ProducerBudget {
        // One retained leaf has at most one output per event callback. Cover
        // the framework's four focus-notification allowances across both
        // reachable event-budget states, rather than reading only today's state.
        // Emitted Noop actions also retain and update the one mounted leaf.
        runenui_core::ProducerBudget::new(6, 4).with_reconciliation(0, 1, 0)
    }

    fn root(state: &Self::State) -> impl View<Action> {
        Element::new(FocusLeaf {
            callback_log: Rc::clone(&state.callback_log),
            event_budget_enabled: Rc::clone(&state.event_budget_enabled),
            emit_output: state.emit_output,
        })
        .id("focus-leaf")
        .key("focus-leaf")
        .focusable(true)
    }

    fn update(state: &mut Self::State, action: Self::Action) {
        if matches!(action, Action::EnableEventBudget) {
            state.event_budget_enabled.set(true);
        }
    }
}

fn keyboard(phase: KeyboardPhase) -> KeyboardEvent {
    KeyboardEvent::new(
        phase,
        PhysicalKey::Code(String::from("KeyQ")),
        LogicalKey::Character(String::from("q")),
        KeyModifiers::NONE,
        false,
        KeyLocation::Standard,
        KeyboardCompositionState::Inactive,
        None,
    )
}

fn focused_runtime(
    capacity: usize,
    trace_capacity: usize,
    emit_output: bool,
) -> (AppRuntime<App>, Rc<RefCell<Vec<&'static str>>>) {
    let callback_log = Rc::new(RefCell::new(Vec::new()));
    let mut runtime = AppRuntime::<App>::mount_with_config(
        State {
            callback_log: Rc::clone(&callback_log),
            event_budget_enabled: Rc::new(Cell::new(false)),
            emit_output,
        },
        RuntimeConfig::default()
            .with_queue_capacity(capacity)
            .with_trace_config(TraceConfig::new(trace_capacity)),
    )
    .unwrap_or_else(|error| unreachable!("bounded initial profile is accepted: {error}"));

    runtime
        .pump(PumpBudget::new(3, 3, 3, 3))
        .unwrap_or_else(|_| unreachable!("initial reconciliation settles"));
    let leaf_id = ElementId::new("focus-leaf").unwrap_or_else(|_| unreachable!());
    let target = runtime
        .index()
        .nodes()
        .iter()
        .find(|node| node.authored_id() == Some(&leaf_id))
        .unwrap_or_else(|| unreachable!("the focusable leaf is mounted"))
        .id()
        .clone();
    runtime
        .submit_command(
            target,
            SemanticCommand::RequestFocus,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|error| {
            unreachable!("the declared focus profile fits the empty queue: {error:?}")
        });
    runtime
        .pump(PumpBudget::new(3, 3, 3, 3))
        .unwrap_or_else(|_| unreachable!("focus commits before the pressure sequence"));
    runtime
        .submit_action(Action::EnableEventBudget)
        .unwrap_or_else(|error| unreachable!("event-budget activation is admitted: {error}"));
    runtime
        .pump(PumpBudget::new(3, 3, 3, 3))
        .unwrap_or_else(|_| unreachable!("event-budget activation settles"));
    callback_log.borrow_mut().clear();
    (runtime, callback_log)
}

#[test]
fn accepted_keyboard_route_keeps_its_outputs_under_later_fifo_pressure() {
    const CAPACITY: usize = 16;
    let (mut runtime, callback_log) = focused_runtime(CAPACITY, 0, false);
    for phase in [KeyboardPhase::Down, KeyboardPhase::Up] {
        let control = runtime
            .submit_keyboard(keyboard(phase))
            .unwrap_or_else(|_| {
                unreachable!("the same route is admissible without follower pressure")
            });
        let batch = runtime
            .pump(PumpBudget::new(1, 0, 0, 0))
            .unwrap_or_else(|_| {
                unreachable!("the unpressured control reaches canonical settlement")
            });
        assert!(batch.ordered_records().iter().any(|record| matches!(
            record,
            InputArbitrationRecord::InputSettled(settlement)
                if settlement.sequence() == control.sequence()
                    && matches!(settlement.finality(), UiInputFinality::Committed(_))
        )));
    }
    assert_eq!(&*callback_log.borrow(), &["keyboard", "keyboard"]);
    callback_log.borrow_mut().clear();

    let receipt = runtime
        .submit_keyboard(keyboard(KeyboardPhase::Down))
        .unwrap_or_else(|error| {
            unreachable!("keyboard ingress accepts its one queued envelope: {error}")
        });
    let mut followers = Vec::new();
    let mut refused = false;
    for _ in 0..CAPACITY {
        match runtime.submit_action(Action::Noop) {
            Ok(sequence) => followers.push(sequence),
            Err(error) => {
                assert_eq!(error.kind(), SubmitActionErrorKind::Full);
                assert_eq!(error.into_action(), Action::Noop);
                refused = true;
                break;
            }
        }
    }
    assert!(
        refused,
        "bounded follower admission eventually returns its exact payload"
    );
    assert!(
        !followers.is_empty(),
        "the positive fixture has room for unrelated work"
    );
    assert!(callback_log.borrow().is_empty());
    assert_eq!(runtime.status(), RuntimeStatus::Running);

    let batch = runtime
        .pump(PumpBudget::new(1, 0, 0, 0))
        .unwrap_or_else(|_| unreachable!("accepted keyboard receipt has its output frontier"));
    assert_eq!(batch.processed_through(), Some(receipt.sequence()));
    assert_eq!(runtime.status(), RuntimeStatus::Running);
    let settlements: Vec<_> = batch
        .ordered_records()
        .iter()
        .filter_map(|record| match record {
            InputArbitrationRecord::InputSettled(settlement) => Some(settlement),
            _ => None,
        })
        .collect();
    assert_eq!(settlements.len(), 1);
    assert_eq!(settlements[0].sequence(), receipt.sequence());
    assert!(
        matches!(settlements[0].finality(), UiInputFinality::Committed(_)),
        "the same valid route committed without pressure; actual pressured finality: {:?}",
        settlements[0].finality()
    );
    assert_eq!(&*callback_log.borrow(), &["keyboard"]);
    assert!(
        followers
            .iter()
            .all(|sequence| receipt.sequence() < *sequence)
    );
    assert!(
        !batch.ordered_records().iter().any(|record| matches!(
            record,
            InputArbitrationRecord::InputSettled(settlement)
                if settlement.sequence() != receipt.sequence()
        )),
        "only the original input is settled by the one-envelope pump"
    );
}

#[test]
fn keyboard_output_keeps_its_frontier_and_refusal_returns_the_original_event() {
    let (mut runtime, callback_log) = focused_runtime(16, 128, true);
    let first = runtime
        .submit_keyboard(keyboard(KeyboardPhase::Down))
        .unwrap_or_else(|_| unreachable!("the expected queued result is present"));
    let second = runtime
        .submit_keyboard(keyboard(KeyboardPhase::Up))
        .unwrap_or_else(|_| unreachable!("the expected queued result is present"));
    let mut follower_sequences = Vec::new();
    for _ in 0..4 {
        follower_sequences.push(
            runtime
                .submit_action(Action::Noop)
                .unwrap_or_else(|_| unreachable!("the expected queued result is present")),
        );
    }
    let rejected = runtime
        .submit_keyboard(keyboard(KeyboardPhase::Down))
        .err()
        .unwrap_or_else(|| unreachable!("the submitted keyboard event is refused"));
    assert_eq!(rejected.kind(), SubmitKeyboardErrorKind::Full);
    let original = rejected.into_event();
    assert_eq!(original.phase(), KeyboardPhase::Down);
    assert_eq!(
        original.physical_key(),
        &PhysicalKey::Code(String::from("KeyQ"))
    );
    assert!(callback_log.borrow().is_empty());
    assert_eq!(runtime.status(), RuntimeStatus::Running);
    let mut settled = Vec::new();
    let mut processed = Vec::new();
    for _ in 0..8 {
        let batch = runtime
            .pump(PumpBudget::new(1, 0, 0, 0))
            .unwrap_or_else(|_| unreachable!("the expected queued result is present"));
        processed.push(
            batch
                .processed_through()
                .unwrap_or_else(|| unreachable!("the pump processed an envelope")),
        );
        for record in batch.ordered_records() {
            if let InputArbitrationRecord::InputSettled(settlement) = record {
                assert!(
                    matches!(settlement.finality(), UiInputFinality::Committed(_)),
                    "unexpected finality: {:?}; trace: {:?}",
                    settlement.finality(),
                    runtime.trace().records().collect::<Vec<_>>()
                );
                settled.push(settlement.sequence());
            }
        }
    }
    assert_eq!(settled, vec![first.sequence(), second.sequence()]);
    assert_eq!(&processed[..2], &[first.sequence(), second.sequence()]);
    assert_eq!(&processed[2..6], follower_sequences.as_slice());
    assert_eq!(&*callback_log.borrow(), &["keyboard", "keyboard"]);
    let retry = runtime
        .submit_keyboard(original)
        .unwrap_or_else(|_| unreachable!("retired roots return their credits"));
    assert!(retry.sequence() > second.sequence());
    assert_eq!(retry.sequence().get(), second.sequence().get() + 7);
    let batch = runtime
        .pump(PumpBudget::new(1, 0, 0, 0))
        .unwrap_or_else(|_| unreachable!("the expected queued result is present"));
    assert!(batch.ordered_records().iter().any(|record| matches!(
        record,
        InputArbitrationRecord::InputSettled(settlement)
            if settlement.sequence() == retry.sequence()
                && matches!(settlement.finality(), UiInputFinality::Committed(_))
    )));
    assert_eq!(
        &*callback_log.borrow(),
        &["keyboard", "keyboard", "keyboard"]
    );
    assert_eq!(runtime.status(), RuntimeStatus::Running);
}

#[test]
fn trace_refusal_releases_the_unaccepted_keyboard_frontier() {
    let (mut runtime, callback_log) = focused_runtime(16, 128, false);
    runtime.__seed_next_trace_sequence_for_test(u64::MAX - 1);
    // Each attempt temporarily needs six credits. A leaked reservation would
    // turn the third attempt into Full instead of the original trace refusal.
    for _ in 0..4 {
        let error = runtime
            .submit_keyboard(keyboard(KeyboardPhase::Down))
            .err()
            .unwrap_or_else(|| unreachable!("the submitted keyboard event is refused"));
        assert_eq!(
            error.kind(),
            SubmitKeyboardErrorKind::TraceSequenceExhausted
        );
        assert_eq!(error.into_event().phase(), KeyboardPhase::Down);
        assert!(callback_log.borrow().is_empty());
        assert_eq!(runtime.status(), RuntimeStatus::Running);
    }
}
