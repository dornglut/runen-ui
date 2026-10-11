#![allow(refining_impl_trait)]
//! Select-only ComboBox uses one focus owner and app-selected Option callbacks.
use runenui_core::{
    ComboBoxRequest, CommandOrigin, ElementId, NoHostProtocol, SemanticAction, SemanticCommand,
    SemanticRole, UiApp, View, combo_box, option_item,
};
use runenui_runtime::{AppRuntime, LogicalSize, MountedNodeId, PumpBudget, SurfaceBuildContext};

#[derive(Clone, Debug, Eq, PartialEq)]
enum Action { Request(ComboBoxRequest), Choose(u8) }
#[derive(Default)]
struct State { open: bool, chosen: u8, log: Vec<ComboBoxRequest> }
struct App;
impl UiApp for App {
    type State = State;
    type Action = Action;
    type HostProtocol = NoHostProtocol;
    fn root(state: &Self::State) -> impl View<Self::Action> {
        combo_box("Choose a value",
            ElementId::new("choice.popup").unwrap_or_else(|_| unreachable!()),
            [
                option_item("One", state.chosen == 1)
                    .id("choice.one").on_activate(|| Action::Choose(1)),
                option_item("Two", state.chosen == 2)
                    .id("choice.two").on_activate(|| Action::Choose(2)),
            ],
            Action::Request,
        )
        .id("choice.owner")
        .open(state.open)
    }
    fn update(state: &mut Self::State, action: Self::Action) {
        match action {
            Action::Request(request) => {
                match &request {
                    ComboBoxRequest::Open => state.open = true,
                    ComboBoxRequest::Close => state.open = false,
                }
                state.log.push(request);
            }
            Action::Choose(value) => {
                state.chosen = value;
                state.open = false;
            }
        }
    }
}
fn settle(app: &mut AppRuntime<App>) {
    assert!(app.pump(PumpBudget::new(
        usize::MAX, usize::MAX, usize::MAX, usize::MAX,
    )).is_quiescent());
}
fn id(app: &AppRuntime<App>, id: &str) -> MountedNodeId {
    let id = ElementId::new(id).unwrap_or_else(|_| unreachable!("valid test ID"));
    app.index().nodes().iter()
        .find(|node| node.authored_id() == Some(&id))
        .unwrap_or_else(|| unreachable!("authored control mounted"))
        .id().clone()
}
fn command(app: &mut AppRuntime<App>, target: MountedNodeId, cmd: SemanticCommand) {
    app.submit_command(target, cmd, CommandOrigin::programmatic())
        .unwrap_or_else(|_| unreachable!("semantic action admitted"));
    settle(app);
}
#[test]
fn select_only_keeps_real_focus_on_owner_while_selection_remains_application_owned() {
    let mut app = AppRuntime::<App>::mount(State::default());
    settle(&mut app);
    let owner = id(&app, "choice.owner");
    command(&mut app, owner.clone(), SemanticCommand::RequestFocus);
    command(&mut app, owner.clone(), SemanticCommand::Activate);
    assert!(app.state().open);
    assert_eq!(app.state().log, [ComboBoxRequest::Open]);
    assert_eq!(app.focus().focused_node(), Some(&owner));
    let one = id(&app, "choice.one");
    let two = id(&app, "choice.two");
    assert!(!app.index().node(&one).is_some_and(|node| node.is_focusable()));
    assert!(!app.index().node(&two).is_some_and(|node| node.is_focusable()));
    let published = app.publish_surface(&SurfaceBuildContext::tight(
        &runenui_core::StyleEnvironment::default(),
        LogicalSize::try_new(500.0, 260.0).unwrap_or_else(|_| unreachable!()),
    )).unwrap_or_else(|e| unreachable!("ComboBox semantic publication: {e:?}"));
    let semantics = published.semantic_publication().snapshot();
    assert!(semantics.nodes().iter().any(|node| node.role() == SemanticRole::ComboBox));
    assert_eq!(semantics.nodes().iter().filter(|n| n.role() == SemanticRole::Option).count(), 2);
    assert!(semantics.nodes().iter().all(|node|
        node.role() != SemanticRole::Option
            || !node.supported_actions().contains(&SemanticAction::RequestFocus)
    ));
    let combo = semantics.nodes().iter().find(|node| node.role() == SemanticRole::ComboBox)
        .unwrap_or_else(|| unreachable!("ComboBox semantic owner"));
    let first = semantics.nodes().iter()
        .find(|node| node.role() == SemanticRole::Option && node.name() == Some("One"))
        .unwrap_or_else(|| unreachable!("first Option"));
    assert_eq!(combo.relationships().iter()
        .find(|link| link.kind() == runenui_core::SemanticRelationshipKind::ActiveDescendant)
        .map(|link| link.target()), Some(first.id()));
    command(&mut app, owner.clone(), SemanticCommand::FocusDown);
    assert_eq!(app.state().chosen, 0, "active descendant is not committed value");
    let updated = app.publish_surface(&SurfaceBuildContext::tight(
        &runenui_core::StyleEnvironment::default(),
        LogicalSize::try_new(500.0, 260.0).unwrap_or_else(|_| unreachable!()),
    )).unwrap_or_else(|error| unreachable!("active semantic publication: {error:?}"));
    let snapshot = updated.semantic_publication().snapshot();
    let combo = snapshot.nodes().iter().find(|node| node.role() == SemanticRole::ComboBox)
        .unwrap_or_else(|| unreachable!("current ComboBox semantic"));
    let second = snapshot.nodes().iter()
        .find(|node| node.role() == SemanticRole::Option && node.name() == Some("Two"))
        .unwrap_or_else(|| unreachable!("second Option"));
    assert_eq!(combo.relationships().iter()
        .find(|link| link.kind() == runenui_core::SemanticRelationshipKind::ActiveDescendant)
        .map(|link| link.target()), Some(second.id()),
        "transient active Option changes while the app value remains uncommitted");
    command(&mut app, owner.clone(), SemanticCommand::Activate);
    assert_eq!(app.state().chosen, 2);
    assert!(!app.state().open);
    assert_eq!(app.state().log, [ComboBoxRequest::Open],
        "keyboard and pointer activation use the same ordinary Option callback");
    assert_eq!(app.focus().focused_node(), Some(&owner));
}

