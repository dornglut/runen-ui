use core::num::NonZeroUsize;

use runenui_core::{
    Axis, ElementId, FocusGroupBoundaryPolicy, Focusability, KeyLocation, KeyModifiers,
    KeyboardCompositionState, KeyboardEvent, KeyboardPhase, LogicalKey, NoHostProtocol,
    PhysicalKey, SemanticCommand, SemanticOrientation, SemanticRole, UiApp, View, button, column,
    radio_button, radio_group, toolbar,
};
use runenui_runtime::PumpBudget;
use runenui_testing::{SemanticQuery, SettleBudget, SettleOutcome, TestHarness};

#[derive(Clone, Copy, Debug)]
enum Action {
    Activate(&'static str),
    SelectRadio,
    Configure(Axis, FocusGroupBoundaryPolicy, bool),
}

#[derive(Clone, Debug)]
struct State {
    orientation: Axis,
    boundary: FocusGroupBoundaryPolicy,
    disabled_discoverable: bool,
    radio_selected: bool,
    activations: Vec<&'static str>,
}

struct App;

impl UiApp for App {
    type State = State;
    type Action = Action;
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> impl View<Self::Action> {
        let disabled = button("Disabled")
            .id("toolbar.disabled")
            .disabled()
            .on_activate(|| Action::Activate("Disabled"))
            .into_element()
            .with_focusability(if state.disabled_discoverable {
                Focusability::FocusableWhenDisabled
            } else {
                Focusability::Automatic
            });
        column([
            button("Before").id("before").on_activate(|| Action::Activate("Before")).into_element(),
            toolbar([
                button("Open").id("toolbar.open").on_activate(|| Action::Activate("Open")).into_element(),
                disabled,
                radio_group([
                    radio_button("Mode A", state.radio_selected)
                        .id("toolbar.radio.a")
                        .on_activate(|| Action::SelectRadio),
                    radio_button("Mode B", !state.radio_selected)
                        .id("toolbar.radio.b")
                        .on_activate(|| Action::SelectRadio),
                ])
                .id("toolbar.radios")
                .standalone_navigation(false)
                .into_element(),
                button("Save").id("toolbar.save").on_activate(|| Action::Activate("Save")).into_element(),
            ])
            .id("toolbar")
            .accessible_name("Editor tools")
            .orientation(state.orientation)
            .boundary(state.boundary)
            .into_element(),
            button("After").id("after").on_activate(|| Action::Activate("After")).into_element(),
        ])
    }

