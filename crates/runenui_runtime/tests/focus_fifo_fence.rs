#![allow(refining_impl_trait)]

use std::{cell::RefCell, rc::Rc};

use runenui_core::{
    CommandOrigin, CommittedTextEvent, Element, ElementId, EventContext, EventPhase, InputDeviceId,
    KeyLocation, KeyModifiers, KeyboardCompositionState, KeyboardEvent, KeyboardPhase, LogicalKey,
    NoHostProtocol, PhysicalKey, SemanticCommand, UiApp, UiEvent, View, Widget, WidgetActivation,
    WidgetEventOutput, WidgetTextInput, children, row,
};
use runenui_runtime::{
    AppRuntime, InputArbitrationRecord, InputPumpBatch, MountedNodeId, PumpBudget, RuntimeConfig,
    RuntimeStatus, TraceConfig, UiInputFinality, UiInputProcessingRejection,
};

const FULL: PumpBudget = PumpBudget::new(usize::MAX, usize::MAX, usize::MAX, usize::MAX);
const ONE: PumpBudget = PumpBudget::new(1, 0, 0, 0);
const ZERO: PumpBudget = PumpBudget::new(0, 0, 0, 0);

#[derive(Debug, Clone, Copy)]
enum Action {
    RevokeText,
    RestoreText,
}

#[derive(Debug)]
struct State {
    text_capable: bool,
    log: Rc<RefCell<Vec<(&'static str, &'static str)>>>,
}

#[derive(Debug)]
struct Probe {
    name: &'static str,
    text_capable: bool,
    log: Rc<RefCell<Vec<(&'static str, &'static str)>>>,
}

impl Widget<Action> for Probe {
    type State = ();
    fn create_state(&self) -> Self::State {}

    fn event(
        &mut self,
        (): &mut (),
        event: &UiEvent,
        context: &mut EventContext<'_, Action>,
    ) -> WidgetEventOutput {
        if context.phase() == EventPhase::Target {
            let kind = match event {
                UiEvent::Keyboard(_) => Some("key"),
                UiEvent::CommittedText(_) => Some("text"),
                _ => None,
            };
            if let Some(kind) = kind {
                self.log.borrow_mut().push((self.name, kind));
            }
        }
        WidgetEventOutput::none()
    }

    fn activation(&self, (): &()) -> WidgetActivation {
        WidgetActivation::actionable(true)
    }

    fn text_input(&self, (): &()) -> WidgetTextInput {
        WidgetTextInput::new(self.text_capable, self.text_capable)
    }
}

struct App;
impl UiApp for App {
    type State = State;
    type Action = Action;
    type HostProtocol = NoHostProtocol;

    fn root(state: &State) -> Element<Action> {
        row(children![
            Element::new(Probe {
                name: "a",
                text_capable: state.text_capable,
                log: Rc::clone(&state.log),
            })
            .id("a")
            .key("a")
            .focusable(true),
            Element::new(Probe {
                name: "b",
                text_capable: false,
                log: Rc::clone(&state.log),
            })
            .id("b")
            .key("b")
            .focusable(true),
        ])
        .key("root")
        .into_element()
    }

    fn update(state: &mut State, action: Action) {
        state.text_capable = matches!(action, Action::RestoreText);
    }
}

fn mount(trace_capacity: usize) -> AppRuntime<App> {
    let log = Rc::new(RefCell::new(Vec::new()));
    let mut runtime = AppRuntime::<App>::mount_with_config(
        State {
            text_capable: true,
            log,
        },
        RuntimeConfig::default().with_trace_config(TraceConfig::new(trace_capacity)),
    );
    runtime.pump(FULL).unwrap_or_else(|_| unreachable!("mount"));
    runtime
}

fn id(runtime: &mut AppRuntime<App>, name: &str) -> MountedNodeId {
    let authored = ElementId::new(name).unwrap_or_else(|_| unreachable!("fixture id"));
    runtime
        .index()
        .nodes()
        .iter()
        .find(|node| node.authored_id() == Some(&authored))
        .unwrap_or_else(|| unreachable!("mounted fixture"))
        .id()
        .clone()
}

