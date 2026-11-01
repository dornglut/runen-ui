#![allow(refining_impl_trait)]

use runenui_core::{
    CommandOrigin, Element, EventContext, EventPhase, FocusEventKind, KeyLocation, KeyModifiers,
    KeyboardCompositionState, KeyboardEvent, KeyboardPhase, LogicalKey, NoHostProtocol,
    PhysicalKey, ProducerBudget, SemanticCommand, UiApp, UiEvent, View, Widget, WidgetEventOutput,
    WidgetProducerBudget, WidgetProducerPhase, WorkKey, WorkResidencyBudget, WorkStartBudget,
};
use runenui_runtime::{
    AppRuntime, PumpBudget, RuntimeConfig, RuntimeLimits, RuntimeStatus, RuntimeTerminalReason,
    TraceConfig, TraceRecordKind, TraceWorkFamily,
};
use std::{cell::RefCell, rc::Rc};

enum Action {
    Arm,
    Disable,
}
struct State {
    same_key: bool,
    armed: bool,
    enabled: bool,
    callbacks: Rc<RefCell<Vec<&'static str>>>,
}
#[derive(Debug)]
struct Owner {
    same_key: bool,
    armed: bool,
    enabled: bool,
    callbacks: Rc<RefCell<Vec<&'static str>>>,
}

impl Widget<Action> for Owner {
    type State = bool;
    fn create_state(&self) -> bool {
        false
    }
    fn producer_budget(&self, phase: WidgetProducerPhase, _: &bool) -> WidgetProducerBudget {
        if phase == WidgetProducerPhase::Event && self.armed {
            WidgetProducerBudget::new(if self.enabled { 2 } else { 1 })
                .with_work_starts(WorkStartBudget::new(1, 0, 0, 0, 0))
        } else {
            WidgetProducerBudget::new(0)
        }
    }
    fn event(
        &mut self,
        held: &mut bool,
        event: &UiEvent,
        context: &mut EventContext<'_, Action>,
    ) -> WidgetEventOutput {
        if context.phase() != EventPhase::Target {
            return WidgetEventOutput::none();
        }
        let down = matches!(event, UiEvent::Keyboard(key) if key.phase() == KeyboardPhase::Down);
        let out = event
            .as_focus()
            .is_some_and(|focus| focus.kind() == FocusEventKind::Out);
        if self.armed && (down || out) {
            *held = down;
            self.callbacks
                .borrow_mut()
                .push(if down { "down" } else { "out" });
            if down || self.same_key {
                let _ = context.try_keyed_local_task(
                    WorkKey::new("focus.work").unwrap_or_else(|_| unreachable!()),
                    std::future::pending::<Option<Action>>(),
                );
            } else {
                // Deliberately violates the declared one-record footprint.
                let _ = context.try_local_task(std::future::pending::<Option<Action>>());
            }
            if down {
                let _ = context.try_emit(Action::Disable);
            }
        }
        WidgetEventOutput::none()
    }
}

struct App;
const fn routed_profile() -> ProducerBudget {
    // One target's event profile is counted once for the command and four
    // times for generic focus notifications: 10 outputs, five possible starts
    // and five future credits. The qualified producer replaces one owner-local
    // key; the negative fixture deliberately attempts a second live record.
    ProducerBudget::new(20, 15)
        .with_application_outputs(0)
        .with_reconciliation(0, 1, 0)
        .with_work_starts(WorkStartBudget::new(5, 0, 0, 0, 0))
        .with_work_residency(WorkResidencyBudget::new(1, 0, 0, 0, 0, 0))
}
impl UiApp for App {
    type State = State;
    type Action = Action;
    type HostProtocol = NoHostProtocol;
    fn initial_producer_budget(_: &State) -> ProducerBudget {
        ProducerBudget::new(1, 0).with_reconciliation(1, 0, 1)
    }
    fn action_producer_budget(_: &State, action: &Action) -> ProducerBudget {
        match action {
            Action::Arm => ProducerBudget::new(1, 0).with_reconciliation(0, 1, 0),
            Action::Disable => routed_profile(),
        }
    }
    fn interaction_producer_budget(_: &State) -> ProducerBudget {
        routed_profile()
    }
    fn root(state: &State) -> impl View<Action> {
        Element::new(Owner {
            same_key: state.same_key,
            armed: state.armed,
            enabled: state.enabled,
            callbacks: Rc::clone(&state.callbacks),
        })
        .key("focus.owner")
        .focusable(state.enabled)
    }
    fn update(state: &mut State, action: Action) {
        match action {
            Action::Arm => state.armed = true,
            Action::Disable => state.enabled = false,
        }
    }
}

