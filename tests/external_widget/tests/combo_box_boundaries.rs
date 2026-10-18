#![allow(refining_impl_trait)]
//! ComboBox negative conformance through ordinary public application contracts.
use runenui_core::{
    ComboBoxRequest, CommandOrigin, ElementId, NoHostProtocol, SemanticCommand,
    SemanticRelationshipKind, SemanticRole, UiApp, View, combo_box, option_item,
};
use runenui_runtime::{AppRuntime, LogicalSize, MountedNodeId, PumpBudget, SurfaceBuildContext};

#[derive(Clone, Copy)]
enum Mode { Disabled, MissingId, DuplicateKey, Churn, OwnerCollision, PopupOwnerCollision }
struct State { mode: Mode, replacement: bool, popup_replaced: bool, option_rekeyed: bool, reordered: bool, chosen: u8, actions: Vec<u8> }
enum Action { Request(ComboBoxRequest), Choose(u8), Replace, ReplacePopup, RekeyOption, Reorder, Disable, Enable }
struct App;
impl UiApp for App {
    type State = State;
    type Action = Action;
    type HostProtocol = NoHostProtocol;
    fn root(state: &Self::State) -> impl View<Self::Action> {
        let first = if matches!(state.mode, Mode::MissingId) {
            option_item("First", false).on_activate(|| Action::Choose(1))
        } else {
            option_item("First", false)
                .id(if matches!(state.mode, Mode::OwnerCollision) {
                    "boundary.owner"
                } else {
                    "boundary.first"
                })
                .on_activate(|| Action::Choose(1))
        };
        let first = if matches!(state.mode, Mode::DuplicateKey) {
            first.key("boundary.option.initial")
        } else {
            first
        };
        let mut options = vec![
            first,
            option_item("Second", false)
                .id(if state.replacement { "boundary.replacement" } else { "boundary.second" })
                .key(if state.replacement {
                    "boundary.option.replacement"
                } else if state.option_rekeyed {
                    "boundary.option.rekeyed"
                } else {
                    "boundary.option.initial"
                })
                .on_activate(|| Action::Choose(2)),
        ];
        if state.reordered {
            options.reverse();
        }
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
            Action::Reorder => state.reordered = true,
            Action::Disable => state.mode = Mode::Disabled,
            Action::Enable => state.mode = Mode::Churn,
        }
    }
}
fn new(mode: Mode) -> AppRuntime<App> {
    let mut app = AppRuntime::<App>::mount(State {
        mode, replacement: false, popup_replaced: false, option_rekeyed: false, reordered: false,
        chosen: 0, actions: Vec::new(),
    });
    settle(&mut app);
    app
}
fn settle(app: &mut AppRuntime<App>) {
    assert!(app.pump(PumpBudget::new(
        usize::MAX, usize::MAX, usize::MAX, usize::MAX,
    )).unwrap_or_else(|error| unreachable!("ComboBox pump: {error:?}"))
            .report()
            .is_quiescent());
}
fn id(app: &mut AppRuntime<App>, name: &str) -> MountedNodeId {
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
    let owner = id(&mut app, "boundary.owner");
    assert!(!app.index().node(&owner).is_some_and(|node| node.is_focusable()));
    assert!(app.index().nodes().iter().all(|node| node.authored_id().is_none_or(
        |id| id.as_str() != "boundary.first" && id.as_str() != "boundary.popup"
    )), "disabled owner cannot retain an undismissable blocking popup");
    // A disabled owner cannot activate or open the popup through the routed path.
    let _ = app.submit_command(owner.clone(), SemanticCommand::Activate, CommandOrigin::programmatic());
    settle(&mut app);
    assert!(app.state().actions.is_empty());
    let published = publish(&mut app);
    let snapshot = published.semantic_publication().snapshot();
    let combo = snapshot
        .nodes()
        .iter()
        .find(|n| n.role() == SemanticRole::ComboBox)
        .unwrap_or_else(|| unreachable!("disabled ComboBox semantically queryable"));
    assert_eq!(combo.state().disabled(), true);
    assert_eq!(combo.state().expanded(), Some(false));
    assert!(!combo.relationships().iter().any(|rel| rel.kind()
        == SemanticRelationshipKind::ActiveDescendant
        || rel.kind() == SemanticRelationshipKind::Controls));
    assert!(snapshot.nodes().iter().all(|node|
        node.role() != SemanticRole::Option && node.role() != SemanticRole::ListBox
    ));
}
#[test]
fn missing_authored_option_identity_fails_closed_before_mounted_popup_creation() {
    let mut app = new(Mode::MissingId);
    let owner = id(&mut app, "boundary.owner");
    assert!(
        !app.index()
            .node(&owner)
            .is_some_and(|node| node.is_focusable())
    );
    let publication = publish(&mut app);
    assert!(
        publication
            .semantic_publication()
            .snapshot()
            .nodes()
            .iter()
            .all(|n| n.role() != SemanticRole::ComboBox && n.role() != SemanticRole::Option)
    );
}
#[test]
fn active_option_replacement_clears_proxy_without_committing_another_value() {
    let mut app = new(Mode::Churn);
    let owner = id(&mut app, "boundary.owner");
    app.submit_command(
        owner.clone(),
        SemanticCommand::RequestFocus,
        CommandOrigin::programmatic(),
    )
    .unwrap_or_else(|_| unreachable!());
    settle(&mut app);
    app.submit_command(
        owner.clone(),
        SemanticCommand::FocusDown,
        CommandOrigin::programmatic(),
    )
    .unwrap_or_else(|_| unreachable!());
    settle(&mut app);
    let old = id(&mut app, "boundary.second");
    app.submit_action(Action::Replace)
        .unwrap_or_else(|_| unreachable!());
    settle(&mut app);
    assert!(!app.index().nodes().iter().any(|n| n.id() == &old));
    let current = publish(&mut app);
    let snapshot = current.semantic_publication().snapshot();
    let combo = snapshot
        .nodes()
        .iter()
        .find(|n| n.role() == SemanticRole::ComboBox)
        .unwrap_or_else(|| unreachable!("ComboBox still valid"));
    assert!(
        !combo
            .relationships()
            .iter()
            .any(|rel| rel.kind() == SemanticRelationshipKind::ActiveDescendant),
        "an exact removed Option must not be retargeted by sibling position"
    );
    app.submit_command(
        owner,
        SemanticCommand::Activate,
        CommandOrigin::programmatic(),
    )
    .unwrap_or_else(|_| unreachable!());
    settle(&mut app);
    assert_eq!(app.state().chosen, 0);
    assert!(app.state().actions.is_empty());
}

