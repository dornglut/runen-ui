#![allow(refining_impl_trait)]

//! Independent public #474 discriminators for the existing queued presentation entry.
use runenui_core::{
    CommandOrigin, CommittedTextEvent, Element, KeyLocation, KeyModifiers,
    KeyboardCompositionState, KeyboardEvent, KeyboardPhase, LogicalKey, Menu, MenuButton,
    MenuItem, NoHostProtocol, PhysicalKey, SemanticCommand, StyleEnvironment, UiApp,
    View, Widget, WidgetTextInput, column,
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
    type State = ();
    fn create_state(&self) -> Self::State {}
    fn text_input(&self, (): &()) -> WidgetTextInput {
        WidgetTextInput::new(true, true)
    }
}

struct App;
impl UiApp for App {
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
            Element::new(TextProbe)
                .id("text")
                .focusable(true),
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
    (StyleEnvironment::default(), LogicalSize::new(320_u16.into(), 240_u16.into()))
}

fn id(runtime: &mut AppRuntime<App>, name: &str) -> runenui_runtime::MountedNodeId {
    runtime.index().nodes().iter()
        .find(|node| node.authored_id().is_some_and(|id| id.as_str() == name))
        .unwrap_or_else(|| unreachable!("mounted fixture recipient"))
        .id().clone()
}

fn focus(runtime: &mut AppRuntime<App>, name: &str) {
    let target = id(runtime, name);
    runtime.submit_command(
        target,
        SemanticCommand::RequestFocus,
        CommandOrigin::programmatic(),
    ).unwrap_or_else(|_| unreachable!("focus request"));
    runtime.pump(FULL).unwrap_or_else(|_| unreachable!("focus processed"));
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
    let mut runtime = AppRuntime::<App>::mount(State { p_open: false });
    runtime.publish_surface(&context).unwrap_or_else(|_| unreachable!());
    runtime.pump(FULL).unwrap_or_else(|_| unreachable!());
    focus(&mut runtime, "p-trigger");
    runtime.submit_action(Action::ShowP).unwrap_or_else(|_| unreachable!());
    runtime.pump(FULL).unwrap_or_else(|_| unreachable!());
    runtime.publish_surface(&context).unwrap_or_else(|_| unreachable!("entry publication"));
    let old = id(&mut runtime, "p-trigger");
    let new = id(&mut runtime, "p-item");
    assert_eq!(runtime.focus().focused_node(), Some(&old));
    let receipt = runtime.submit_keyboard(key()).unwrap_or_else(|_| unreachable!("input admitted"));
    let scope = runtime.input_ownership().unwrap_or_else(|_| unreachable!()).scope().clone();
    let zero = runtime.pump(PumpBudget::new(0, 0, 0, 0)).unwrap_or_else(|_| unreachable!());
    assert!(!zero.ordered_records().iter().any(|r| matches!(r, InputArbitrationRecord::InputSettled(_))));
    runtime.pump(ONE).unwrap_or_else(|_| unreachable!("queued focus entry"));
    assert_eq!(runtime.focus().focused_node(), Some(&new));
    let reached = runtime.pump(ONE).unwrap_or_else(|_| unreachable!("keyboard"));
    let entries: Vec<_> = reached.ordered_records().iter().filter_map(|record| {
        if let InputArbitrationRecord::InputSettled(settled) = record { Some(settled) } else { None }
    }).collect();
    assert_eq!(entries.len(), 1);
    assert_eq!(entries[0].sequence(), receipt.sequence());
    assert_eq!(entries[0].scope(), &scope);
    assert!(matches!(
        entries[0].finality(),
        UiInputFinality::Committed(facts)
            if matches!(facts.route(), UiInputRoute::Routed { target } if target == &new)
    ));
    let tail = runtime.pump(FULL).unwrap_or_else(|_| unreachable!());
    assert!(!tail.ordered_records().iter().any(|r| matches!(r, InputArbitrationRecord::InputSettled(_))));
    assert!(runtime.state().p_open);
}

#[test]
fn pending_entry_never_transfers_ambiguous_committed_text() {
    let (env, size) = environment();
    let context = SurfaceBuildContext::tight(&env, size);
    let mut runtime = AppRuntime::<App>::mount(State { p_open: false });
    runtime.publish_surface(&context).unwrap_or_else(|_| unreachable!());
    runtime.pump(FULL).unwrap_or_else(|_| unreachable!());
    focus(&mut runtime, "text");
    runtime.submit_action(Action::ShowP).unwrap_or_else(|_| unreachable!());
    runtime.pump(FULL).unwrap_or_else(|_| unreachable!());
    runtime.publish_surface(&context).unwrap_or_else(|_| unreachable!());
    let error = runtime.submit_text(
        CommittedTextEvent::new("secret", None).unwrap_or_else(|_| unreachable!())
    ).expect_err("a pending different text recipient cannot be inferred");
    assert_eq!(error.kind(), SubmitTextErrorKind::PendingFocusHandoff);
    let original = id(&mut runtime, "text");
    assert_eq!(runtime.focus().focused_node(), Some(&original));
    let entry = runtime.pump(ONE).unwrap_or_else(|_| unreachable!());
    assert!(!entry.ordered_records().iter().any(|r| matches!(r, InputArbitrationRecord::InputSettled(_))));
}

#[test]
fn retired_pending_presentation_entry_rejects_without_retarged_keyboard() {
    let (env, size) = environment();
    let context = SurfaceBuildContext::tight(&env, size);
    let mut runtime = AppRuntime::<App>::mount(State { p_open: false });
    runtime.publish_surface(&context).unwrap_or_else(|_| unreachable!());
    runtime.pump(FULL).unwrap_or_else(|_| unreachable!());
    focus(&mut runtime, "p-trigger");
    runtime.submit_action(Action::ShowP).unwrap_or_else(|_| unreachable!());
    runtime.pump(FULL).unwrap_or_else(|_| unreachable!());
    runtime.submit_action(Action::HideP).unwrap_or_else(|_| unreachable!());
    runtime.publish_surface(&context).unwrap_or_else(|_| unreachable!());
    let receipt = runtime.submit_keyboard(key()).unwrap_or_else(|_| unreachable!());
    let results = runtime.pump(FULL).unwrap_or_else(|_| unreachable!());
    let settled: Vec<_> = results.ordered_records().iter().filter_map(|r| {
        if let InputArbitrationRecord::InputSettled(v) = r { Some(v) } else { None }
    }).collect();
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
