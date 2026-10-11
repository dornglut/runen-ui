#![allow(refining_impl_trait)]
//! ComboBox negative conformance through ordinary public application contracts.
use runenui_core::{
    ComboBoxRequest, CommandOrigin, ElementId, NoHostProtocol, SemanticCommand,
    SemanticRelationshipKind, SemanticRole, UiApp, View, combo_box, option_item,
};
use runenui_runtime::{AppRuntime, LogicalSize, MountedNodeId, PumpBudget, SurfaceBuildContext};

#[derive(Clone, Copy)]
enum Mode { Disabled, MissingId, Churn, OwnerCollision, PopupOwnerCollision }
struct State { mode: Mode, replacement: bool, popup_replaced: bool, option_rekeyed: bool, chosen: u8, actions: Vec<u8> }
enum Action { Request(ComboBoxRequest), Choose(u8), Replace, ReplacePopup, RekeyOption }
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
                option_item("First", false)
                    .id(if matches!(state.mode, Mode::OwnerCollision) {
                        "boundary.owner"
                    } else {
                        "boundary.first"
                    })
                    .on_activate(|| Action::Choose(1))
            },
            option_item("Second", false)
                .id(if state.replacement { "boundary.replacement" } else { "boundary.second" })
                .key(if state.option_rekeyed { "boundary.option.new" } else { "boundary.option.old" })
                .on_activate(|| Action::Choose(2)),
        ];
        combo_box(
            "Boundary",
            ElementId::new(if matches!(state.mode, Mode::PopupOwnerCollision) {
                "boundary.owner"
            } else if state.popup_replaced {
                "boundary.new-popup"
            } else {
                "boundary.popup"
            })
                .unwrap_or_else(|_| unreachable!()),
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
            Action::ReplacePopup => state.popup_replaced = true,
            Action::RekeyOption => state.option_rekeyed = true,
        }
    }
}
fn new(mode: Mode) -> AppRuntime<App> {
    let mut app = AppRuntime::<App>::mount(State {
        mode, replacement: false, popup_replaced: false, option_rekeyed: false,
        chosen: 0, actions: Vec::new(),
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
    // Both command-admission refusal and a routed disabled no-op are legal.
    let _ = app.submit_command(option, SemanticCommand::Activate, CommandOrigin::programmatic());
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

#[test]
fn replacing_controlled_listbox_retires_old_transient_active_candidate() {
    let mut app = new(Mode::Churn);
    let owner = id(&app, "boundary.owner");
    app.submit_command(owner.clone(), SemanticCommand::RequestFocus,
        CommandOrigin::programmatic()).unwrap_or_else(|_| unreachable!());
    settle(&mut app);
    app.submit_command(owner.clone(), SemanticCommand::FocusDown,
        CommandOrigin::programmatic()).unwrap_or_else(|_| unreachable!());
    settle(&mut app);
    app.submit_action(Action::ReplacePopup).unwrap_or_else(|_| unreachable!());
    settle(&mut app);
    let current = publish(&mut app);
    let snapshot = current.semantic_publication().snapshot();
    let combo = snapshot.nodes().iter()
        .find(|node| node.role() == SemanticRole::ComboBox)
        .unwrap_or_else(|| unreachable!("new popup ComboBox"));
    let controlled = combo.relationships().iter()
        .find(|link| link.kind() == SemanticRelationshipKind::Controls)
        .unwrap_or_else(|| unreachable!("current controls target"));
    assert_eq!(snapshot.nodes().iter()
        .find(|node| node.id() == controlled.target())
        .map(runenui_runtime::SemanticNode::role), Some(SemanticRole::ListBox));
    let active = combo.relationships().iter()
        .find(|link| link.kind() == SemanticRelationshipKind::ActiveDescendant)
        .unwrap_or_else(|| unreachable!("current active candidate"));
    assert_eq!(snapshot.nodes().iter()
        .find(|node| node.id() == active.target()).and_then(|node| node.name()),
        Some("First"), "replacement must not carry old transient second-item navigation");
    assert_eq!(app.state().chosen, 0);
    assert!(app.state().actions.is_empty());
    assert_eq!(app.focus().focused_node(), Some(&owner));
}

#[test]
fn option_id_equal_to_owner_id_fails_closed_without_ambiguous_semantic_identity() {
    let mut app = new(Mode::OwnerCollision);
    let owner = id(&app, "boundary.owner");
    assert!(!app.index().node(&owner).is_some_and(|node| node.is_focusable()));
    assert_eq!(
        app.index().nodes().iter()
            .filter(|node| node.authored_id().is_some_and(|id| id.as_str() == "boundary.owner"))
            .count(),
        1,
        "invalid option must not mount with the owner's authored identity"
    );
    let publication = publish(&mut app);
    assert!(publication.semantic_publication().snapshot().nodes().iter().all(|node|
        node.role() != SemanticRole::ComboBox && node.role() != SemanticRole::Option
    ));
    assert!(app.state().actions.is_empty());
}

#[test]
fn popup_id_equal_to_owner_id_fails_closed_before_semantic_target_resolution() {
    let mut app = new(Mode::PopupOwnerCollision);
    let owner = id(&app, "boundary.owner");
    assert!(!app.index().node(&owner).is_some_and(|node| node.is_focusable()));
    let publication = publish(&mut app);
    assert!(publication.semantic_publication().snapshot().nodes().iter().all(|node|
        node.role() != SemanticRole::ComboBox && node.role() != SemanticRole::ListBox
    ));
    assert!(app.state().actions.is_empty());
}

#[test]
fn same_authored_id_with_new_reconciliation_key_must_retire_active_option_generation() {
    let mut app = new(Mode::Churn);
    let owner = id(&app, "boundary.owner");
    app.submit_command(
        owner.clone(), SemanticCommand::RequestFocus, CommandOrigin::programmatic(),
    ).unwrap_or_else(|_| unreachable!("focus current owner"));
    settle(&mut app);
    app.submit_command(
        owner.clone(), SemanticCommand::FocusDown, CommandOrigin::programmatic(),
    ).unwrap_or_else(|_| unreachable!("navigate to second"));
    settle(&mut app);

    let previous = id(&app, "boundary.second");
    let before = publish(&mut app);
    let combo = before.semantic_publication().snapshot().nodes().iter()
        .find(|node| node.role() == SemanticRole::ComboBox)
        .unwrap_or_else(|| unreachable!("ComboBox publishes"));
    assert_eq!(combo.relationships().iter()
        .filter(|link| link.kind() == SemanticRelationshipKind::ActiveDescendant).count(), 1);

    app.submit_action(Action::RekeyOption)
        .unwrap_or_else(|_| unreachable!("app changes standard Option reconciliation key"));
    settle(&mut app);
    let replacement = id(&app, "boundary.second");
    assert_ne!(previous, replacement,
        "accepted M3 keyed reconciliation retires the old Option mounted generation");

    let after = publish(&mut app);
    let combo = after.semantic_publication().snapshot().nodes().iter()
        .find(|node| node.role() == SemanticRole::ComboBox)
        .unwrap_or_else(|| unreachable!("ComboBox remains published"));
    assert!(!combo.relationships().iter()
        .any(|link| link.kind() == SemanticRelationshipKind::ActiveDescendant),
        "same authored ID must never retarget a replaced mounted Option generation");
    app.submit_command(owner.clone(), SemanticCommand::Activate, CommandOrigin::programmatic())
        .unwrap_or_else(|_| unreachable!("activation routed to current owner"));
    settle(&mut app);
    assert_eq!(app.state().chosen, 0);
    assert!(app.state().actions.is_empty(),
        "old active Option cannot commit through a replacement's new callback");
    assert_eq!(app.focus().focused_node(), Some(&owner));
}
