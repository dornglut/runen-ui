#![allow(refining_impl_trait)]

// Production TreeView must work from an ordinary external application.
use runenui_core::{
    CommandOrigin, NoHostProtocol, SemanticAction, SemanticCollectionPosition, SemanticCommand,
    SemanticRole, TreeItemRequest, UiApp, View, tree_item, tree_view,
};
use runenui_runtime::{AppRuntime, LogicalSize, MountedNodeId, PumpBudget, SurfaceBuildContext};

#[derive(Clone, Debug)]
struct State {
    expanded: bool,
    selected: String,
    actions: Vec<TreeItemRequest>,
    disabled_a: bool,
    discover_a: bool,
    show_b: bool,
}
#[derive(Clone, Debug)]
enum Action {
    Request(TreeItemRequest),
    DisabledA(bool),
    DiscoverA(bool),
    ShowB(bool),
    Expanded(bool),
}
struct App;
impl UiApp for App {
    type State = State;
    type Action = Action;
    type HostProtocol = NoHostProtocol;
    fn root(state: &Self::State) -> impl View<Self::Action> {
        let first = tree_item("a", "Alpha", Vec::new())
            .selected(state.selected == "a")
            .enabled(!state.disabled_a)
            .discoverable_when_disabled(state.discover_a)
            .on_request(Action::Request).id("tree.a");
        let second = tree_item("b", "Bravo", Vec::new())
            .selected(state.selected == "b")
            .on_request(Action::Request).id("tree.b");
        let children = if state.show_b { vec![first, second] } else { vec![first] };
        let parent = tree_item("parent", "Folder", children)
            .expanded(state.expanded)
            .selected(state.selected == "parent")
            .on_request(Action::Request).id("tree.parent");
        let other = tree_item("other", "Outside", Vec::new())
            .selected(state.selected == "other")
            .on_request(Action::Request).id("tree.other");
        tree_view(vec![parent, other]).id("tree.root")
    }
    fn update(state: &mut Self::State, action: Self::Action) {
        match action {
            Action::DisabledA(value) => state.disabled_a = value,
            Action::DiscoverA(value) => state.discover_a = value,
            Action::ShowB(value) => state.show_b = value,
            Action::Expanded(value) => state.expanded = value,
            Action::Request(intent) => {
                match &intent {
                    TreeItemRequest::Expand(id) if id == "parent" => state.expanded = true,
                    TreeItemRequest::Collapse(id) if id == "parent" => state.expanded = false,
                    TreeItemRequest::Select(id) => state.selected.clone_from(id),
                    _ => {},
                }
                state.actions.push(intent);
            }
        }
    }
}
fn fresh(expanded: bool) -> AppRuntime<App> {
    let mut runtime = AppRuntime::<App>::mount(State {
        expanded, selected: "parent".to_owned(), actions: Vec::new(),
        disabled_a: false, discover_a: false, show_b: true,
    });
    settle(&mut runtime);
    runtime
}
fn settle(runtime: &mut AppRuntime<App>) {
    assert!(runtime.pump(PumpBudget::new(
        usize::MAX, usize::MAX, usize::MAX, usize::MAX,
    )).is_quiescent());
}
fn item(runtime: &mut AppRuntime<App>, name: &str) -> MountedNodeId {
    let authored = runenui_core::ElementId::new(name).unwrap_or_else(|_| unreachable!());
    runtime.index().nodes().iter()
        .find(|node| node.authored_id() == Some(&authored))
        .unwrap_or_else(|| unreachable!("authored tree item present"))
        .id().clone()
}
fn command(runtime: &mut AppRuntime<App>, target: MountedNodeId, key: SemanticCommand) {
    runtime.submit_command(target, key, CommandOrigin::programmatic())
        .unwrap_or_else(|_| unreachable!("tree command admitted"));
    settle(runtime);
}

#[test]
fn exact_parent_focus_uses_bubbling_not_preceding_sibling() {
    let mut runtime = fresh(true);
    let b = item(&mut runtime, "tree.b");
    let parent = item(&mut runtime, "tree.parent");
    command(&mut runtime, b.clone(), SemanticCommand::RequestFocus);
    assert_eq!(runtime.focus().focused_node(), Some(&b));
    command(&mut runtime, b, SemanticCommand::FocusLeft);
    assert_eq!(runtime.focus().focused_node(), Some(&parent));
    assert!(runtime.state().actions.is_empty(), "focus never commits selection");
    command(&mut runtime, parent.clone(), SemanticCommand::FocusRight);
    let first_child = item(&mut runtime, "tree.a");
    assert_eq!(runtime.focus().focused_node(), Some(&first_child));
    command(&mut runtime, parent, SemanticCommand::FocusDown);
    assert_eq!(runtime.state().selected, "parent");
}

