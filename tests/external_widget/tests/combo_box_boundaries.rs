#![allow(refining_impl_trait)]
//! ComboBox negative conformance through ordinary public application contracts.
use runenui_core::{
    ComboBoxRequest, CommandOrigin, ElementId, NoHostProtocol, SemanticCommand,
    SemanticRelationshipKind, SemanticRole, UiApp, View, combo_box, option_item,
};
use runenui_runtime::{AppRuntime, LogicalSize, MountedNodeId, PumpBudget, SurfaceBuildContext};

#[derive(Clone, Copy)]
enum Mode { Disabled, MissingId, Churn }
struct State { mode: Mode, replacement: bool, chosen: u8, actions: Vec<u8> }
enum Action { Request(ComboBoxRequest), Choose(u8), Replace }
struct App;
impl UiApp for App {
    type State = State;
    type Action = Action;
    type HostProtocol = NoHostProtocol;
    fn root(state: &Self::State) -> impl View<Self::Action> {
        let options = vec![
            if matches!(state.mode, Mode::MissingId) {
                option_item("First", false).on_activate(|| Action::Choose(1))
            } else {
                option_item("First", false).id("boundary.first")
                    .on_activate(|| Action::Choose(1))
            },
            option_item("Second", false)
                .id(if state.replacement { "boundary.replacement" } else { "boundary.second" })
                .on_activate(|| Action::Choose(2)),
        ];
        combo_box(
            "Boundary",
            ElementId::new("boundary.popup").unwrap_or_else(|_| unreachable!()),
            options, Action::Request,
        )
        .id("boundary.owner")
        .open(true)
        .enabled(!matches!(state.mode, Mode::Disabled))
    }
    fn update(state: &mut Self::State, action: Self::Action) {
        match action {
            Action::Choose(value) => { state.chosen = value; state.actions.push(value); }
            Action::Request(_) => state.actions.push(9),
            Action::Replace => state.replacement = true,
        }
    }
}
fn new(mode: Mode) -> AppRuntime<App> {
    let mut app = AppRuntime::<App>::mount(State {
        mode, replacement: false, chosen: 0, actions: Vec::new(),
    });
    settle(&mut app);
    app
}
fn settle(app: &mut AppRuntime<App>) {
    assert!(app.pump(PumpBudget::new(
        usize::MAX, usize::MAX, usize::MAX, usize::MAX,
    )).is_quiescent());
}
fn id(app: &AppRuntime<App>, name: &str) -> MountedNodeId {
    let id = ElementId::new(name).unwrap_or_else(|_| unreachable!());
    app.index().nodes().iter()
        .find(|node| node.authored_id() == Some(&id))
        .unwrap_or_else(|| unreachable!("authored ComboBox node exists"))
        .id().clone()
}
fn publish(app: &mut AppRuntime<App>) -> runenui_runtime::SurfacePublication {
    app.publish_surface(&SurfaceBuildContext::tight(
        &runenui_core::StyleEnvironment::default(),
        LogicalSize::try_new(500.0, 260.0).unwrap_or_else(|_| unreachable!()),
    )).unwrap_or_else(|e| unreachable!("boundary publication: {e:?}"))
}
#[test]
fn disabled_owner_blocks_option_activation_and_never_advertises_active_proxy() {
    let mut app = new(Mode::Disabled);
    let owner = id(&app, "boundary.owner");
    let option = id(&app, "boundary.first");
    assert!(!app.index().node(&owner).is_some_and(|node| node.is_focusable()));
    assert!(!app.index().node(&option).is_some_and(|node| node.is_focusable()));
    app.submit_command(option, SemanticCommand::Activate, CommandOrigin::programmatic())
        .unwrap_or_else(|_| unreachable!("disabled target exists"));
    settle(&mut app);
    assert!(app.state().actions.is_empty());
    let published = publish(&mut app);
    let snapshot = published.semantic_publication().snapshot();
    let combo = snapshot.nodes().iter().find(|n| n.role() == SemanticRole::ComboBox)
        .unwrap_or_else(|| unreachable!("disabled ComboBox semantically queryable"));
    assert_eq!(combo.state().disabled(), Some(true));
    assert!(!combo.relationships().iter()
        .any(|rel| rel.kind() == SemanticRelationshipKind::ActiveDescendant));
}
#[test]
fn missing_authored_option_identity_fails_closed_before_mounted_popup_creation() {
    let mut app = new(Mode::MissingId);
    let owner = id(&app, "boundary.owner");
    assert!(!app.index().node(&owner).is_some_and(|node| node.is_focusable()));
    let publication = publish(&mut app);
    assert!(publication.semantic_publication().snapshot().nodes().iter()
        .all(|n| n.role() != SemanticRole::ComboBox && n.role() != SemanticRole::Option));
}
#[test]
fn active_option_replacement_clears_proxy_without_committing_another_value() {
    let mut app = new(Mode::Churn);
    let owner = id(&app, "boundary.owner");
    app.submit_command(owner.clone(), SemanticCommand::RequestFocus,
        CommandOrigin::programmatic()).unwrap_or_else(|_| unreachable!());
    settle(&mut app);
    app.submit_command(owner.clone(), SemanticCommand::FocusDown,
        CommandOrigin::programmatic()).unwrap_or_else(|_| unreachable!());
    settle(&mut app);
    let old = id(&app, "boundary.second");
    app.submit_action(Action::Replace).unwrap_or_else(|_| unreachable!());
    settle(&mut app);
    assert!(!app.index().nodes().iter().any(|n| n.id() == &old));
    let current = publish(&mut app);
    let snapshot = current.semantic_publication().snapshot();
    let combo = snapshot.nodes().iter().find(|n| n.role() == SemanticRole::ComboBox)
        .unwrap_or_else(|| unreachable!("ComboBox still valid"));
    assert!(!combo.relationships().iter()
        .any(|rel| rel.kind() == SemanticRelationshipKind::ActiveDescendant),
        "an exact removed Option must not be retargeted by sibling position");
    app.submit_command(owner, SemanticCommand::Activate, CommandOrigin::programmatic())
        .unwrap_or_else(|_| unreachable!());
    settle(&mut app);
    assert_eq!(app.state().chosen, 0);
    assert!(app.state().actions.is_empty());
}
