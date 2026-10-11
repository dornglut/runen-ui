#![allow(refining_impl_trait)]
//! Select-only ComboBox uses one focus owner and app-selected Option callbacks.
use runenui_core::{
    ComboBoxRequest, CommandOrigin, ElementId, NoHostProtocol, SemanticAction, SemanticCommand,
    SemanticRole, UiApp, View, combo_box, option_item,
};
use runenui_runtime::{
    AppRuntime, FontFamilyName, GenericFontFamily, LogicalSize, MountedNodeId, PumpBudget,
    SurfaceBuildContext,
};

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
        .find(|node| node.id() == controlled.target()).map(runenui_runtime::SemanticNode::role),
        Some(SemanticRole::ListBox),
        "M5 requires the actual ListBox, not a surrounding Popover Group");
    let active = combo.relationships().iter()
        .find(|link| link.kind() == runenui_core::SemanticRelationshipKind::ActiveDescendant)
        .unwrap_or_else(|| unreachable!("active proxy resolves without synthetic focus"));
    assert_eq!(snapshot.nodes().iter()
        .find(|node| node.id() == active.target()).and_then(|node| node.name()),
        Some("One"));
}

fn key(physical: runenui_core::PhysicalKey, logical: runenui_core::LogicalKey,
    composition: runenui_core::KeyboardCompositionState) -> runenui_core::KeyboardEvent {
    runenui_core::KeyboardEvent::new(
        runenui_core::KeyboardPhase::Down, physical, logical,
        runenui_core::KeyModifiers::NONE, false,
        runenui_core::KeyLocation::Standard, composition, None,
    )
}
fn active_label(app: &mut AppRuntime<App>) -> Option<String> {
    let publication = app.publish_surface(&SurfaceBuildContext::tight(
        &runenui_core::StyleEnvironment::default(),
        LogicalSize::try_new(500.0, 260.0).unwrap_or_else(|_| unreachable!()),
    )).unwrap_or_else(|error| unreachable!("current ComboBox semantics: {error:?}"));
    let snapshot = publication.semantic_publication().snapshot();
    let owner = snapshot.nodes().iter().find(|node| node.role() == SemanticRole::ComboBox)?;
    let target = owner.relationships().iter()
        .find(|link| link.kind() == runenui_core::SemanticRelationshipKind::ActiveDescendant)?
        .target();
    snapshot.nodes().iter().find(|node| node.id() == target)
        .and_then(|node| node.name()).map(str::to_owned)
}
#[test]
fn keyboard_ime_escape_and_enter_keep_focus_and_committed_value_separate() {
    use runenui_core::{KeyboardCompositionState as Composition, LogicalKey, PhysicalKey};
    let mut app = AppRuntime::<App>::mount(State::default());
    settle(&mut app);
    let owner = id(&app, "choice.owner");
    command(&mut app, owner.clone(), SemanticCommand::RequestFocus);
    app.submit_keyboard(key(PhysicalKey::ArrowDown, LogicalKey::ArrowDown,
        Composition::Inactive)).unwrap_or_else(|_| unreachable!("arrow ingress"));
    settle(&mut app);
    assert!(app.state().open);
    assert_eq!(active_label(&mut app).as_deref(), Some("One"));
    app.submit_keyboard(key(PhysicalKey::ArrowDown, LogicalKey::ArrowDown,
        Composition::Active)).unwrap_or_else(|_| unreachable!("composition ingress"));
    settle(&mut app);
    assert_eq!(active_label(&mut app).as_deref(), Some("One"));
    app.submit_keyboard(key(PhysicalKey::ArrowDown, LogicalKey::ArrowDown,
        Composition::Inactive)).unwrap_or_else(|_| unreachable!("navigation ingress"));
    settle(&mut app);
    assert_eq!(active_label(&mut app).as_deref(), Some("Two"));
    assert_eq!(app.state().chosen, 0, "navigation is not committed selection");
    app.submit_keyboard(key(PhysicalKey::Escape, LogicalKey::Escape,
        Composition::Inactive)).unwrap_or_else(|_| unreachable!("Escape ingress"));
    settle(&mut app);
    assert!(!app.state().open);
    assert_eq!(app.state().chosen, 0, "Escape never commits the active option");
    assert_eq!(app.focus().focused_node(), Some(&owner));

    app.submit_keyboard(key(PhysicalKey::ArrowDown, LogicalKey::ArrowDown,
        Composition::Inactive)).unwrap_or_else(|_| unreachable!("reopen ingress"));
    settle(&mut app);
    app.submit_keyboard(key(PhysicalKey::ArrowDown, LogicalKey::ArrowDown,
        Composition::Inactive)).unwrap_or_else(|_| unreachable!("second selection ingress"));
    settle(&mut app);
    app.submit_keyboard(key(PhysicalKey::Enter, LogicalKey::Enter,
        Composition::Inactive)).unwrap_or_else(|_| unreachable!("accept ingress"));
    settle(&mut app);
    assert_eq!(app.state().chosen, 2,
        "Enter must activate the existing Option callback, never copy the active authored ID");
    assert!(!app.state().open);
    assert_eq!(app.focus().focused_node(), Some(&owner));
}

