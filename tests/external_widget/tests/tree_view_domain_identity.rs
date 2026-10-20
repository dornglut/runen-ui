#![allow(refining_impl_trait)]

//! Durable application identity must survive replacement of rendered/mounted items.
use runenui_core::{
    CommandOrigin, NoHostProtocol, SemanticCommand, TreeItemRequest, UiApp, View, tree_item,
    tree_view,
};
use runenui_runtime::{AppRuntime, MountedNodeId, PumpBudget};

#[derive(Default)]
struct State {
    generation: u8,
    selected: Option<String>,
    actions: Vec<TreeItemRequest>,
}
enum Action {
    Rebuild,
    Request(TreeItemRequest),
}
struct App;
impl UiApp for App {
    type State = State;
    type Action = Action;
    type HostProtocol = NoHostProtocol;
    fn root(state: &Self::State) -> impl View<Self::Action> {
        // Authored IDs identify public nodes; keys determine mounted lifetimes.
        // Change the key as well to exercise a genuine remount.
        let leaf = tree_item("stable-domain-item", "Durable", Vec::new())
            .id(format!("realized.{}", state.generation))
            .key(format!("realized.{}", state.generation))
            .selected(state.selected.as_deref() == Some("stable-domain-item"))
            .on_request(Action::Request);
        tree_view(vec![leaf]).id("stable.tree")
    }
    fn update(state: &mut Self::State, action: Self::Action) {
        match action {
            Action::Rebuild => state.generation += 1,
            Action::Request(request) => {
                if let TreeItemRequest::Select(ref domain_id) = request {
                    state.selected = Some(domain_id.clone());
                }
                state.actions.push(request);
            }
        }
    }
}
fn pump(runtime: &mut AppRuntime<App>) {
    let batch = runtime
        .pump(PumpBudget::new(
            usize::MAX,
            usize::MAX,
            usize::MAX,
            usize::MAX,
        ))
        .unwrap_or_else(|_| unreachable!("TreeView pump observation"));
    assert!(batch.report().is_quiescent());
}
fn mounted(runtime: &mut AppRuntime<App>, generation: u8) -> MountedNodeId {
    let id = runenui_core::ElementId::new(format!("realized.{generation}"))
        .unwrap_or_else(|_| unreachable!("valid realization name"));
    runtime
        .index()
        .nodes()
        .iter()
        .find(|node| node.authored_id() == Some(&id))
        .unwrap_or_else(|| unreachable!("current realization exists"))
        .id()
        .clone()
}
#[test]
fn domain_selection_survives_rebuild_with_new_mounted_identity() {
    let mut runtime = AppRuntime::<App>::mount(State::default());
    pump(&mut runtime);
    let original = mounted(&mut runtime, 0);
    runtime
        .submit_command(
            original.clone(),
            SemanticCommand::Activate,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("first activation admitted"));
    pump(&mut runtime);
    assert_eq!(
        runtime.state().selected.as_deref(),
        Some("stable-domain-item")
    );
    assert_eq!(
        runtime.state().actions,
        [TreeItemRequest::Select("stable-domain-item".to_owned())]
    );
    runtime
        .submit_action(Action::Rebuild)
        .unwrap_or_else(|_| unreachable!("re-author realized identity"));
    pump(&mut runtime);
    let next = mounted(&mut runtime, 1);
    assert_ne!(original, next, "realized identity actually changed");
    assert!(
        !runtime
            .index()
            .nodes()
            .iter()
            .any(|node| node.id() == &original),
        "stale realization retired"
    );
    runtime
        .submit_command(
            next,
            SemanticCommand::Activate,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("current activation admitted"));
    pump(&mut runtime);
    assert_eq!(
        runtime.state().actions,
        [
            TreeItemRequest::Select("stable-domain-item".to_owned()),
            TreeItemRequest::Select("stable-domain-item".to_owned()),
        ]
    );
}
