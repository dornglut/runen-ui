#![cfg(feature = "internal-test-seams")]
#![allow(refining_impl_trait)]

use runenui_core::{
    CommandOrigin, Element, FocusGroup, FocusGroupActivationPolicy, FocusGroupBoundaryPolicy,
    FocusReason, NoHostProtocol, SemanticCommand, StyleEnvironment, UiApp, View, button, column,
};
use runenui_runtime::{
    AppRuntime, LayoutConstraints, MountedNodeId, PumpBudget, ReconciliationDiagnostic,
    SurfaceBuildContext,
};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
struct State {
    disable_preferred: bool,
    duplicate_preferred: bool,
    activations: Vec<&'static str>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Action {
    Activated(&'static str),
}

struct App;

impl UiApp for App {
    type State = State;
    type Action = Action;
    type HostProtocol = NoHostProtocol;

    fn root(state: &State) -> Element<Action> {
        let group = column(vec![member(state, "a"), member(state, "b"), member(state, "c")])
            .id("group")
            .key("group")
            .into_element()
            .focus_group(
                FocusGroup::new()
                    .with_boundary(FocusGroupBoundaryPolicy::Wrap)
                    .with_activation(FocusGroupActivationPolicy::ActivateTarget),
            );
        column(vec![member(state, "before"), group, member(state, "after")])
            .key("root")
            .into_element()
    }

    fn update(state: &mut State, action: Action) {
        let Action::Activated(name) = action;
        state.activations.push(name);
    }
}

fn member(state: &State, name: &'static str) -> Element<Action> {
    let mut control = button(name)
        .id(name)
        .key(name)
        .on_activate(move || Action::Activated(name));
    if state.disable_preferred && name == "b" {
        control = control.disabled();
    }
    let mut element = control.into_element();
    if name == "b" || (state.duplicate_preferred && name == "a") {
        element = element.focus_group_preferred(true);
    }
    element
}

fn settle(runtime: &mut AppRuntime<App>) {
    runtime.pump(PumpBudget::new(usize::MAX, usize::MAX, usize::MAX, usize::MAX));
}

fn id(runtime: &mut AppRuntime<App>, name: &str) -> MountedNodeId {
    let authored = runenui_core::ElementId::new(name).unwrap_or_else(|_| unreachable!());
    runtime
        .index()
        .nodes()
        .iter()
        .find(|node| node.authored_id() == Some(&authored))
        .unwrap_or_else(|| unreachable!("named focus-group corpus node is mounted"))
        .id()
        .clone()
}

fn command(runtime: &mut AppRuntime<App>, target: MountedNodeId, command: SemanticCommand) {
    runtime
        .submit_command(target, command, CommandOrigin::programmatic())
        .unwrap_or_else(|_| unreachable!("live focus-group command is accepted"));
    assert_eq!(
        runtime.pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX)).processed_envelopes(),
        1
    );
}

fn publish_geometry(runtime: &mut AppRuntime<App>) {
    let style_environment = StyleEnvironment::default();
    let _ = runtime.publish_surface(&SurfaceBuildContext::new(
        &style_environment,
        LayoutConstraints::unbounded(),
    ));
    let issued = [
        ("before", [0.0, 0.0, 10.0, 10.0]),
        ("group", [20.0, 0.0, 30.0, 10.0]),
        ("a", [20.0, 0.0, 10.0, 10.0]),
        ("b", [30.0, 0.0, 10.0, 10.0]),
        ("c", [40.0, 0.0, 10.0, 10.0]),
        ("after", [60.0, 0.0, 10.0, 10.0]),
    ]
    .into_iter()
    .map(|(name, rect)| (id(runtime, name), rect))
    .collect::<Vec<_>>();
    runtime.__replace_current_focus_geometry_for_test(&issued);
}

#[test]
fn external_traversal_collapses_group_and_uses_preferred_entry() {
    let mut runtime = AppRuntime::<App>::mount(State::default());
    settle(&mut runtime);
    publish_geometry(&mut runtime);

    let before = id(&mut runtime, "before");
    let preferred = id(&mut runtime, "b");
    let after = id(&mut runtime, "after");

    command(&mut runtime, before.clone(), SemanticCommand::RequestFocus);
    command(&mut runtime, before.clone(), SemanticCommand::FocusNext);
    assert_eq!(runtime.focus().focused_node(), Some(&preferred));
    assert_eq!(runtime.focus().reason(), Some(FocusReason::LinearNavigation));

    command(&mut runtime, preferred.clone(), SemanticCommand::FocusNext);
    assert_eq!(runtime.focus().focused_node(), Some(&after));

    command(&mut runtime, before.clone(), SemanticCommand::RequestFocus);
    command(&mut runtime, before, SemanticCommand::FocusRight);
    assert_eq!(runtime.focus().focused_node(), Some(&preferred));
    assert_eq!(runtime.focus().reason(), Some(FocusReason::DirectionalNavigation));
}

#[test]
fn preferred_entry_falls_back_when_preferred_member_is_disabled() {
    let mut runtime = AppRuntime::<App>::mount(State {
        disable_preferred: true,
        ..State::default()
    });
    settle(&mut runtime);
    let before = id(&mut runtime, "before");
    let fallback = id(&mut runtime, "a");
    command(&mut runtime, before.clone(), SemanticCommand::RequestFocus);
    command(&mut runtime, before, SemanticCommand::FocusNext);
    assert_eq!(runtime.focus().focused_node(), Some(&fallback));
}

#[test]
fn internal_navigation_wraps_and_activation_is_deferred_until_after_focus() {
    let mut runtime = AppRuntime::<App>::mount(State::default());
    settle(&mut runtime);
    let b = id(&mut runtime, "b");
    let c = id(&mut runtime, "c");
    let a = id(&mut runtime, "a");

    command(&mut runtime, b.clone(), SemanticCommand::RequestFocus);
    command(&mut runtime, b, SemanticCommand::FocusGroupNext);
    assert_eq!(runtime.focus().focused_node(), Some(&c));
    assert_eq!(runtime.focus().reason(), Some(FocusReason::GroupNavigation));
    assert!(runtime.state().activations.is_empty());
    settle(&mut runtime);
    assert_eq!(runtime.state().activations, vec!["c"]);

    command(&mut runtime, c, SemanticCommand::FocusGroupNext);
    assert_eq!(runtime.focus().focused_node(), Some(&a));
    settle(&mut runtime);
    assert_eq!(runtime.state().activations, vec!["c", "a"]);
}

#[test]
fn invalid_multiple_preferred_members_diagnose_and_fail_closed() {
    let mut runtime = AppRuntime::<App>::mount(State {
        duplicate_preferred: true,
        ..State::default()
    });
    settle(&mut runtime);

    assert!(runtime.reconciliation_report().diagnostics().iter().any(|diagnostic| {
        matches!(
            diagnostic,
            ReconciliationDiagnostic::MultiplePreferredFocusGroupMembers { group_path, .. }
                if group_path == "root/1"
        )
    }));

    let before = id(&mut runtime, "before");
    let after = id(&mut runtime, "after");
    command(&mut runtime, before.clone(), SemanticCommand::RequestFocus);
    command(&mut runtime, before, SemanticCommand::FocusNext);
    assert_eq!(runtime.focus().focused_node(), Some(&after));
}
