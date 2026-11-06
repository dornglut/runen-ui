#![allow(refining_impl_trait)]

//! Independent public #474 discriminators for the existing queued presentation entry.
use runenui_core::{
    CommandOrigin, CommittedTextEvent, Element, KeyLocation, KeyModifiers,
    KeyboardCompositionState, KeyboardEvent, KeyboardPhase, LogicalKey, Menu, MenuButton, MenuItem,
    NoHostProtocol, PhysicalKey, SemanticCommand, StyleEnvironment, UiApp, View, Widget,
    WidgetTextInput, column,
};
use runenui_runtime::{
    AppRuntime, InputArbitrationRecord, LogicalSize, PumpBudget, SubmitTextErrorKind,
    SurfaceBuildContext, UiInputFinality, UiInputProcessingRejection, UiInputRoute,
};

const FULL: PumpBudget = PumpBudget::new(128, 128, 128, 128);
const ONE: PumpBudget = PumpBudget::new(1, 0, 0, 0);

#[derive(Clone, Copy, Debug)]
enum Action {
    ShowP,
    HideP,
    Noop,
}

#[derive(Clone, Debug)]
struct State {
    p_open: bool,
}

#[derive(Debug)]
struct TextProbe;

impl Widget<Action> for TextProbe {
    fn producer_budget(
        &self,
        phase: runenui_core::WidgetProducerPhase,
        _state: &Self::State,
    ) -> runenui_core::WidgetProducerBudget {
        match phase {
            runenui_core::WidgetProducerPhase::Activation => {
                runenui_core::WidgetProducerBudget::new(0)
            }
            runenui_core::WidgetProducerPhase::Subscriptions => {
                runenui_core::WidgetProducerBudget::new(0).with_subscriptions(0)
            }
            _ => runenui_core::WidgetProducerBudget::new(0),
        }
    }

    type State = ();
    fn create_state(&self) -> Self::State {}
    fn text_input(&self, (): &()) -> WidgetTextInput {
        WidgetTextInput::new(true, true)
    }
}

struct App;
impl UiApp for App {
    fn initial_producer_budget(state: &Self::State) -> runenui_core::ProducerBudget {
        let initial_nodes = if state.p_open { 5 } else { 3 };
        runenui_core::ProducerBudget::new(initial_nodes, 0)
            .with_application_outputs(0)
            .with_reconciliation(initial_nodes, 0, initial_nodes)
            .with_subscriptions(0, 0)
            .with_work_starts(runenui_core::WorkStartBudget::new(0, 0, 0, 0, 0))
    }

    fn action_producer_budget(
        state: &Self::State,
        action: &Self::Action,
    ) -> runenui_core::ProducerBudget {
        let next_open = match action {
            Action::ShowP => true,
            Action::HideP => false,
            Action::Noop => state.p_open,
        };
        let changed = state.p_open != next_open;
        let old_nodes: usize = if state.p_open { 5 } else { 3 };
        let new_nodes: usize = if next_open { 5 } else { 3 };
        let mounted_nodes = new_nodes.saturating_sub(old_nodes);
        let retained_updates = if changed { old_nodes.min(new_nodes) } else { 0 };
        let dirty_owners = if changed { new_nodes } else { 0 };
        runenui_core::ProducerBudget::new((dirty_owners + usize::from(changed)).max(1), 0)
            .with_application_outputs(0)
            .with_reconciliation(mounted_nodes, retained_updates, dirty_owners)
            .with_subscriptions(0, 0)
            .with_work_starts(runenui_core::WorkStartBudget::new(0, 0, 0, 0, 0))
    }

    fn interaction_producer_budget(state: &Self::State) -> runenui_core::ProducerBudget {
        // Focus handoff can notify the trigger, menu container, and menu item
        // four times each; the deepest ordinary route adds nine more outputs.
        let outputs = if state.p_open { 29 } else { 5 };
        runenui_core::ProducerBudget::new(outputs, outputs).with_application_outputs(0)
    }

    type State = State;
    type Action = Action;
    type HostProtocol = NoHostProtocol;

    fn root(state: &State) -> impl View<Action> {
        let mut trigger = MenuButton::new("P", state.p_open).id("p-trigger");
        if state.p_open {
            trigger = trigger.with_submenu(
                Menu::new(vec![
                    MenuItem::new("item")
                        .id("p-item")
                        .on_activate(|| Action::Noop)
                        .into_element(),
                ])
                .id("p-menu"),
                true,
            );
        }
        column(vec![
            Element::new(TextProbe).id("text").focusable(true),
            trigger.into_element(),
        ])
    }

    fn update(state: &mut State, action: Action) {
        match action {
            Action::ShowP => state.p_open = true,
            Action::HideP => state.p_open = false,
            Action::Noop => {}
        }
    }
}

fn environment() -> (StyleEnvironment, LogicalSize) {
    (
        StyleEnvironment::default(),
        LogicalSize::new(320_u16.into(), 240_u16.into()),
    )
}

