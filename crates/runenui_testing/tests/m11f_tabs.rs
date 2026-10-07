#![allow(refining_impl_trait)]

use runenui_core::{
    Axis, Element, ElementId, KeyLocation, KeyModifiers, KeyboardCompositionState, KeyboardEvent,
    KeyboardPhase, LogicalKey, NoHostProtocol, PhysicalKey, SemanticCommand,
    SemanticRelationshipKind, SemanticRole, UiApp, View, button, column, tab, tab_list, tab_panel,
    text,
};
use runenui_runtime::PumpBudget;
use runenui_testing::{SemanticQuery, SettleBudget, SettleOutcome, TestHarness};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Action {
    Select(u8),
    Noop,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ActivationMode {
    Manual,
    Automatic,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct State {
    selected: u8,
    activation: ActivationMode,
    orientation: Axis,
    second_disabled: bool,
    conditional_panels: bool,
    activations: Vec<u8>,
}

struct TabsApp;

impl UiApp for TabsApp {
    type State = State;
    type Action = Action;
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> impl View<Self::Action> {
        let panel_mounted = |value| !state.conditional_panels || state.selected == value;

        let mut one = tab("One", state.selected == 1)
            .id("tab.one")
            .on_activate(|| Action::Select(1));
        if panel_mounted(1) {
            one = one.controls("panel.one");
        }

        let mut two = tab("Two", state.selected == 2)
            .id("tab.two")
            .on_activate(|| Action::Select(2));
        if state.second_disabled {
            two = two.disabled();
        }
        if panel_mounted(2) {
            two = two.controls("panel.two");
        }

        let mut three = tab("Three", state.selected == 3)
            .id("tab.three")
            .on_activate(|| Action::Select(3));
        if panel_mounted(3) {
            three = three.controls("panel.three");
        }

        let mut tabs = tab_list([one, two, three])
            .id("tabs")
            .orientation(state.orientation);
        if state.activation == ActivationMode::Automatic {
            tabs = tabs.automatic_activation(true);
        }

        let mut children: Vec<Element<Action>> = vec![
            button("Before")
                .id("before")
                .on_activate(|| Action::Noop)
                .into_element(),
            tabs.into_element(),
            button("After")
                .id("after")
                .on_activate(|| Action::Noop)
                .into_element(),
        ];
        if panel_mounted(1) {
            children.push(
                tab_panel("tab.one", [text("Panel one")])
                    .id("panel.one")
                    .into_element(),
            );
        }
        if panel_mounted(2) {
            children.push(
                tab_panel("tab.two", [text("Panel two")])
                    .id("panel.two")
                    .into_element(),
            );
        }
        if panel_mounted(3) {
            children.push(
                tab_panel("tab.three", [text("Panel three")])
                    .id("panel.three")
                    .into_element(),
            );
        }
        column(children)
    }

    fn update(state: &mut Self::State, action: Self::Action) {
        match action {
            Action::Select(value) => {
                state.selected = value;
                state.activations.push(value);
            }
            Action::Noop => {}
        }
    }
}

const fn fixture(selected: u8) -> State {
    State {
        selected,
        activation: ActivationMode::Manual,
        orientation: Axis::Horizontal,
        second_disabled: false,
        conditional_panels: false,
        activations: Vec::new(),
    }
}

fn budget() -> SettleBudget {
    SettleBudget::new(
        core::num::NonZeroUsize::new(16).unwrap_or(core::num::NonZeroUsize::MIN),
        PumpBudget::new(64, 64, 64, 64),
    )
}

fn settle(harness: &mut TestHarness<TabsApp>) {
    assert_eq!(
        harness.run_until_idle(budget()).outcome(),
        SettleOutcome::Idle
    );
}

fn element_id(value: &str) -> ElementId {
    ElementId::new(value).unwrap_or_else(|_| unreachable!("fixture IDs are valid"))
}

fn command(harness: &mut TestHarness<TabsApp>, target: &str, command: SemanticCommand) {
    harness
        .submit_automation_command(element_id(target), command)
        .unwrap_or_else(|error| unreachable!("Tabs command is accepted: {error:?}"));
    settle(harness);
}

fn assert_focus_name(harness: &TestHarness<TabsApp>, name: &str) {
    let target = harness
        .unique_semantic_target(
            &SemanticQuery::new()
                .with_role(SemanticRole::Tab)
                .with_name(name),
        )
        .unwrap_or_else(|error| unreachable!("Tab target is unique: {error:?}"));
    let snapshot = harness
        .semantic_snapshot()
        .unwrap_or_else(|_| unreachable!("publication exists"));
    assert_eq!(snapshot.focused(), Some(target.node_id()));
}

const fn key(physical: PhysicalKey, logical: LogicalKey, phase: KeyboardPhase) -> KeyboardEvent {
    KeyboardEvent::new(
        phase,
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
fn manual_tabs_enter_selected_wrap_home_end_and_activate_without_selection_on_focus() {
    let mut harness = TestHarness::<TabsApp>::mount(fixture(2));
    assert!(harness.publish().is_ok());

    command(&mut harness, "before", SemanticCommand::RequestFocus);
    command(&mut harness, "before", SemanticCommand::FocusNext);
    assert!(harness.publish().is_ok());
    assert_focus_name(&harness, "Two");

    command(&mut harness, "tab.two", SemanticCommand::FocusRight);
    assert!(harness.publish().is_ok());
    assert_focus_name(&harness, "Three");
    assert_eq!(harness.state().selected, 2);
    assert_eq!(harness.state().activations.as_slice(), &[] as &[u8]);

    command(&mut harness, "tab.three", SemanticCommand::FocusRight);
    assert!(harness.publish().is_ok());
    assert_focus_name(&harness, "One");

    harness
        .submit_keyboard(key(PhysicalKey::End, LogicalKey::End, KeyboardPhase::Down))
        .unwrap_or_else(|error| unreachable!("End is accepted: {error:?}"));
    settle(&mut harness);
    assert!(harness.publish().is_ok());
    assert_focus_name(&harness, "Three");

    harness
        .submit_keyboard(key(
            PhysicalKey::Home,
            LogicalKey::Home,
            KeyboardPhase::Down,
        ))
        .unwrap_or_else(|error| unreachable!("Home is accepted: {error:?}"));
    settle(&mut harness);
    assert!(harness.publish().is_ok());
    assert_focus_name(&harness, "One");

    command(&mut harness, "tab.three", SemanticCommand::RequestFocus);
    harness
        .submit_keyboard(key(
            PhysicalKey::Space,
            LogicalKey::Space,
            KeyboardPhase::Down,
        ))
        .unwrap_or_else(|error| unreachable!("Space down is accepted: {error:?}"));
    settle(&mut harness);
    harness
        .submit_keyboard(key(
            PhysicalKey::Space,
            LogicalKey::Space,
            KeyboardPhase::Up,
        ))
        .unwrap_or_else(|error| unreachable!("Space up is accepted: {error:?}"));
    settle(&mut harness);
    assert_eq!(harness.state().selected, 3);
    assert_eq!(harness.state().activations, vec![3]);
    assert!(harness.publish().is_ok());
    assert_focus_name(&harness, "Three");
}

#[test]
fn zero_selected_falls_back_without_repair_and_reentry_prefers_application_selection() {
    let mut harness = TestHarness::<TabsApp>::mount(fixture(0));
    assert!(harness.publish().is_ok());

    command(&mut harness, "before", SemanticCommand::RequestFocus);
    command(&mut harness, "before", SemanticCommand::FocusNext);
    assert!(harness.publish().is_ok());
    assert_focus_name(&harness, "One");
    assert_eq!(harness.state().selected, 0);
    assert_eq!(harness.state().activations.as_slice(), &[] as &[u8]);

    command(&mut harness, "tab.three", SemanticCommand::RequestFocus);
    command(&mut harness, "tab.three", SemanticCommand::Activate);
    assert_eq!(harness.state().selected, 3);
    assert_eq!(harness.state().activations, vec![3]);

    command(&mut harness, "before", SemanticCommand::RequestFocus);
    command(&mut harness, "before", SemanticCommand::FocusNext);
    assert!(harness.publish().is_ok());
    assert_focus_name(&harness, "Three");
}

#[test]
fn automatic_tabs_activate_only_after_successful_focus_movement() {
    let mut state = fixture(1);
    state.activation = ActivationMode::Automatic;
    let mut harness = TestHarness::<TabsApp>::mount(state);
    assert!(harness.publish().is_ok());

    command(&mut harness, "before", SemanticCommand::RequestFocus);
    command(&mut harness, "before", SemanticCommand::FocusNext);
    assert!(harness.publish().is_ok());
    assert_focus_name(&harness, "One");
    assert_eq!(harness.state().activations.as_slice(), &[] as &[u8]);

    command(&mut harness, "tab.one", SemanticCommand::FocusRight);
    assert_eq!(harness.state().selected, 2);
    assert_eq!(harness.state().activations, vec![2]);
    assert!(harness.publish().is_ok());
    assert_focus_name(&harness, "Two");
}

#[test]
fn disabled_selected_falls_back_vertical_navigation_skips_disabled_and_wrong_axis_is_unowned() {
    let mut state = fixture(2);
    state.orientation = Axis::Vertical;
    state.second_disabled = true;
    let mut harness = TestHarness::<TabsApp>::mount(state);
    assert!(harness.publish().is_ok());

    command(&mut harness, "before", SemanticCommand::RequestFocus);
    command(&mut harness, "before", SemanticCommand::FocusNext);
    assert!(harness.publish().is_ok());
    assert_focus_name(&harness, "One");

    command(&mut harness, "tab.one", SemanticCommand::FocusRight);
    assert!(harness.publish().is_ok());
    assert_focus_name(&harness, "One");

    command(&mut harness, "tab.one", SemanticCommand::FocusDown);
    assert!(harness.publish().is_ok());
    assert_focus_name(&harness, "Three");
    assert_eq!(harness.state().selected, 2);

    command(&mut harness, "tab.three", SemanticCommand::FocusDown);
    assert!(harness.publish().is_ok());
    assert_focus_name(&harness, "One");
}

#[test]
fn conditional_panel_rebuild_keeps_relationships_exact_without_dangling_targets() {
    let mut state = fixture(1);
    state.conditional_panels = true;
    let mut harness = TestHarness::<TabsApp>::mount(state);
    let publication = harness
        .publish()
        .unwrap_or_else(|error| unreachable!("conditional Tabs publish: {error:?}"));
    assert!(publication.semantic_diagnostics().is_empty());

    let snapshot = publication.semantic_publication().snapshot();
    assert_eq!(
        snapshot
            .nodes()
            .iter()
            .filter(|node| node.role() == SemanticRole::TabPanel)
            .count(),
        1
    );
    let one = snapshot
        .nodes()
        .iter()
        .find(|node| node.role() == SemanticRole::Tab && node.name() == Some("One"))
        .unwrap_or_else(|| unreachable!("selected Tab is published"));
    let two = snapshot
        .nodes()
        .iter()
        .find(|node| node.role() == SemanticRole::Tab && node.name() == Some("Two"))
        .unwrap_or_else(|| unreachable!("unselected Tab is published"));
    let controls = one
        .relationships()
        .iter()
        .filter(|relationship| relationship.kind() == SemanticRelationshipKind::Controls)
        .collect::<Vec<_>>();
    assert_eq!(controls.len(), 1);
    assert_eq!(
        snapshot
            .node(controls[0].target())
            .map(runenui_runtime::SemanticNode::role),
        Some(SemanticRole::TabPanel)
    );
    assert_eq!(two.relationships().len(), 0);

    let panel = snapshot
        .node(controls[0].target())
        .unwrap_or_else(|| unreachable!("controlled panel is published"));
    let labelled_by = panel
        .relationships()
        .iter()
        .filter(|relationship| relationship.kind() == SemanticRelationshipKind::LabelledBy)
        .collect::<Vec<_>>();
    assert_eq!(labelled_by.len(), 1);
    assert_eq!(
        snapshot
            .node(labelled_by[0].target())
            .map(runenui_runtime::SemanticNode::role),
        Some(SemanticRole::Tab)
    );

    command(&mut harness, "tab.two", SemanticCommand::RequestFocus);
    command(&mut harness, "tab.two", SemanticCommand::Activate);
    assert_eq!(harness.state().selected, 2);
    let publication = harness
        .publish()
        .unwrap_or_else(|error| unreachable!("rebuilt conditional Tabs publish: {error:?}"));
    assert!(publication.semantic_diagnostics().is_empty());
    let snapshot = publication.semantic_publication().snapshot();
    let one = snapshot
        .nodes()
        .iter()
        .find(|node| node.role() == SemanticRole::Tab && node.name() == Some("One"))
        .unwrap_or_else(|| unreachable!("first Tab remains published"));
    let two = snapshot
        .nodes()
        .iter()
        .find(|node| node.role() == SemanticRole::Tab && node.name() == Some("Two"))
        .unwrap_or_else(|| unreachable!("second Tab remains published"));
    assert_eq!(one.relationships().len(), 0);
    assert_eq!(
        two.relationships()
            .iter()
            .filter(|relationship| relationship.kind() == SemanticRelationshipKind::Controls)
            .count(),
        1
    );
    assert_eq!(
        snapshot
            .nodes()
            .iter()
            .filter(|node| node.role() == SemanticRole::TabPanel)
            .count(),
        1
    );
}
