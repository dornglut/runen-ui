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
}
#[derive(Clone, Debug)]
enum Action {
    Request(TreeItemRequest),
}
struct App;
impl UiApp for App {
    type State = State;
    type Action = Action;
    type HostProtocol = NoHostProtocol;
    fn root(state: &Self::State) -> impl View<Self::Action> {
        let first = tree_item("a", "Alpha", Vec::new())
            .selected(state.selected == "a")
            .on_request(Action::Request).id("tree.a");
        let second = tree_item("b", "Bravo", Vec::new())
            .selected(state.selected == "b")
            .on_request(Action::Request).id("tree.b");
        let parent = tree_item("parent", "Folder", vec![first, second])
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