fn id(runtime: &mut AppRuntime<App>, name: &str) -> runenui_runtime::MountedNodeId {
    runtime
        .index()
        .nodes()
        .iter()
        .find(|node| node.authored_id().is_some_and(|id| id.as_str() == name))
        .unwrap_or_else(|| unreachable!("mounted fixture recipient"))
        .id()
        .clone()
}

fn focus(runtime: &mut AppRuntime<App>, name: &str) {
    let target = id(runtime, name);
    runtime
        .submit_command(
            target,
            SemanticCommand::RequestFocus,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("focus request"));
    runtime
        .pump(FULL)
        .unwrap_or_else(|_| unreachable!("focus processed"));
}

fn key() -> KeyboardEvent {
    KeyboardEvent::new(
        KeyboardPhase::Down,
        PhysicalKey::Code(String::from("KeyJ")),
        LogicalKey::Character(String::from("j")),
        KeyModifiers::NONE,
        false,
        KeyLocation::Standard,
        KeyboardCompositionState::Inactive,
        None,
    )
}

#[test]
fn pending_published_entry_binds_non_escape_to_exact_recipient_with_one_m13_settlement() {
    let (env, size) = environment();
    let context = SurfaceBuildContext::tight(&env, size);
    let mut runtime = AppRuntime::<App>::mount(State { p_open: false })
        .unwrap_or_else(|error| unreachable!("fixture initial admission must succeed: {error}"));
    runtime
        .publish_surface(&context)
        .unwrap_or_else(|_| unreachable!());
    runtime.pump(FULL).unwrap_or_else(|_| unreachable!());
    focus(&mut runtime, "p-trigger");
    runtime
        .submit_action(Action::ShowP)
        .unwrap_or_else(|_| unreachable!());
    runtime.pump(FULL).unwrap_or_else(|_| unreachable!());
    runtime
        .publish_surface(&context)
        .unwrap_or_else(|_| unreachable!("entry publication"));
    let old = id(&mut runtime, "p-trigger");
    let new = id(&mut runtime, "p-item");
    assert_eq!(runtime.focus().focused_node(), Some(&old));
    let receipt = runtime
        .submit_keyboard(key())
        .unwrap_or_else(|_| unreachable!("input admitted"));
    let scope = runtime
        .input_ownership()
        .unwrap_or_else(|_| unreachable!())
        .scope()
        .clone();
    let zero = runtime
        .pump(PumpBudget::new(0, 0, 0, 0))
        .unwrap_or_else(|_| unreachable!());
    assert!(
        !zero
            .ordered_records()
            .iter()
            .any(|r| matches!(r, InputArbitrationRecord::InputSettled(_)))
    );
    runtime
        .pump(ONE)
        .unwrap_or_else(|_| unreachable!("queued focus entry"));
    assert_eq!(runtime.focus().focused_node(), Some(&new));
    let reached = runtime
        .pump(ONE)
        .unwrap_or_else(|_| unreachable!("keyboard"));
    let entries: Vec<_> = reached
        .ordered_records()
        .iter()
        .filter_map(|record| {
            if let InputArbitrationRecord::InputSettled(settled) = record {
                Some(settled)
            } else {
                None
            }
        })
        .collect();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].sequence(), receipt.sequence());
    assert_eq!(entries[0].scope(), &scope);
    assert!(matches!(
        entries[0].finality(),
        UiInputFinality::Committed(facts)
            if matches!(facts.route(), UiInputRoute::Routed { target } if target == &new)
    ));
    let tail = runtime.pump(FULL).unwrap_or_else(|_| unreachable!());
    assert!(
        !tail
            .ordered_records()
            .iter()
            .any(|r| matches!(r, InputArbitrationRecord::InputSettled(_)))
    );
    assert!(runtime.state().p_open);
}

#[test]
fn pending_entry_never_transfers_ambiguous_committed_text() {
    let (env, size) = environment();
    let context = SurfaceBuildContext::tight(&env, size);
    let mut runtime = AppRuntime::<App>::mount(State { p_open: false })
        .unwrap_or_else(|error| unreachable!("fixture initial admission must succeed: {error}"));
    runtime
        .publish_surface(&context)
        .unwrap_or_else(|_| unreachable!());
    runtime.pump(FULL).unwrap_or_else(|_| unreachable!());
    focus(&mut runtime, "text");
    runtime
        .submit_action(Action::ShowP)
        .unwrap_or_else(|_| unreachable!());
    runtime.pump(FULL).unwrap_or_else(|_| unreachable!());
    runtime
        .publish_surface(&context)
        .unwrap_or_else(|_| unreachable!());
    let error = runtime
        .submit_text(CommittedTextEvent::new("secret", None).unwrap_or_else(|_| unreachable!()))
        .err()
        .unwrap_or_else(|| {
            unreachable!("an ambiguous pending recipient must reject before queueing")
        });
    assert_eq!(error.kind(), SubmitTextErrorKind::PendingFocusHandoff);
    let original = id(&mut runtime, "text");
    assert_eq!(runtime.focus().focused_node(), Some(&original));
    let entry = runtime.pump(ONE).unwrap_or_else(|_| unreachable!());
    assert!(
        !entry
            .ordered_records()
            .iter()
            .any(|r| matches!(r, InputArbitrationRecord::InputSettled(_)))
    );
}

