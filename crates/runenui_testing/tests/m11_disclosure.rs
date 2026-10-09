use core::num::NonZeroUsize;

use runenui_core::{
    ElementId, NoHostProtocol, SemanticAction, SemanticCommand,
    SemanticRelationshipKind, SemanticRole, UiApp, View, button, column, disclosure,
};
use runenui_runtime::PumpBudget;
use runenui_testing::{SemanticQuery, SettleBudget, SettleOutcome, TestHarness};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Action {
    Toggle,
    Author(bool),
    Enable(bool),
    Child,
}

#[derive(Clone, Debug)]
struct State {
    expanded: bool,
    enabled: bool,
    toggles: u32,
    child_activations: u32,
}

struct DisclosureApp;

impl UiApp for DisclosureApp {
    type State = State;
    type Action = Action;
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> impl View<Self::Action> {
        let mut trigger = disclosure("Advanced", state.expanded)
            .id("disclosure.trigger")
            .controls("disclosure.panel")
            .on_activate(|| Action::Toggle);
        if !state.enabled {
            trigger = trigger.disabled();
        }
        let mut children = vec![trigger.into_element()];
        if state.expanded {
            children.push(
                column([button("Panel action")
                    .id("disclosure.child")
                    .on_activate(|| Action::Child)])
                .id("disclosure.panel")
                .into_element(),
            );
        }
        column(children)
    }

    fn update(state: &mut Self::State, action: Self::Action) {
        match action {
            Action::Toggle => {
                state.toggles += 1;
                state.expanded = !state.expanded;
            }
            Action::Author(expanded) => state.expanded = expanded,
            Action::Enable(enabled) => state.enabled = enabled,
            Action::Child => state.child_activations += 1,
        }
    }
}

fn state(expanded: bool) -> State {
    State {
        expanded,
        enabled: true,
        toggles: 0,
        child_activations: 0,
    }
}

fn budget() -> SettleBudget {
    SettleBudget::new(
        NonZeroUsize::new(16).unwrap_or(NonZeroUsize::MIN),
        PumpBudget::new(64, 64, 64, 64),
    )
}

fn settle(harness: &mut TestHarness<DisclosureApp>) {
    assert_eq!(harness.run_until_idle(budget()).outcome(), SettleOutcome::Idle);
}

fn query() -> SemanticQuery {
    SemanticQuery::new()
        .with_role(SemanticRole::Button)
        .with_name("Advanced")
}

fn assert_disclosure(harness: &TestHarness<DisclosureApp>, expanded: bool) {
    let snapshot = harness.semantic_snapshot().unwrap_or_else(|_| unreachable!("published"));
    let node = snapshot.nodes().iter()
        .find(|node| node.role() == SemanticRole::Button && node.name() == Some("Advanced"))
        .unwrap_or_else(|| unreachable!("trigger semantic node"));
    assert_eq!(node.state().expanded(), Some(expanded));
    assert!(node.supported_actions().contains(&SemanticAction::Activate));
    assert_eq!(
        node.supported_actions().contains(&SemanticAction::Expand),
        !expanded && harness.state().enabled,
    );
    assert_eq!(
        node.supported_actions().contains(&SemanticAction::Collapse),
        expanded && harness.state().enabled,
    );
    let relations = node.relationships().iter()
        .filter(|r| r.kind() == SemanticRelationshipKind::Controls)
        .collect::<Vec<_>>();
    assert_eq!(relations.len(), usize::from(expanded));
    let panel_nodes = snapshot.nodes().iter()
        .filter(|node| node.role() == SemanticRole::Button && node.name() == Some("Panel action"))
        .count();
    assert_eq!(panel_nodes, usize::from(expanded));
    if expanded {
        assert_eq!(
            snapshot.node(relations[0].target()).map(runenui_runtime::SemanticNode::role),
            Some(SemanticRole::Group)
        );
    }
}

fn author(harness: &mut TestHarness<DisclosureApp>, action: Action) {
    assert!(harness.submit_action(action).is_ok());
    settle(harness);
    assert!(harness.publish().is_ok());
}

