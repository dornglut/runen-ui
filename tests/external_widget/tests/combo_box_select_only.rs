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
                    ComboBoxRequest::Accept(id) => {
                        state.chosen = match id.as_str() {
                            "choice.one" => 1,
                            "choice.two" => 2,
                            _ => unreachable!("only current authored Options accepted"),
                        };
                        state.open = false;
                    }
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
    command(&mut app, owner.clone(), SemanticCommand::FocusDown);
    assert_eq!(app.state().chosen, 0, "active descendant is not committed value");
    command(&mut app, owner.clone(), SemanticCommand::Activate);
    assert_eq!(app.state().chosen, 2);
    assert!(!app.state().open);
    assert_eq!(app.state().log.last(), Some(&ComboBoxRequest::Accept(
        ElementId::new("choice.two").unwrap_or_else(|_| unreachable!()),
    )));
    assert_eq!(app.focus().focused_node(), Some(&owner));
}