#[test]
fn replacing_controlled_listbox_retires_old_transient_active_candidate() {
    let mut app = new(Mode::Churn);
    let owner = id(&mut app, "boundary.owner");
    app.submit_command(
        owner.clone(),
        SemanticCommand::RequestFocus,
        CommandOrigin::programmatic(),
    )
    .unwrap_or_else(|_| unreachable!());
    settle(&mut app);
    app.submit_command(
        owner.clone(),
        SemanticCommand::FocusDown,
        CommandOrigin::programmatic(),
    )
    .unwrap_or_else(|_| unreachable!());
    settle(&mut app);
    app.submit_action(Action::ReplacePopup)
        .unwrap_or_else(|_| unreachable!());
    settle(&mut app);
    let current = publish(&mut app);
    let snapshot = current.semantic_publication().snapshot();
    let combo = snapshot
        .nodes()
        .iter()
        .find(|node| node.role() == SemanticRole::ComboBox)
        .unwrap_or_else(|| unreachable!("new popup ComboBox"));
    let controlled = combo
        .relationships()
        .iter()
        .find(|link| link.kind() == SemanticRelationshipKind::Controls)
        .unwrap_or_else(|| unreachable!("current controls target"));
    assert_eq!(
        snapshot
            .nodes()
            .iter()
            .find(|node| node.id() == controlled.target())
            .map(runenui_runtime::SemanticNode::role),
        Some(SemanticRole::ListBox)
    );
    let active = combo
        .relationships()
        .iter()
        .find(|link| link.kind() == SemanticRelationshipKind::ActiveDescendant)
        .unwrap_or_else(|| unreachable!("current active candidate"));
    assert_eq!(
        snapshot
            .nodes()
            .iter()
            .find(|node| node.id() == active.target())
            .and_then(|node| node.name()),
        Some("First"),
        "replacement must not carry old transient second-item navigation"
    );
    assert_eq!(app.state().chosen, 0);
    assert!(app.state().actions.is_empty());
    assert_eq!(app.focus().focused_node(), Some(&owner));
}

#[test]
fn option_id_equal_to_owner_id_fails_closed_without_ambiguous_semantic_identity() {
    let mut app = new(Mode::OwnerCollision);
    let owner = id(&mut app, "boundary.owner");
    assert!(
        !app.index()
            .node(&owner)
            .is_some_and(|node| node.is_focusable())
    );
    assert_eq!(
        app.index()
            .nodes()
            .iter()
            .filter(|node| node
                .authored_id()
                .is_some_and(|id| id.as_str() == "boundary.owner"))
            .count(),
        1,
        "invalid option must not mount with the owner's authored identity"
    );
    let publication = publish(&mut app);
    assert!(
        publication
            .semantic_publication()
            .snapshot()
            .nodes()
            .iter()
            .all(
                |node| node.role() != SemanticRole::ComboBox && node.role() != SemanticRole::Option
            )
    );
    assert!(app.state().actions.is_empty());
}