#[test]
fn activate_expand_collapse_converge_on_one_application_update_fifo() {
    let mut harness = TestHarness::<DisclosureApp>::mount(state(false));
    assert!(harness.publish().is_ok());
    assert_disclosure(&harness, false);
    let trigger = harness.unique_semantic_target(&query())
        .unwrap_or_else(|error| unreachable!("trigger exists: {error:?}"));

    assert!(harness.submit_semantic_action(&trigger, SemanticAction::Expand).is_ok());
    assert_eq!(harness.state().expanded, false);
    settle(&mut harness);
    assert_eq!(harness.state().toggles, 1);
    assert_eq!(harness.state().expanded, true);
    assert!(harness.publish().is_ok());
    assert_disclosure(&harness, true);

    let trigger = harness.unique_semantic_target(&query())
        .unwrap_or_else(|error| unreachable!("trigger remains: {error:?}"));
    assert!(harness.submit_semantic_action(&trigger, SemanticAction::Expand).is_err());
    assert!(harness.submit_semantic_action(&trigger, SemanticAction::Activate).is_ok());
    settle(&mut harness);
    assert_eq!(harness.state().toggles, 2);
    assert_eq!(harness.state().expanded, false);
    assert!(harness.publish().is_ok());
    assert_disclosure(&harness, false);

    let trigger = harness.unique_semantic_target(&query())
        .unwrap_or_else(|error| unreachable!("collapsed trigger remains: {error:?}"));
    assert!(harness.submit_semantic_action(&trigger, SemanticAction::Collapse).is_err());
    assert!(harness.submit_semantic_action(&trigger, SemanticAction::Activate).is_ok());
    settle(&mut harness);
    assert!(harness.publish().is_ok());
    assert_disclosure(&harness, true);

    let trigger = harness.unique_semantic_target(&query())
        .unwrap_or_else(|error| unreachable!("expanded trigger remains: {error:?}"));
    assert!(harness.submit_semantic_action(&trigger, SemanticAction::Collapse).is_ok());
    settle(&mut harness);
    assert!(harness.publish().is_ok());
    assert_disclosure(&harness, false);
    assert_eq!(harness.state().toggles, 4);
}

#[test]
fn disabled_and_authored_state_do_not_mutate_expansion_behind_app() {
    let mut harness = TestHarness::<DisclosureApp>::mount(state(false));
    assert!(harness.publish().is_ok());
    author(&mut harness, Action::Enable(false));
    assert_disclosure(&harness, false);
    let trigger = harness.unique_semantic_target(&query())
        .unwrap_or_else(|error| unreachable!("disabled trigger remains: {error:?}"));
    assert!(harness.submit_semantic_action(&trigger, SemanticAction::Expand).is_err());
    assert!(harness.submit_semantic_action(&trigger, SemanticAction::Activate).is_err());
    settle(&mut harness);
    assert!(!harness.state().expanded);
    assert_eq!(harness.state().toggles, 0);

    author(&mut harness, Action::Author(true));
    assert_disclosure(&harness, true);
    assert_eq!(harness.state().toggles, 0);
    assert_eq!(harness.state().child_activations, 0);
    author(&mut harness, Action::Enable(true));
    assert_disclosure(&harness, true);
}

#[test]
fn collapsing_mounted_panel_retires_focusable_child_and_relationship() {
    let mut harness = TestHarness::<DisclosureApp>::mount(state(true));
    assert!(harness.publish().is_ok());
    assert_disclosure(&harness, true);
    let child = harness.unique_semantic_target(
        &SemanticQuery::new().with_role(SemanticRole::Button).with_name("Panel action")
    ).unwrap_or_else(|error| unreachable!("focusable content: {error:?}"));
    harness.submit_automation_command(
        ElementId::new("disclosure.child")
            .unwrap_or_else(|_| unreachable!("fixture ID valid")),
        SemanticCommand::RequestFocus,
    ).unwrap_or_else(|error| unreachable!("focus request admitted: {error:?}"));
    settle(&mut harness);
    assert!(harness.publish().is_ok());
    assert_eq!(
        harness.semantic_snapshot().unwrap_or_else(|_| unreachable!("published")).focused(),
        Some(child.node_id())
    );
    author(&mut harness, Action::Author(false));
    assert_disclosure(&harness, false);
    let snapshot = harness.semantic_snapshot().unwrap_or_else(|_| unreachable!("published"));
    assert!(snapshot.node(child.node_id()).is_none());
    assert_ne!(snapshot.focused(), Some(child.node_id()));
    assert_eq!(harness.state().child_activations, 0);

    author(&mut harness, Action::Author(true));
    assert_disclosure(&harness, true);
    assert_eq!(harness.state().child_activations, 0);
}
