use core::num::NonZeroUsize;

use runenui_core::{
    CommandOrigin, ElementId, NoHostProtocol, SemanticAction, SemanticCheckedState, SemanticCommand,
    SemanticRole, UiApp, View, button, children, column, radio_button, radio_group,
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
            button("Before")
                .id("before")
                .on_activate(|| Action::Noop),
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
            .unique_semantic_target(&radio_query(
                "One",
                SemanticCheckedState::Unchecked,
            ))
            .is_ok()
    );
    assert!(
        harness
            .unique_semantic_target(&radio_query("Two", SemanticCheckedState::Checked))
            .is_ok()
    );
    assert!(
        harness
            .unique_semantic_target(&radio_query(
                "Three",
                SemanticCheckedState::Unchecked,
            ))
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
    assert_focus(
        &harness,
        &radio_query("Two", SemanticCheckedState::Checked),
    );
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
        assert_focus(
            &harness,
            &radio_query(name, SemanticCheckedState::Checked),
        );
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
fn semantic_activation_updates_application_selection_before_checked_republication() {
    let mut harness = TestHarness::<RadioApp>::mount(State {
        selected: Some(1),
        disable_two: false,
        hide_two_from_focus: false,
        activations: Vec::new(),
    });
    assert!(harness.publish().is_ok());

    let two = harness
        .unique_semantic_target(&radio_query(
            "Two",
            SemanticCheckedState::Unchecked,
        ))
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

#[test]
fn invalid_multiple_checked_group_fails_closed_without_rewriting_children() {
    let mut harness = TestHarness::<InvalidRadioApp>::mount(());
    assert!(harness.publish().is_ok());

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