#[test]
fn collapse_expand_and_activation_are_application_fifo_with_stable_domain_ids() {
    let mut runtime = fresh(false);
    let parent = item(&mut runtime, "tree.parent");
    command(&mut runtime, parent.clone(), SemanticCommand::RequestFocus);
    command(&mut runtime, parent.clone(), SemanticCommand::FocusRight);
    assert!(runtime.state().expanded);
    assert_eq!(runtime.state().actions, [TreeItemRequest::Expand("parent".to_owned())]);
    let a = item(&mut runtime, "tree.a");
    command(&mut runtime, a, SemanticCommand::Activate);
    assert_eq!(runtime.state().selected, "a");
    assert_eq!(runtime.state().actions[1], TreeItemRequest::Select("a".to_owned()));
    let parent = item(&mut runtime, "tree.parent");
    command(&mut runtime, parent.clone(), SemanticCommand::FocusLeft);
    assert!(!runtime.state().expanded);
    assert_eq!(runtime.state().actions[2], TreeItemRequest::Collapse("parent".to_owned()));
    assert!(!runtime.index().nodes().iter().any(|n|
        n.authored_id().is_some_and(|id| id.as_str() == "tree.a")));
}

#[test]
fn exact_semantic_tree_hierarchy_selection_and_expand_commands() {
    let mut runtime = fresh(true);
    let publication = runtime.publish_surface(&SurfaceBuildContext::tight(
        &runenui_core::StyleEnvironment::default(),
        LogicalSize::try_new(640.0, 480.0).unwrap_or_else(|_| unreachable!()),
    )).unwrap_or_else(|error| unreachable!("tree publication: {error:?}"));
    let snapshot = publication.semantic_publication().snapshot();
    let tree = snapshot.nodes().iter().find(|node| node.role() == SemanticRole::Tree)
        .unwrap_or_else(|| unreachable!("Tree semantic role"));
    assert_eq!(tree.children().len(), 2);
    let parent = snapshot.nodes().iter().find(|node|
        node.role() == SemanticRole::TreeItem && node.name() == Some("Folder")
    ).unwrap_or_else(|| unreachable!("TreeItem role"));
    assert_eq!(parent.state().selected(), Some(true));
    assert_eq!(parent.state().expanded(), Some(true));
    assert_eq!(parent.hierarchy_level().map(runenui_core::SemanticHierarchyLevel::get), Some(1));
    assert_eq!(parent.collection_position(), Some(
        SemanticCollectionPosition::new(0, Some(2)).unwrap_or_else(|_| unreachable!())
    ));
    assert_eq!(parent.children().len(), 2);
    assert!(parent.supported_actions().contains(&SemanticAction::Collapse));
    let first = snapshot.nodes().iter().find(|node|
        node.role() == SemanticRole::TreeItem && node.name() == Some("Alpha")
    ).unwrap_or_else(|| unreachable!("nested TreeItem"));
    assert_eq!(first.hierarchy_level().map(runenui_core::SemanticHierarchyLevel::get), Some(2));
}

#[test]
fn disabled_discoverability_is_generic_group_focus_without_disabled_activation() {
    let mut runtime = fresh(true);
    runtime.submit_action(Action::DisabledA(true))
        .unwrap_or_else(|_| unreachable!("application authors disabled state"));
    settle(&mut runtime);
    let parent = item(&mut runtime, "tree.parent");
    let b = item(&mut runtime, "tree.b");
    command(&mut runtime, parent.clone(), SemanticCommand::RequestFocus);
    command(&mut runtime, parent.clone(), SemanticCommand::FocusRight);
    assert_eq!(runtime.focus().focused_node(), Some(&b),
        "ineligible disabled child skipped without private Tree registry");
    runtime.submit_action(Action::DiscoverA(true))
        .unwrap_or_else(|_| unreachable!("application authors discoverability"));
    settle(&mut runtime);
    let parent = item(&mut runtime, "tree.parent");
    command(&mut runtime, parent.clone(), SemanticCommand::RequestFocus);
    command(&mut runtime, parent.clone(), SemanticCommand::FocusRight);
    let a = item(&mut runtime, "tree.a");
    assert_eq!(runtime.focus().focused_node(), Some(&a));
    command(&mut runtime, a.clone(), SemanticCommand::Activate);
    assert_eq!(runtime.state().selected, "parent");
    assert!(runtime.state().actions.is_empty(),
        "disabled but discoverable item never activates");
}

