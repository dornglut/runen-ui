use core::num::NonZeroUsize;

use runenui_core::{
    CommandOrigin, ElementId, KeyLocation, KeyModifiers, KeyboardCompositionState, KeyboardEvent,
    KeyboardPhase, LogicalKey, LogicalPoint, NoHostProtocol, PhysicalKey, PointerButton,
    PointerButtons, PointerDeviceKind, PointerId, PointerPhase, SemanticAction,
    SemanticCheckedState, SemanticCommand, SemanticRole, UiApp, View, button, children, column,
    radio_button, radio_group,
};
use runenui_runtime::{PumpBudget, ReconciliationDiagnostic};
use runenui_testing::{SemanticQuery, SettleBudget, SettleOutcome, TestHarness};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Action {
    Select(u8),
    Noop,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct State {
    selected: Option<u8>,
    disable_two: bool,
    hide_two_from_focus: bool,
    activations: Vec<u8>,
}

struct RadioApp;

impl UiApp for RadioApp {
    type State = State;
    type Action = Action;
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> impl View<Self::Action> {
        let one = radio_button("One", state.selected == Some(1))
            .id("radio.one")
            .on_activate(|| Action::Select(1));
        let mut two = radio_button("Two", state.selected == Some(2))
            .id("radio.two")
            .on_activate(|| Action::Select(2));
        if state.disable_two {
            two = two.disabled();
        }
        if state.hide_two_from_focus {
            two = two.focus_hidden(true);
        }
        let three = radio_button("Three", state.selected == Some(3))
            .id("radio.three")
            .on_activate(|| Action::Select(3));

        let group = radio_group([one, two, three]).id("radio.group");
        column(children![
            button("Before").id("before").on_activate(|| Action::Noop),
            group,
            button("After").id("after").on_activate(|| Action::Noop),
        ])
    }

    fn update(state: &mut Self::State, action: Self::Action) {
        match action {
            Action::Select(value) => {
                state.selected = Some(value);
                state.activations.push(value);
            }
            Action::Noop => {}
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ExternallyManagedState {
    selected: u8,
    activations: Vec<u8>,
}

struct ExternallyManagedRadioApp;

impl UiApp for ExternallyManagedRadioApp {
    type State = ExternallyManagedState;
    type Action = Action;
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> impl View<Self::Action> {
        radio_group([
            radio_button("One", state.selected == 1)
                .id("managed.one")
                .on_activate(|| Action::Select(1)),
            radio_button("Two", state.selected == 2)
                .id("managed.two")
                .on_activate(|| Action::Select(2)),
        ])
        .id("managed.group")
        .standalone_navigation(false)
    }

    fn update(state: &mut Self::State, action: Self::Action) {
        if let Action::Select(value) = action {
            state.selected = value;
            state.activations.push(value);
        }
    }
}

struct InvalidRadioApp;

impl UiApp for InvalidRadioApp {
    type State = ();
    type Action = ();
    type HostProtocol = NoHostProtocol;

    fn root((): &Self::State) -> impl View<Self::Action> {
        radio_group([
            radio_button("One", true)
                .id("invalid.one")
                .on_activate(|| ()),
            radio_button("Two", true)
                .id("invalid.two")
                .on_activate(|| ()),
        ])
        .id("invalid.group")
    }

    fn update((): &mut Self::State, (): Self::Action) {}
}

fn settle_budget() -> SettleBudget {
    SettleBudget::new(
        NonZeroUsize::new(16).unwrap_or(NonZeroUsize::MIN),
        PumpBudget::new(64, 64, 64, 64),
    )
}

fn settle(harness: &mut TestHarness<RadioApp>) {
    assert_eq!(
        harness.run_until_idle(settle_budget()).outcome(),
        SettleOutcome::Idle
    );
}

fn element_id(value: &str) -> ElementId {
    ElementId::new(value).unwrap_or_else(|_| unreachable!("test IDs are valid"))
}

const fn arrow_right_down() -> KeyboardEvent {
    KeyboardEvent::new(
        KeyboardPhase::Down,
        PhysicalKey::ArrowRight,
        LogicalKey::ArrowRight,
        KeyModifiers::NONE,
        false,
        KeyLocation::Standard,
        KeyboardCompositionState::Inactive,
        None,
    )
}

const fn space_down() -> KeyboardEvent {
    KeyboardEvent::new(
        KeyboardPhase::Down,
        PhysicalKey::Space,
        LogicalKey::Space,
        KeyModifiers::NONE,
        false,
        KeyLocation::Standard,
        KeyboardCompositionState::Inactive,
        None,
    )
}

const fn space_up() -> KeyboardEvent {
    KeyboardEvent::new(
        KeyboardPhase::Up,
        PhysicalKey::Space,
        LogicalKey::Space,
        KeyModifiers::NONE,
        false,
        KeyLocation::Standard,
        KeyboardCompositionState::Inactive,
        None,
    )
}

fn command(harness: &mut TestHarness<RadioApp>, target: &str, command: SemanticCommand) {
    harness
        .submit_automation_command(element_id(target), command)
        .unwrap_or_else(|error| unreachable!("radio command is accepted: {error:?}"));
    settle(harness);
}

fn radio_query(name: &str, checked: SemanticCheckedState) -> SemanticQuery {
    SemanticQuery::new()
        .with_role(SemanticRole::RadioButton)
        .with_name(name)
        .with_checked(checked)
        .with_supported_action(SemanticAction::Activate)
}

fn assert_focus(harness: &TestHarness<RadioApp>, query: &SemanticQuery) {
    let target = harness
        .unique_semantic_target(query)
        .unwrap_or_else(|error| unreachable!("focused semantic target is unique: {error:?}"));
    let snapshot = harness
        .semantic_snapshot()
        .unwrap_or_else(|_| unreachable!("publication exists"));
    assert_eq!(snapshot.focused(), Some(target.node_id()));
}

#[test]
fn radio_semantics_form_one_group_with_exact_application_checked_state() {
    let mut harness = TestHarness::<RadioApp>::mount(State {
        selected: Some(2),
        disable_two: false,
        hide_two_from_focus: false,
        activations: Vec::new(),
    });
    assert!(harness.publish().is_ok());

    let group = harness
        .unique_semantic_target(&SemanticQuery::new().with_role(SemanticRole::RadioGroup))
        .unwrap_or_else(|error| unreachable!("radio group is unique: {error:?}"));
    let snapshot = harness
        .semantic_snapshot()
        .unwrap_or_else(|_| unreachable!("publication exists"));
    let group_node = snapshot
        .node(group.node_id())
        .unwrap_or_else(|| unreachable!("group target belongs to snapshot"));
    assert_eq!(group_node.children().len(), 3);
    assert!(group_node.children().iter().all(|child| {
        snapshot
            .node(child)
            .is_some_and(|node| node.role() == SemanticRole::RadioButton)
    }));

    assert!(
        harness
            .unique_semantic_target(&radio_query("One", SemanticCheckedState::Unchecked,))
            .is_ok()
    );
    assert!(
        harness
            .unique_semantic_target(&radio_query("Two", SemanticCheckedState::Checked))
            .is_ok()
    );
    assert!(
        harness
            .unique_semantic_target(&radio_query("Three", SemanticCheckedState::Unchecked,))
            .is_ok()
    );
}

#[test]
fn external_entry_prefers_checked_and_no_selection_falls_back_without_selecting() {
    let mut harness = TestHarness::<RadioApp>::mount(State {
        selected: Some(2),
        disable_two: false,
        hide_two_from_focus: false,
        activations: Vec::new(),
    });
    assert!(harness.publish().is_ok());

    command(&mut harness, "before", SemanticCommand::RequestFocus);
    command(&mut harness, "before", SemanticCommand::FocusNext);
    assert!(harness.publish().is_ok());
    assert_focus(&harness, &radio_query("Two", SemanticCheckedState::Checked));
    assert_eq!(harness.state().selected, Some(2));
    assert!(harness.state().activations.is_empty());

    command(&mut harness, "radio.two", SemanticCommand::FocusNext);
    assert!(harness.publish().is_ok());
    assert_focus(
        &harness,
        &SemanticQuery::new()
            .with_role(SemanticRole::Button)
            .with_name("After"),
    );

    command(&mut harness, "after", SemanticCommand::FocusPrevious);
    assert!(harness.publish().is_ok());
    assert_focus(&harness, &radio_query("Two", SemanticCheckedState::Checked));

    command(&mut harness, "radio.two", SemanticCommand::FocusPrevious);
    assert!(harness.publish().is_ok());
    assert_focus(
        &harness,
        &SemanticQuery::new()
            .with_role(SemanticRole::Button)
            .with_name("Before"),
    );

    let mut harness = TestHarness::<RadioApp>::mount(State {
        selected: None,
        disable_two: false,
        hide_two_from_focus: false,
        activations: Vec::new(),
    });
    assert!(harness.publish().is_ok());
    command(&mut harness, "before", SemanticCommand::RequestFocus);
    command(&mut harness, "before", SemanticCommand::FocusNext);
    assert!(harness.publish().is_ok());
    assert_focus(
        &harness,
        &radio_query("One", SemanticCheckedState::Unchecked),
    );
    assert_eq!(harness.state().selected, None);
    assert!(harness.state().activations.is_empty());
}

#[test]
fn directional_radio_navigation_wraps_focus_then_activates_application_state() {
    let mut harness = TestHarness::<RadioApp>::mount(State {
        selected: Some(1),
        disable_two: false,
        hide_two_from_focus: false,
        activations: Vec::new(),
    });
    assert!(harness.publish().is_ok());
    command(&mut harness, "radio.one", SemanticCommand::RequestFocus);

    for (target, command_kind, expected) in [
        ("radio.one", SemanticCommand::FocusRight, 2),
        ("radio.two", SemanticCommand::FocusDown, 3),
        ("radio.three", SemanticCommand::FocusRight, 1),
        ("radio.one", SemanticCommand::FocusLeft, 3),
        ("radio.three", SemanticCommand::FocusUp, 2),
    ] {
        command(&mut harness, target, command_kind);
        assert_eq!(harness.state().selected, Some(expected));
        assert_eq!(harness.state().activations.last(), Some(&expected));
        assert!(harness.publish().is_ok());
        let name = match expected {
            1 => "One",
            2 => "Two",
            3 => "Three",
            _ => unreachable!("radio fixture has three values"),
        };
        assert_focus(&harness, &radio_query(name, SemanticCheckedState::Checked));
    }
    assert_eq!(harness.state().activations, vec![2, 3, 1, 3, 2]);
}

#[test]
fn disabled_radio_is_skipped_and_rejects_semantic_activation() {
    let mut harness = TestHarness::<RadioApp>::mount(State {
        selected: Some(1),
        disable_two: true,
        hide_two_from_focus: false,
        activations: Vec::new(),
    });
    assert!(harness.publish().is_ok());
    command(&mut harness, "radio.one", SemanticCommand::RequestFocus);
    command(&mut harness, "radio.one", SemanticCommand::FocusRight);
    assert_eq!(harness.state().selected, Some(3));
    assert_eq!(harness.state().activations, vec![3]);
    assert!(harness.publish().is_ok());
    assert_focus(
        &harness,
        &radio_query("Three", SemanticCheckedState::Checked),
    );

    let disabled_two = harness
        .unique_semantic_target(
            &radio_query("Two", SemanticCheckedState::Unchecked).with_disabled(true),
        )
        .unwrap_or_else(|error| unreachable!("disabled radio remains semantic: {error:?}"));
    assert!(
        harness
            .submit_semantic_action(&disabled_two, SemanticAction::Activate)
            .is_err()
    );
    settle(&mut harness);
    assert_eq!(harness.state().selected, Some(3));
    assert_eq!(harness.state().activations, vec![3]);

    let mut harness = TestHarness::<RadioApp>::mount(State {
        selected: Some(1),
        disable_two: false,
        hide_two_from_focus: true,
        activations: Vec::new(),
    });
    assert!(harness.publish().is_ok());
    command(&mut harness, "radio.one", SemanticCommand::RequestFocus);
    command(&mut harness, "radio.one", SemanticCommand::FocusRight);
    assert_eq!(harness.state().selected, Some(3));
    assert_eq!(harness.state().activations, vec![3]);
    assert!(harness.publish().is_ok());
    assert_focus(
        &harness,
        &radio_query("Three", SemanticCheckedState::Checked),
    );
}

#[test]
fn raw_keyboard_arrow_and_space_use_the_same_radio_authority() {
    let mut harness = TestHarness::<RadioApp>::mount(State {
        selected: Some(1),
        disable_two: false,
        hide_two_from_focus: false,
        activations: Vec::new(),
    });
    assert!(harness.publish().is_ok());

    command(&mut harness, "radio.one", SemanticCommand::RequestFocus);
    harness
        .submit_keyboard(arrow_right_down())
        .unwrap_or_else(|error| unreachable!("radio ArrowRight is accepted: {error:?}"));
    settle(&mut harness);
    assert_eq!(harness.state().selected, Some(2));
    assert_eq!(harness.state().activations, vec![2]);

    command(&mut harness, "radio.three", SemanticCommand::RequestFocus);
    assert_eq!(harness.state().selected, Some(2));
    harness
        .submit_keyboard(space_down())
        .unwrap_or_else(|error| unreachable!("radio Space down is accepted: {error:?}"));
    settle(&mut harness);
    assert_eq!(harness.state().selected, Some(2));
    assert_eq!(harness.state().activations, vec![2]);

    harness
        .submit_keyboard(space_up())
        .unwrap_or_else(|error| unreachable!("radio Space up is accepted: {error:?}"));
    settle(&mut harness);
    assert_eq!(harness.state().selected, Some(3));
    assert_eq!(harness.state().activations, vec![2, 3]);

    assert!(harness.publish().is_ok());
    assert_focus(
        &harness,
        &radio_query("Three", SemanticCheckedState::Checked),
    );
}

#[test]
fn controller_origin_directional_command_uses_radio_group_navigation() {
    let mut harness = TestHarness::<RadioApp>::mount(State {
        selected: Some(1),
        disable_two: false,
        hide_two_from_focus: false,
        activations: Vec::new(),
    });
    assert!(harness.publish().is_ok());

    command(&mut harness, "radio.one", SemanticCommand::RequestFocus);
    let point = semantic_center(&harness, &radio_query("One", SemanticCheckedState::Checked));
    harness
        .submit_surface_command(
            point,
            SemanticCommand::FocusRight,
            CommandOrigin::controller(),
        )
        .unwrap_or_else(|error| unreachable!("controller radio navigation is accepted: {error:?}"));
    settle(&mut harness);

    assert_eq!(harness.state().selected, Some(2));
    assert_eq!(harness.state().activations, vec![2]);
    assert!(harness.publish().is_ok());
    assert_focus(&harness, &radio_query("Two", SemanticCheckedState::Checked));
}

#[test]
fn semantic_activation_updates_application_selection_before_checked_republication() {
    let mut harness = TestHarness::<RadioApp>::mount(State {
        selected: Some(1),
        disable_two: false,
        hide_two_from_focus: false,
        activations: Vec::new(),
    });
    assert!(harness.publish().is_ok());

    let two = harness
        .unique_semantic_target(&radio_query("Two", SemanticCheckedState::Unchecked))
        .unwrap_or_else(|error| unreachable!("unchecked second radio is unique: {error:?}"));
    harness
        .submit_semantic_action(&two, SemanticAction::Activate)
        .unwrap_or_else(|error| unreachable!("semantic radio activation is accepted: {error:?}"));
    settle(&mut harness);
    assert_eq!(harness.state().selected, Some(2));
    assert_eq!(harness.state().activations, vec![2]);

    assert!(harness.publish().is_ok());
    assert!(
        harness
            .unique_semantic_target(&radio_query("Two", SemanticCheckedState::Checked))
            .is_ok()
    );
}

fn semantic_center(harness: &TestHarness<RadioApp>, query: &SemanticQuery) -> LogicalPoint {
    let target = harness
        .unique_semantic_target(query)
        .unwrap_or_else(|error| unreachable!("point target is unique: {error:?}"));
    let snapshot = harness
        .semantic_snapshot()
        .unwrap_or_else(|_| unreachable!("publication exists"));
    let bounds = snapshot
        .node(target.node_id())
        .unwrap_or_else(|| unreachable!("target belongs to snapshot"))
        .bounds();
    LogicalPoint::new(
        bounds.x() + bounds.width() / 2.0,
        bounds.y() + bounds.height() / 2.0,
    )
    .unwrap_or_else(|_| unreachable!("semantic bounds have a finite center"))
}

#[test]
fn programmatic_surface_focus_targets_one_radio_without_selecting_it() {
    let mut harness = TestHarness::<RadioApp>::mount(State {
        selected: Some(1),
        disable_two: false,
        hide_two_from_focus: false,
        activations: Vec::new(),
    });
    assert!(harness.publish().is_ok());

    let three = radio_query("Three", SemanticCheckedState::Unchecked);
    let point = semantic_center(&harness, &three);
    harness
        .submit_surface_command(
            point,
            SemanticCommand::RequestFocus,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|error| unreachable!("programmatic focus is accepted: {error:?}"));
    settle(&mut harness);
    assert_eq!(harness.state().selected, Some(1));
    assert!(harness.state().activations.is_empty());

    assert!(harness.publish().is_ok());
    assert_focus(&harness, &three);
}

#[test]
fn pointer_activation_converges_through_ordinary_application_selection() {
    let mut harness = TestHarness::<RadioApp>::mount(State {
        selected: Some(1),
        disable_two: false,
        hide_two_from_focus: false,
        activations: Vec::new(),
    });
    assert!(harness.publish().is_ok());

    let two = radio_query("Two", SemanticCheckedState::Unchecked);
    let point = semantic_center(&harness, &two);
    let pointer_id = PointerId::new(1).unwrap_or_else(|| unreachable!("pointer ID is non-zero"));

    let down = harness
        .pointer_event(
            pointer_id,
            PointerDeviceKind::Mouse,
            PointerPhase::Down,
            point,
        )
        .unwrap_or_else(|_| unreachable!("published radio accepts pointer context"))
        .with_buttons(PointerButtons::new([PointerButton::Primary]))
        .with_changed_button(PointerButton::Primary);
    harness
        .submit_pointer(down)
        .unwrap_or_else(|error| unreachable!("radio pointer down is accepted: {error:?}"));
    settle(&mut harness);

    let up = harness
        .pointer_event(
            pointer_id,
            PointerDeviceKind::Mouse,
            PointerPhase::Up,
            point,
        )
        .unwrap_or_else(|_| unreachable!("published radio accepts pointer context"))
        .with_changed_button(PointerButton::Primary);
    harness
        .submit_pointer(up)
        .unwrap_or_else(|error| unreachable!("radio pointer up is accepted: {error:?}"));
    settle(&mut harness);

    assert_eq!(harness.state().selected, Some(2));
    assert_eq!(harness.state().activations, vec![2]);
    assert!(harness.publish().is_ok());
    assert_focus(&harness, &radio_query("Two", SemanticCheckedState::Checked));
}

#[test]
fn externally_managed_radio_group_leaves_directional_command_unclaimed() {
    let mut harness = TestHarness::<ExternallyManagedRadioApp>::mount(ExternallyManagedState {
        selected: 1,
        activations: Vec::new(),
    });
    assert!(harness.publish().is_ok());

    harness
        .submit_automation_command(element_id("managed.one"), SemanticCommand::RequestFocus)
        .unwrap_or_else(|error| unreachable!("managed radio focus is accepted: {error:?}"));
    assert_eq!(
        harness.run_until_idle(settle_budget()).outcome(),
        SettleOutcome::Idle
    );

    harness
        .submit_automation_command(element_id("managed.one"), SemanticCommand::FocusRight)
        .unwrap_or_else(|error| unreachable!("managed directional command is accepted: {error:?}"));
    assert_eq!(
        harness.run_until_idle(settle_budget()).outcome(),
        SettleOutcome::Idle
    );

    assert_eq!(harness.state().selected, 1);
    assert!(harness.state().activations.is_empty());
}

#[test]
fn invalid_multiple_checked_group_fails_closed_without_rewriting_children() {
    let mut harness = TestHarness::<InvalidRadioApp>::mount(());
    assert!(harness.publish().is_ok());
    assert!(
        harness
            .publication()
            .unwrap_or_else(|| unreachable!("publication exists"))
            .diagnostics()
            .iter()
            .any(|diagnostic| {
                diagnostic.code() == "runenui.control.radio-group.multiple-checked"
            })
    );

    let groups = harness
        .query_semantics(&SemanticQuery::new().with_role(SemanticRole::RadioGroup))
        .unwrap_or_else(|_| unreachable!("publication exists"));
    assert!(groups.is_empty());

    for name in ["One", "Two"] {
        assert!(
            harness
                .unique_semantic_target(
                    &SemanticQuery::new()
                        .with_role(SemanticRole::RadioButton)
                        .with_name(name)
                        .with_checked(SemanticCheckedState::Checked)
                        .with_supported_action(SemanticAction::Activate),
                )
                .is_ok()
        );
    }

    assert!(
        harness
            .reconciliation_report()
            .diagnostics()
            .iter()
            .any(|diagnostic| matches!(
                diagnostic,
                ReconciliationDiagnostic::MultiplePreferredFocusGroupMembers { .. }
            ))
    );
}