#[test]
fn semantic_expand_and_collapse_actions_follow_application_open_state() {
    let mut app = AppRuntime::<App>::mount(State::default());
    settle(&mut app);
    let owner = id(&app, "choice.owner");
    command(&mut app, owner.clone(), SemanticCommand::RequestFocus);
    let published = app.publish_surface(&SurfaceBuildContext::tight(
        &runenui_core::StyleEnvironment::default(),
        LogicalSize::try_new(500.0, 260.0).unwrap_or_else(|_| unreachable!()),
    )).unwrap_or_else(|e| unreachable!("closed ComboBox semantics: {e:?}"));
    let snapshot = published.semantic_publication().snapshot();
    let combo = snapshot.nodes().iter()
        .find(|node| node.role() == SemanticRole::ComboBox)
        .unwrap_or_else(|| unreachable!("ComboBox role"));
    assert!(combo.supported_actions().contains(&SemanticAction::Expand));
    assert!(!combo.supported_actions().contains(&SemanticAction::Collapse));
    command(&mut app, owner.clone(), SemanticCommand::Expand);
    assert!(app.state().open);
    assert_eq!(app.state().log, [ComboBoxRequest::Open]);
    let published = app.publish_surface(&SurfaceBuildContext::tight(
        &runenui_core::StyleEnvironment::default(),
        LogicalSize::try_new(500.0, 260.0).unwrap_or_else(|_| unreachable!()),
    )).unwrap_or_else(|e| unreachable!("expanded ComboBox semantics: {e:?}"));
    let snapshot = published.semantic_publication().snapshot();
    let combo = snapshot.nodes().iter()
        .find(|node| node.role() == SemanticRole::ComboBox)
        .unwrap_or_else(|| unreachable!("ComboBox role"));
    assert!(combo.supported_actions().contains(&SemanticAction::Collapse));
    command(&mut app, owner.clone(), SemanticCommand::Collapse);
    assert!(!app.state().open);
    assert_eq!(app.state().log, [ComboBoxRequest::Open, ComboBoxRequest::Close]);
    assert_eq!(app.focus().focused_node(), Some(&owner));
}

const FONT: &[u8] =
    include_bytes!("../../../crates/runenui_text/tests/fixtures/Cantarell-Regular.ttf");

fn collapsed_display_glyphs(chosen: u8) -> Vec<u32> {
    let mut app = AppRuntime::<App>::mount(State {
        chosen,
        ..State::default()
    });
    app.register_text_font_bytes(FONT.to_vec())
        .unwrap_or_else(|_| unreachable!("checked font fixture"));
    app.set_text_generic_family_mapping(
        GenericFontFamily::SansSerif,
        &[FontFamilyName::new("Cantarell").unwrap_or_else(|_| unreachable!())],
    )
    .unwrap_or_else(|_| unreachable!("checked font family"));
    settle(&mut app);
    let publication = app
        .publish_surface(&SurfaceBuildContext::tight(
            &runenui_core::StyleEnvironment::default(),
            LogicalSize::try_new(500.0, 240.0).unwrap_or_else(|_| unreachable!()),
        ))
        .unwrap_or_else(|error| unreachable!("closed combo paint: {error:?}"));
    publication
        .paint_scene()
        .items()
        .iter()
        .filter_map(|item| item.primitive().as_shaped_text_run())
        .flat_map(|run| {
            publication
                .paint_scene()
                .shaped_text_resource(run.resource_ref())
                .into_iter()
                .flat_map(|resource| resource.glyphs().iter().map(|glyph| glyph.id()))
        })
        .collect()
}

#[test]
fn closed_select_only_displays_app_selected_option_using_retained_m8_text_not_static_name() {
    let unselected = collapsed_display_glyphs(0);
    let one = collapsed_display_glyphs(1);
    let two = collapsed_display_glyphs(2);
    assert!(!unselected.is_empty());
    assert!(unselected.len() > one.len(),
        "the unselected fallback prompt must not replace the chosen option");
    assert_eq!(one.len(), two.len(),
        "both selected options use their own ordinary three-letter label");
    assert_ne!(one, two,
        "changing only app-authored selected Option changes retained text glyphs");
}