#[test]
fn popup_id_equal_to_owner_id_fails_closed_before_semantic_target_resolution() {
    let mut app = new(Mode::PopupOwnerCollision);
    let owner = id(&mut app, "boundary.owner");
    assert!(
        !app.index()
            .node(&owner)
            .is_some_and(|node| node.is_focusable())
    );
    let publication = publish(&mut app);
    assert!(
        publication
            .semantic_publication()
            .snapshot()
            .nodes()
            .iter()
            .all(|node| node.role() != SemanticRole::ComboBox
                && node.role() != SemanticRole::ListBox)
    );
    assert!(app.state().actions.is_empty());
}

#[test]
fn same_authored_id_with_new_reconciliation_key_must_retire_active_option_generation() {
    let mut app = new(Mode::Churn);
    let owner = id(&mut app, "boundary.owner");
    app.submit_command(
        owner.clone(),
        SemanticCommand::RequestFocus,
        CommandOrigin::programmatic(),
    )
    .unwrap_or_else(|_| unreachable!("focus current owner"));
    settle(&mut app);
    app.submit_command(
        owner.clone(),
        SemanticCommand::FocusDown,
        CommandOrigin::programmatic(),
    )
    .unwrap_or_else(|_| unreachable!("navigate to second"));
    settle(&mut app);

    let previous = id(&mut app, "boundary.second");
    let before = publish(&mut app);
    let combo = before
        .semantic_publication()
        .snapshot()
        .nodes()
        .iter()
        .find(|node| node.role() == SemanticRole::ComboBox)
        .unwrap_or_else(|| unreachable!("ComboBox publishes"));
    assert_eq!(
        combo
            .relationships()
            .iter()
            .filter(|link| link.kind() == SemanticRelationshipKind::ActiveDescendant)
            .count(),
        1
    );

    app.submit_action(Action::RekeyOption)
        .unwrap_or_else(|_| unreachable!("app changes standard Option reconciliation key"));
    settle(&mut app);
    let replacement = id(&mut app, "boundary.second");
    assert_ne!(
        previous, replacement,
        "accepted M3 keyed reconciliation retires the old Option mounted generation"
    );

    let after = publish(&mut app);
    let combo = after
        .semantic_publication()
        .snapshot()
        .nodes()
        .iter()
        .find(|node| node.role() == SemanticRole::ComboBox)
        .unwrap_or_else(|| unreachable!("ComboBox remains published"));
    assert!(
        !combo
            .relationships()
            .iter()
            .any(|link| link.kind() == SemanticRelationshipKind::ActiveDescendant),
        "same authored ID must never retarget a replaced mounted Option generation"
    );
    app.submit_command(
        owner.clone(),
        SemanticCommand::Activate,
        CommandOrigin::programmatic(),
    )
    .unwrap_or_else(|_| unreachable!("activation routed to current owner"));
    settle(&mut app);
    assert_eq!(app.state().chosen, 0);
    assert!(
        app.state().actions.is_empty(),
        "old active Option cannot commit through a replacement's new callback"
    );
    assert_eq!(app.focus().focused_node(), Some(&owner));
}

#[test]
fn same_key_option_move_keeps_active_candidate_and_m4_focus_without_reselection() {
    let mut app = new(Mode::Churn);
    let owner = id(&mut app, "boundary.owner");
    app.submit_command(
        owner.clone(),
        SemanticCommand::RequestFocus,
        CommandOrigin::programmatic(),
    )
    .unwrap_or_else(|_| unreachable!("focus current owner"));
    settle(&mut app);
    app.submit_command(
        owner.clone(),
        SemanticCommand::FocusDown,
        CommandOrigin::programmatic(),
    )
    .unwrap_or_else(|_| unreachable!("navigate to keyed second"));
    settle(&mut app);
    let original = id(&mut app, "boundary.second");

    app.submit_action(Action::Reorder)
        .unwrap_or_else(|_| unreachable!("reorder ordinary authored Options"));
    settle(&mut app);
    assert_eq!(
        id(&mut app, "boundary.second"),
        original,
        "M3 keyed same-parent reorder preserves mounted Option generation"
    );
    let snapshot = publish(&mut app);
    let nodes = snapshot.semantic_publication().snapshot();
    let combo = nodes
        .nodes()
        .iter()
        .find(|node| node.role() == SemanticRole::ComboBox)
        .unwrap_or_else(|| unreachable!("ComboBox remains published"));
    let active = combo
        .relationships()
        .iter()
        .find(|link| link.kind() == SemanticRelationshipKind::ActiveDescendant)
        .unwrap_or_else(|| unreachable!("same keyed active Option survives reorder"));
    assert_eq!(
        nodes
            .nodes()
            .iter()
            .find(|node| node.id() == active.target())
            .and_then(|node| node.name()),
        Some("Second")
    );
    assert_eq!(app.state().chosen, 0);
    assert!(app.state().actions.is_empty());
    assert_eq!(app.focus().focused_node(), Some(&owner));

    app.submit_command(
        owner.clone(),
        SemanticCommand::Activate,
        CommandOrigin::programmatic(),
    )
    .unwrap_or_else(|_| unreachable!("current active Option activation"));
    settle(&mut app);
    assert_eq!(app.state().chosen, 2);
    assert_eq!(app.state().actions, [2]);
    assert_eq!(app.focus().focused_node(), Some(&owner));
}