#[test]
fn admitted_focus_cleanup_reuses_only_its_reserved_task_footprint() {
    for (same_key, capacity, trace) in [(true, 1), (false, 4)]
        .into_iter()
        .flat_map(|(same_key, capacity)| [0, 512].map(|trace| (same_key, capacity, trace)))
    {
        let callbacks = Rc::new(RefCell::new(Vec::new()));
        let mut app = AppRuntime::<App>::mount_with_config(
            State {
                same_key,
                armed: false,
                enabled: true,
                callbacks: Rc::clone(&callbacks),
            },
            RuntimeConfig::default()
                .with_limits(
                    RuntimeLimits::default()
                        .with_waiting_envelopes(32)
                        .with_transaction_outputs(16)
                        .with_local_tasks(capacity),
                )
                .with_trace_config(TraceConfig::new(trace)),
        )
        .unwrap_or_else(|error| unreachable!("pure initial tree fits: {error}"));
        let queue_only = PumpBudget::new(usize::MAX, 0, 0, 0);
        app.pump(queue_only).unwrap_or_else(|_| unreachable!());
        let target = app.index().nodes()[0].id().clone();
        app.submit_command(
            target.clone(),
            SemanticCommand::RequestFocus,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|error| unreachable!("unarmed focus route fits: {error}"));
        app.pump(queue_only).unwrap_or_else(|_| unreachable!());
        assert_eq!(app.focus().focused_node(), Some(&target));
        app.submit_action(Action::Arm)
            .unwrap_or_else(|_| unreachable!());
        app.pump(queue_only).unwrap_or_else(|_| unreachable!());
        app.submit_keyboard(KeyboardEvent::new(
            KeyboardPhase::Down,
            PhysicalKey::Code(String::from("KeyW")),
            LogicalKey::Character(String::from("w")),
            KeyModifiers::NONE,
            false,
            KeyLocation::Standard,
            KeyboardCompositionState::Inactive,
            None,
        ))
        .unwrap_or_else(|error| unreachable!("complete input root fits: {error}"));
        app.pump(queue_only).unwrap_or_else(|_| unreachable!());
        assert!(!app.state().enabled);
        assert_eq!(&*callbacks.borrow(), &["down", "out"]);
        if same_key {
            assert_eq!(
                app.status(),
                RuntimeStatus::Running,
                "an already funded same-key replacement is not ordinary overload"
            );
            assert!(app.focus().focused_node().is_none());
            assert_eq!(app.index().nodes()[0].id(), &target);
        } else {
            assert_eq!(
                app.status(),
                RuntimeStatus::Terminal(RuntimeTerminalReason::Poisoned),
                "an undeclared second resident is an integrity violation even with spare global capacity"
            );
            if trace != 0 {
                assert_eq!(
                    app.trace()
                        .records()
                        .filter(|record| {
                            matches!(record.kind(), TraceRecordKind::WorkRequested)
                                && record
                                    .work()
                                    .is_some_and(|work| work.family() == TraceWorkFamily::LocalTask)
                        })
                        .count(),
                    1,
                    "the unqualified replacement must not be committed"
                );
            }
        }
    }
}