fn request_focus(runtime: &mut AppRuntime<App>, name: &str) {
    let target = id(runtime, name);
    runtime
        .submit_command(
            target,
            SemanticCommand::RequestFocus,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("focus admission"));
}

fn focused_a(trace_capacity: usize) -> AppRuntime<App> {
    let mut runtime = mount(trace_capacity);
    request_focus(&mut runtime, "a");
    runtime.pump(FULL).unwrap_or_else(|_| unreachable!("focus"));
    runtime.state().log.borrow_mut().clear();
    runtime
}

fn keyboard(device: u64) -> KeyboardEvent {
    KeyboardEvent::new(
        KeyboardPhase::Down,
        PhysicalKey::Code(String::from("KeyQ")),
        LogicalKey::Character(String::from("q")),
        KeyModifiers::NONE,
        false,
        KeyLocation::Standard,
        KeyboardCompositionState::Inactive,
        Some(InputDeviceId::new(device).unwrap_or_else(|| unreachable!("device"))),
    )
}

fn settlements(batch: &InputPumpBatch) -> Vec<&runenui_runtime::UiInputSettlement> {
    batch
        .ordered_records()
        .iter()
        .filter_map(|record| match record {
            InputArbitrationRecord::InputSettled(settlement) => Some(settlement),
            _ => None,
        })
        .collect()
}

#[test]
fn focus_transfer_rejects_newer_waiting_input_once_and_never_delivers_to_b() {
    for trace in [0, 1024] {
        let mut runtime = focused_a(trace);
        let initial_scope = runtime
            .input_ownership()
            .unwrap_or_else(|_| unreachable!())
            .scope()
            .clone();
        request_focus(&mut runtime, "b");
        let stale = runtime
            .submit_keyboard(keyboard(1))
            .unwrap_or_else(|_| unreachable!("accepted A"));
        assert!(settlements(&runtime.pump(ZERO).unwrap_or_else(|_| unreachable!())).is_empty());
        let transfer = runtime
            .pump(ONE)
            .unwrap_or_else(|_| unreachable!("focus transition"));
        assert!(settlements(&transfer).is_empty());
        let revision = transfer.final_ownership().revision();
        let settled = runtime
            .pump(ONE)
            .unwrap_or_else(|_| unreachable!("stale input"));
        let entries = settlements(&settled);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].sequence(), stale.sequence());
        assert_eq!(entries[0].scope(), &initial_scope);
        assert_eq!(entries[0].ownership_revision(), revision);
        assert_eq!(
            entries[0].finality(),
            &UiInputFinality::ProcessingRejected(UiInputProcessingRejection::StaleFocusLifetime)
        );
        assert!(runtime.state().log.borrow().is_empty());
        assert_eq!(runtime.status(), RuntimeStatus::Running);
        let fresh = runtime
            .submit_keyboard(keyboard(2))
            .unwrap_or_else(|_| unreachable!("fresh B"));
        let result = runtime
            .pump(FULL)
            .unwrap_or_else(|_| unreachable!("fresh B dispatch"));
        assert_eq!(settlements(&result).len(), 1);
        assert_eq!(settlements(&result)[0].sequence(), fresh.sequence());
        assert!(matches!(
            settlements(&result)[0].finality(),
            UiInputFinality::Committed(_)
        ));
        assert_eq!(&*runtime.state().log.borrow(), &[("b", "key")]);
        assert!(settlements(&runtime.pump(FULL).unwrap_or_else(|_| unreachable!())).is_empty());
        if trace == 0 {
            assert_eq!(runtime.trace().len(), 0);
        }
    }
}