#[test]
fn collapsing_or_removing_focused_descendant_retires_exact_target_identity() {
    let mut runtime = fresh(true);
    let old_b = item(&mut runtime, "tree.b");
    command(&mut runtime, old_b.clone(), SemanticCommand::RequestFocus);
    assert_eq!(runtime.focus().focused_node(), Some(&old_b));
    runtime.submit_action(Action::Expanded(false))
        .unwrap_or_else(|_| unreachable!("application collapses"));
    settle(&mut runtime);
    assert_ne!(runtime.focus().focused_node(), Some(&old_b));
    assert!(!runtime.index().nodes().iter().any(|n| n.id() == &old_b));
    runtime.submit_action(Action::ShowB(false))
        .unwrap_or_else(|_| unreachable!("application removes domain item"));
    runtime.submit_action(Action::Expanded(true))
        .unwrap_or_else(|_| unreachable!("application expands"));
    settle(&mut runtime);
    assert!(!runtime.index().nodes().iter().any(|n| n.id() == &old_b));
    assert!(runtime.state().actions.is_empty(),
        "collapse and retirement do not fabricate an app selection");
}

struct InvalidTreeApp;
impl UiApp for InvalidTreeApp {
    type State = u8;
    type Action = u8;
    type HostProtocol = NoHostProtocol;
    fn root(mode: &Self::State) -> impl View<Self::Action> {
        let child_id = if *mode == 1 { "root" } else if *mode == 2 { "" } else { "child" };
        let child = tree_item(child_id, "Nested", Vec::new())
            .selected(*mode == 3);
        let parent = tree_item("root", "Parent", vec![child])
            .expanded(false)
            .selected(*mode == 3);
        tree_view(vec![parent]).id("tree.invalid")
    }
    fn update(state: &mut Self::State, action: Self::Action) {
        *state = action;
    }
}

#[test]
fn invalid_domain_id_and_single_selection_fail_closed_even_under_collapsed_ancestor() {
    let mut runtime = AppRuntime::<InvalidTreeApp>::mount(0);
    for mode in [1_u8, 2, 3] {
        runtime.submit_action(mode).unwrap_or_else(|_| unreachable!("authoring transition"));
        assert!(runtime.pump(PumpBudget::new(
            usize::MAX, usize::MAX, usize::MAX, usize::MAX,
        )).is_quiescent());
        let publication = runtime.publish_surface(&SurfaceBuildContext::tight(
            &runenui_core::StyleEnvironment::default(),
            LogicalSize::try_new(600.0, 300.0)
                .unwrap_or_else(|_| unreachable!("finite tree surface")),
        )).unwrap_or_else(|error| unreachable!("invalid tree reports diagnostic: {error:?}"));
        let snapshot = publication.semantic_publication().snapshot();
        assert!(!snapshot.nodes().iter().any(|node| node.role() == SemanticRole::Tree));
        assert!(!snapshot.nodes().iter().any(|node| node.role() == SemanticRole::TreeItem));
        assert!(runtime.index().nodes().iter().all(|node|
            node.authored_id().is_none_or(|id| id.as_str() != "tree.invalid")
            || runtime.focus().focused_node() != Some(node.id())
        ));
    }
    runtime.submit_action(0).unwrap_or_else(|_| unreachable!("valid authoring transition"));
    assert!(runtime.pump(PumpBudget::new(
        usize::MAX, usize::MAX, usize::MAX, usize::MAX,
    )).is_quiescent());
    let publication = runtime.publish_surface(&SurfaceBuildContext::tight(
        &runenui_core::StyleEnvironment::default(),
        LogicalSize::try_new(600.0, 300.0).unwrap_or_else(|_| unreachable!()),
    )).unwrap_or_else(|error| unreachable!("restored valid tree: {error:?}"));
    assert!(publication.semantic_publication().snapshot().nodes().iter()
        .any(|node| node.role() == SemanticRole::Tree));
}