#[test]
fn duplicate_option_reconciliation_keys_fail_closed_before_mounting_popup_proxy() {
    let mut app = new(Mode::DuplicateKey);
    let owner = id(&mut app, "boundary.owner");
    assert!(
        !app.index()
            .node(&owner)
            .is_some_and(|node| node.is_focusable())
    );
    assert!(
        app.index().nodes().iter().all(|node| node
            .authored_id()
            .is_none_or(|id| id.as_str() != "boundary.first")),
        "invalid popup Options are not mounted"
    );
    let published = publish(&mut app);
    assert!(
        published
            .semantic_publication()
            .snapshot()
            .nodes()
            .iter()
            .all(|node| node.role() != SemanticRole::ComboBox
                && node.role() != SemanticRole::Option
                && node.role() != SemanticRole::ListBox)
    );
    assert!(
        app.state().actions.is_empty(),
        "ambiguous sibling reconciliation keys cannot activate an Option"
    );
}

// Even if the application retains its requested open state, disabling the
// real M4 owner must withdraw the Popover and its pointer-blocking presentation.
#[test]
fn disabling_an_open_combo_unmounts_blocking_popup_and_reenabling_restores_app_authored_state() {
    let mut app = new(Mode::Churn);
    let owner = id(&mut app, "boundary.owner");
    let old_popup = id(&mut app, "boundary.popup");
    let old_option = id(&mut app, "boundary.first");
    let before = publish(&mut app);
    let node = before
        .semantic_publication()
        .snapshot()
        .nodes()
        .iter()
        .find(|node| node.role() == SemanticRole::ComboBox)
        .unwrap_or_else(|| unreachable!("enabled open ComboBox"));
    assert_eq!(node.state().expanded(), Some(true));
    assert!(
        node.relationships()
            .iter()
            .any(|rel| rel.kind() == SemanticRelationshipKind::Controls)
    );

    app.submit_action(Action::Disable)
        .unwrap_or_else(|_| unreachable!("disable action admitted"));
    settle(&mut app);
    assert!(
        !app.index()
            .node(&owner)
            .is_some_and(|node| node.is_focusable())
    );
    assert!(
        app.index()
            .nodes()
            .iter()
            .all(|node| node.id() != &old_option && node.id() != &old_popup)
    );
    let disabled = publish(&mut app);
    let semantics = disabled.semantic_publication().snapshot();
    let combo = semantics
        .nodes()
        .iter()
        .find(|node| node.role() == SemanticRole::ComboBox)
        .unwrap_or_else(|| unreachable!("disabled owner remains semantic"));
    assert_eq!(combo.state().disabled(), true);
    assert_eq!(combo.state().expanded(), Some(false));
    assert!(
        combo
            .relationships()
            .iter()
            .all(|rel| rel.kind() != SemanticRelationshipKind::Controls
                && rel.kind() != SemanticRelationshipKind::ActiveDescendant)
    );
    assert!(semantics.nodes().iter().all(|node|
        node.role() != SemanticRole::ListBox && node.role() != SemanticRole::Option));
    assert!(app.state().actions.is_empty());
    assert_eq!(app.state().chosen, 0);

    // App-owned open(true) remains authored: re-enabling restores a *fresh*
    // mounted popup and never resurrects a stale old-generation Option.
    app.submit_action(Action::Enable)
        .unwrap_or_else(|_| unreachable!("reenable action admitted"));
    settle(&mut app);
    let popup = id(&mut app, "boundary.popup");
    let option = id(&mut app, "boundary.first");
    assert_ne!(popup, old_popup);
    assert_ne!(option, old_option);
    let enabled = publish(&mut app);
    let semantics = enabled.semantic_publication().snapshot();
    let combo = semantics
        .nodes()
        .iter()
        .find(|node| node.role() == SemanticRole::ComboBox)
        .unwrap_or_else(|| unreachable!("re-enabled ComboBox"));
    assert_eq!(combo.state().disabled(), false);
    assert_eq!(combo.state().expanded(), Some(true));
    assert!(
        combo
            .relationships()
            .iter()
            .any(|rel| rel.kind() == SemanticRelationshipKind::Controls)
    );
    assert!(
        app.state().actions.is_empty(),
        "re-authoring visibility is not a selection action"
    );
    assert_eq!(app.state().chosen, 0);
}