#[test]
fn same_mounted_a_b_a_reentry_rejects_old_keyboard_and_text_but_accepts_fresh() {
    for trace in [0, 1024] {
        let mut runtime = focused_a(trace);
        let original = id(&mut runtime, "a");
        request_focus(&mut runtime, "b");
        request_focus(&mut runtime, "a");
        let old_key = runtime
            .submit_keyboard(keyboard(1))
            .unwrap_or_else(|_| unreachable!());
        let old_text = runtime
            .submit_text(CommittedTextEvent::new("secret", None).unwrap_or_else(|_| unreachable!("valid fixture text")))
            .unwrap_or_else(|_| unreachable!());
        runtime
            .pump(PumpBudget::new(2, 0, 0, 0))
            .unwrap_or_else(|_| unreachable!("two transfers"));
        assert_eq!(runtime.focus().focused_node(), Some(&original));
        assert_eq!(id(&mut runtime, "a"), original);
        let new_key = runtime
            .submit_keyboard(keyboard(2))
            .unwrap_or_else(|_| unreachable!());
        let new_text = runtime
            .submit_text(CommittedTextEvent::new("fresh", None).unwrap_or_else(|_| unreachable!("valid fixture text")))
            .unwrap_or_else(|_| unreachable!());
        let results = runtime
            .pump(FULL)
            .unwrap_or_else(|_| unreachable!("input dispatch"));
        let entries = settlements(&results);
        assert_eq!(entries.len(), 4);
        for (index, sequence) in [old_key.sequence(), old_text.sequence()].iter().enumerate() {
            assert_eq!(entries[index].sequence(), *sequence);
            assert_eq!(
                entries[index].finality(),
                &UiInputFinality::ProcessingRejected(
                    UiInputProcessingRejection::StaleFocusLifetime
                )
            );
        }
        for (index, sequence) in [new_key.sequence(), new_text.sequence()].iter().enumerate() {
            assert_eq!(entries[index + 2].sequence(), *sequence);
            assert!(matches!(
                entries[index + 2].finality(),
                UiInputFinality::Committed(_)
            ));
        }
        assert_eq!(
            &*runtime.state().log.borrow(),
            &[("a", "key"), ("a", "text")]
        );
    }
}

#[test]
fn unchanged_focused_recipient_losing_text_capability_rejects_at_dispatch() {
    for trace in [0, 1024] {
        let mut runtime = focused_a(trace);
        let owner = id(&mut runtime, "a");
        runtime
            .submit_action(Action::RevokeText)
            .unwrap_or_else(|_| unreachable!());
        let text = runtime
            .submit_text(CommittedTextEvent::new("private", None).unwrap_or_else(|_| unreachable!("valid fixture text")))
            .unwrap_or_else(|_| unreachable!());
        let result = runtime.pump(FULL).unwrap_or_else(|_| unreachable!());
        assert_eq!(runtime.focus().focused_node(), Some(&owner));
        let entries = settlements(&result);
        assert_eq!(entries.len(), 1);
        assert_eq!(entries[0].sequence(), text.sequence());
        assert_eq!(
            entries[0].finality(),
            &UiInputFinality::ProcessingRejected(
                UiInputProcessingRejection::CommittedTextRecipientUnavailable
            )
        );
        assert!(runtime.state().log.borrow().is_empty());
        runtime
            .submit_action(Action::RestoreText)
            .unwrap_or_else(|_| unreachable!());
        runtime.pump(FULL).unwrap_or_else(|_| unreachable!());
        let restored = runtime
            .submit_text(CommittedTextEvent::new("valid", None).unwrap_or_else(|_| unreachable!("valid fixture text")))
            .unwrap_or_else(|_| unreachable!());
        let result = runtime.pump(FULL).unwrap_or_else(|_| unreachable!());
        assert_eq!(settlements(&result).len(), 1);
        assert_eq!(settlements(&result)[0].sequence(), restored.sequence());
        assert!(matches!(
            settlements(&result)[0].finality(),
            UiInputFinality::Committed(_)
        ));
        assert_eq!(&*runtime.state().log.borrow(), &[("a", "text")]);
    }
}

#[test]
fn same_owner_noop_focus_request_does_not_invalidate_following_keyboard() {
    let mut runtime = focused_a(0);
    request_focus(&mut runtime, "a");
    let receipt = runtime
        .submit_keyboard(keyboard(1))
        .unwrap_or_else(|_| unreachable!());
    let result = runtime.pump(FULL).unwrap_or_else(|_| unreachable!());
    assert_eq!(settlements(&result).len(), 1);
    assert_eq!(settlements(&result)[0].sequence(), receipt.sequence());
    assert!(matches!(
        settlements(&result)[0].finality(),
        UiInputFinality::Committed(_)
    ));
    assert_eq!(&*runtime.state().log.borrow(), &[("a", "key")]);
}