#[test]
fn retired_pending_presentation_entry_rejects_without_retarged_keyboard() {
    let (env, size) = environment();
    let context = SurfaceBuildContext::tight(&env, size);
    let mut runtime = AppRuntime::<App>::mount(State { p_open: false })
        .unwrap_or_else(|error| unreachable!("fixture initial admission must succeed: {error}"));
    runtime
        .publish_surface(&context)
        .unwrap_or_else(|_| unreachable!());
    runtime.pump(FULL).unwrap_or_else(|_| unreachable!());
    focus(&mut runtime, "p-trigger");
    runtime
        .submit_action(Action::ShowP)
        .unwrap_or_else(|_| unreachable!());
    runtime.pump(FULL).unwrap_or_else(|_| unreachable!());
    runtime
        .submit_action(Action::HideP)
        .unwrap_or_else(|_| unreachable!());
    runtime
        .publish_surface(&context)
        .unwrap_or_else(|_| unreachable!());
    let receipt = runtime
        .submit_keyboard(key())
        .unwrap_or_else(|_| unreachable!());
    let results = runtime.pump(FULL).unwrap_or_else(|_| unreachable!());
    let settled: Vec<_> = results
        .ordered_records()
        .iter()
        .filter_map(|r| {
            if let InputArbitrationRecord::InputSettled(v) = r {
                Some(v)
            } else {
                None
            }
        })
        .collect();
    assert_eq!(settled.len(), 1);
    assert_eq!(settled[0].sequence(), receipt.sequence());
    assert!(matches!(
        settled[0].finality(),
        UiInputFinality::ProcessingRejected(
            UiInputProcessingRejection::FocusBindingInvalidated
                | UiInputProcessingRejection::StaleTarget
                | UiInputProcessingRejection::MissingTarget
        )
    ));
    assert!(!runtime.state().p_open);
}

#[test]
fn paired_keyboard_text_refuses_unknown_pending_presentation_without_partial_ingress() {
    use runenui_runtime::SubmitKeyboardWithTextErrorKind;

    let (env, size) = environment();
    let context = SurfaceBuildContext::tight(&env, size);
    let mut runtime = AppRuntime::<App>::mount(State { p_open: true })
        .unwrap_or_else(|error| unreachable!("initial admission: {error}"));
    focus(&mut runtime, "text");
    runtime
        .publish_surface(&context)
        .unwrap_or_else(|_| unreachable!("pending presentation entry"));
    let text = CommittedTextEvent::new("é", None)
        .unwrap_or_else(|_| unreachable!("original Unicode text"));
    let Err(error) = runtime.submit_keyboard_with_text(key(), text) else {
        unreachable!("paired original cannot mix current and pending focus witnesses")
    };
    assert!(matches!(
        error.kind(),
        SubmitKeyboardWithTextErrorKind::Text(SubmitTextErrorKind::PendingFocusHandoff)
    ));
    assert_eq!(error.text().text(), "é");
    let settled = runtime
        .pump(FULL)
        .unwrap_or_else(|_| unreachable!("canonical pump"));
    assert!(
        !settled
            .ordered_records()
            .iter()
            .any(|record| matches!(record, InputArbitrationRecord::InputSettled(_))),
        "a refused pair must create no keyboard or text terminal receipt"
    );
}

#[test]
fn exact_noop_presentation_entry_certifies_the_unchanged_focused_recipient() {
    let (env, size) = environment();
    let context = SurfaceBuildContext::tight(&env, size);
    let mut runtime = AppRuntime::<App>::mount(State { p_open: true })
        .unwrap_or_else(|error| unreachable!("fixture initial admission must succeed: {error}"));
    // An eligible mounted recipient can already hold focus before the
    // first interaction publication schedules EnterAndRestore for it.
    focus(&mut runtime, "p-item");
    let original = id(&mut runtime, "p-item");
    assert_eq!(runtime.focus().focused_node(), Some(&original));
    runtime
        .publish_surface(&context)
        .unwrap_or_else(|_| unreachable!("known P publication"));
    let receipt = runtime
        .submit_keyboard(key())
        .unwrap_or_else(|_| unreachable!("certified pending no-op ingress"));
    runtime
        .pump(ONE)
        .unwrap_or_else(|_| unreachable!("queued no-op entry"));
    assert_eq!(runtime.focus().focused_node(), Some(&original));
    let reached = runtime
        .pump(ONE)
        .unwrap_or_else(|_| unreachable!("no-op-bound keyboard"));
    let records: Vec<_> = reached
        .ordered_records()
        .iter()
        .filter_map(|record| {
            if let InputArbitrationRecord::InputSettled(value) = record {
                Some(value)
            } else {
                None
            }
        })
        .collect();
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].sequence(), receipt.sequence());
    assert!(matches!(
        records[0].finality(),
        UiInputFinality::Committed(facts)
            if matches!(facts.route(), UiInputRoute::Routed { target } if target == &original)
    ));
    assert!(runtime.state().p_open);
}