#[test]
fn controls_identifies_actual_listbox_and_active_option_without_focus_repair() {
    let mut app = AppRuntime::<App>::mount(State {
        open: true,
        chosen: 1,
        log: Vec::new(),
    });
    settle(&mut app);
    assert!(app.focus().focused_node().is_none());
    let published = app.publish_surface(&SurfaceBuildContext::tight(
        &runenui_core::StyleEnvironment::default(),
        LogicalSize::try_new(500.0, 260.0).unwrap_or_else(|_| unreachable!()),
    )).unwrap_or_else(|error| unreachable!("current popup semantics: {error:?}"));
    let snapshot = published.semantic_publication().snapshot();
    let combo = snapshot.nodes().iter()
        .find(|node| node.role() == SemanticRole::ComboBox)
        .unwrap_or_else(|| unreachable!("focused-or-not ComboBox semantics"));
    let controlled = combo.relationships().iter()
        .find(|link| link.kind() == runenui_core::SemanticRelationshipKind::Controls)
        .unwrap_or_else(|| unreachable!("ComboBox Controls relation"));
    assert_eq!(snapshot.nodes().iter()
        .find(|node| node.id() == controlled.target()).map(|node| node.role()),
        Some(SemanticRole::ListBox),
        "M5 requires the actual ListBox, not a surrounding Popover Group");
    let active = combo.relationships().iter()
        .find(|link| link.kind() == runenui_core::SemanticRelationshipKind::ActiveDescendant)
        .unwrap_or_else(|| unreachable!("active proxy resolves without synthetic focus"));
    assert_eq!(snapshot.nodes().iter()
        .find(|node| node.id() == active.target()).and_then(|node| node.name()),
        Some("One"));
}