    fn update(
        state: &mut Self::State,
        action: Self::Action,
    ) -> impl runenui_core::IntoUpdateOutput<Self::Action, Self::HostProtocol> {
        match action {
            Action::Activate(label) => state.activations.push(label),
            Action::SelectRadio => state.radio_selected = !state.radio_selected,
            Action::Configure(orientation, boundary, discoverable) => {
                state.orientation = orientation;
                state.boundary = boundary;
                state.disabled_discoverable = discoverable;
            }
        }
    }
}

fn state() -> State {
    State {
        orientation: Axis::Horizontal,
        boundary: FocusGroupBoundaryPolicy::Stop,
        disabled_discoverable: false,
        radio_selected: true,
        activations: Vec::new(),
    }
}

fn budget() -> SettleBudget {
    SettleBudget::new(
        NonZeroUsize::new(16).unwrap_or(NonZeroUsize::MIN),
        PumpBudget::new(64, 64, 64, 64),
    )
}

fn settle(harness: &mut TestHarness<App>) {
    assert_eq!(harness.run_until_idle(budget()).outcome(), SettleOutcome::Idle);
}

fn command(harness: &mut TestHarness<App>, id: &str, command: SemanticCommand) {
    harness.submit_automation_command(
        ElementId::new(id).unwrap_or_else(|_| unreachable!("static ID valid")),
        command,
    ).unwrap_or_else(|error| unreachable!("command submitted: {error:?}"));
    settle(harness);
    assert!(harness.publish().is_ok());
}

fn focus_name(h: &TestHarness<App>, label: &str) {
    let target = h.unique_semantic_target(
        &SemanticQuery::new().with_name(label),
    ).unwrap_or_else(|error| unreachable!("semantic named focus target: {error:?}"));
    assert_eq!(h.semantic_snapshot().unwrap_or_else(|_| unreachable!("published semantic snapshot")).focused(), Some(target.node_id()));
}

fn key(physical: PhysicalKey, logical: LogicalKey) -> KeyboardEvent {
    KeyboardEvent::new(
        KeyboardPhase::Down,
        physical,
        logical,
        KeyModifiers::NONE,
        false,
        KeyLocation::Standard,
        KeyboardCompositionState::Inactive,
        None,
    )
}

#[test]
fn toolbar_is_one_external_stop_with_manual_real_focus_and_stop_boundary() {
    let mut h = TestHarness::<App>::mount(state());
    assert!(h.publish().is_ok());
    let node = h.semantic_snapshot().unwrap_or_else(|_| unreachable!("published semantic snapshot")).nodes().iter()
        .find(|n| n.role() == SemanticRole::Toolbar)
        .unwrap_or_else(|| unreachable!("toolbar semantics"));
    assert_eq!(node.name(), Some("Editor tools"));
    assert_eq!(node.orientation(), Some(SemanticOrientation::Horizontal));
    assert!(node.supported_actions().is_empty());

    command(&mut h, "before", SemanticCommand::RequestFocus);
    command(&mut h, "before", SemanticCommand::FocusNext);
    focus_name(&h, "Open");
    command(&mut h, "toolbar.open", SemanticCommand::FocusRight);
    focus_name(&h, "Mode A");
    assert!(h.state().activations.is_empty());
    assert!(h.state().radio_selected);

    command(&mut h, "toolbar.radio.a", SemanticCommand::FocusRight);
    focus_name(&h, "Save");
    command(&mut h, "toolbar.save", SemanticCommand::FocusRight);
    focus_name(&h, "Save");
    assert!(h.state().activations.is_empty());
    command(&mut h, "toolbar.save", SemanticCommand::FocusNext);
    focus_name(&h, "After");
    command(&mut h, "after", SemanticCommand::FocusPrevious);
    focus_name(&h, "Open");
    assert!(h.state().activations.is_empty());
}

#[test]
fn toolbar_vertical_wrap_home_end_and_disabled_discoverability() {
    let mut h = TestHarness::<App>::mount(state());
    assert!(h.publish().is_ok());
    assert!(h.submit_action(Action::Configure(
        Axis::Vertical,
        FocusGroupBoundaryPolicy::Wrap,
        true,
    )).is_ok());
    settle(&mut h);
    assert!(h.publish().is_ok());
    let node = h.semantic_snapshot().unwrap_or_else(|_| unreachable!("published semantic snapshot")).nodes().iter()
        .find(|n| n.role() == SemanticRole::Toolbar)
        .unwrap_or_else(|| unreachable!("toolbar semantics"));
    assert_eq!(node.orientation(), Some(SemanticOrientation::Vertical));

    command(&mut h, "before", SemanticCommand::RequestFocus);
    command(&mut h, "before", SemanticCommand::FocusNext);
    focus_name(&h, "Open");
    command(&mut h, "toolbar.open", SemanticCommand::FocusDown);
    focus_name(&h, "Disabled");
    let disabled = h.unique_semantic_target(
        &SemanticQuery::new().with_role(SemanticRole::Button).with_name("Disabled")
    ).unwrap_or_else(|error| unreachable!("disabled target: {error:?}"));
    assert!(h.submit_semantic_action(&disabled, runenui_core::SemanticAction::Activate).is_err());
    command(&mut h, "toolbar.disabled", SemanticCommand::FocusDown);
    focus_name(&h, "Mode A");
    assert!(h.state().radio_selected);
    command(&mut h, "toolbar.radio.a", SemanticCommand::FocusDown);
    focus_name(&h, "Save");
    command(&mut h, "toolbar.save", SemanticCommand::FocusDown);
    focus_name(&h, "Open");

    h.submit_keyboard(key(PhysicalKey::End, LogicalKey::End))
        .unwrap_or_else(|error| unreachable!("End admitted: {error:?}"));
    settle(&mut h);
    assert!(h.publish().is_ok());
    focus_name(&h, "Save");
    h.submit_keyboard(key(PhysicalKey::Home, LogicalKey::Home))
        .unwrap_or_else(|error| unreachable!("Home admitted: {error:?}"));
    settle(&mut h);
    assert!(h.publish().is_ok());
    focus_name(&h, "Open");
    assert!(h.state().activations.is_empty());
}

#[test]
fn explicit_activation_does_not_transfer_selection_or_activate_on_navigation() {
    let mut h = TestHarness::<App>::mount(state());
    assert!(h.publish().is_ok());
    command(&mut h, "before", SemanticCommand::RequestFocus);
    command(&mut h, "before", SemanticCommand::FocusNext);
    command(&mut h, "toolbar.open", SemanticCommand::FocusRight);
    assert_eq!(h.state().radio_selected, true);
    assert!(h.state().activations.is_empty());
    command(&mut h, "toolbar.radio.a", SemanticCommand::Activate);
    assert_eq!(h.state().radio_selected, false);
    assert!(h.state().activations.is_empty());
    command(&mut h, "toolbar.save", SemanticCommand::Activate);
    assert_eq!(h.state().activations, vec!["Save"]);
}
