#![allow(refining_impl_trait)]
//! Editable ComboBox must be one M10 TextField / M5 ComboBox semantic owner.
use runenui_core::{
    ComboBoxRequest, CommandOrigin, CommittedTextEvent, EditIntent, EditResolution,
    Effects, ElementId, IntoUpdateOutput, KeyboardCompositionState, KeyboardEvent,
    KeyboardPhase, KeyLocation, KeyModifiers, LogicalKey, NoHostProtocol,
    PhysicalKey, SemanticAutocomplete, SemanticEditableMode,
    SemanticRelationshipKind, SemanticRole, SemanticCommand, StyleEnvironment,
    TextAffinity, TextDocumentId, TextDocumentRevision, TextDocumentSnapshot,
    TextPosition, TextSelection, UiApp, UpdateOutput, View,
    editable_combo_box, option_item, text_field,
};
use runenui_runtime::{
    AppRuntime, FontFamilyName, GenericFontFamily, LogicalSize, MountedNodeId,
    PumpBudget, SurfaceBuildContext,
};

const FONT: &[u8] =
    include_bytes!("../../../crates/runenui_text/tests/fixtures/Cantarell-Regular.ttf");

fn doc(revision: u64) -> TextDocumentSnapshot {
    TextDocumentSnapshot::new(TextDocumentId::new(331), TextDocumentRevision::new(revision))
}

enum Action {
    Edit(Box<EditIntent>),
    Popup(ComboBoxRequest),
    Choose(u8),
}

struct State {
    source: String,
    revision: u64,
    open: bool,
    chosen: u8,
    popup_log: Vec<ComboBoxRequest>,
}

struct App;
impl UiApp for App {
    type State = State;
    type Action = Action;
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> impl View<Self::Action> {
        let snapshot = doc(state.revision);
        let caret = TextPosition::new(
            snapshot,
            &state.source,
            state.source.len(),
            TextAffinity::Upstream,
        ).unwrap_or_else(|_| unreachable!("valid authored UTF-8 caret"));
        let editor = text_field(
            snapshot,
            state.source.clone(),
            TextSelection::collapsed(caret),
            SemanticEditableMode::SingleLine,
            |intent| Action::Edit(Box::new(intent)),
        ).unwrap_or_else(|_| unreachable!("one checked M10 source"))
            .id("editable.owner");
        editable_combo_box(
            editor,
            "Search suggestions",
            ElementId::new("editable.list").unwrap_or_else(|_| unreachable!()),
            [
                option_item("Alpha", state.chosen == 1)
                    .id("editable.alpha")
                    .on_activate(|| Action::Choose(1)),
                option_item("Beta", state.chosen == 2)
                    .id("editable.beta")
                    .on_activate(|| Action::Choose(2)),
            ],
            Action::Popup,
        ).unwrap_or_else(|_| unreachable!("public single-line editor"))
            .open(state.open)
            .autocomplete(SemanticAutocomplete::List)
    }

    fn update(
        state: &mut Self::State,
        action: Self::Action,
    ) -> impl IntoUpdateOutput<Self::Action, Self::HostProtocol> {
        match action {
            Action::Edit(intent) => {
                let replacement = intent.replacement();
                state.source.replace_range(
                    replacement.start()..replacement.end(),
                    intent.replacement_text(),
                );
                state.revision += 1;
                UpdateOutput::edit(EditResolution::accepted(
                    intent.request().clone(), doc(state.revision),
                ))
            }
            Action::Popup(request) => {
                state.open = request == ComboBoxRequest::Open;
                state.popup_log.push(request);
                UpdateOutput::effects(Effects::none())
            }
            Action::Choose(value) => {
                state.chosen = value;
                state.open = false;
                UpdateOutput::effects(Effects::none())
            }
        }
    }
}

fn runtime() -> AppRuntime<App> {
    let mut runtime = AppRuntime::<App>::mount(State {
        source: "q".into(), revision: 1, open: true,
        chosen: 0, popup_log: Vec::new(),
    });
    assert!(runtime.register_text_font_bytes(FONT.to_vec()).is_ok());
    assert!(runtime.set_text_generic_family_mapping(
        GenericFontFamily::SansSerif,
        &[FontFamilyName::new("Cantarell").unwrap_or_else(|_| unreachable!())],
    ).is_ok());
    settle(&mut runtime);
    runtime
}
fn settle(runtime: &mut AppRuntime<App>) {
    assert!(runtime.pump(PumpBudget::new(256, 256, 256, 256)).is_quiescent());
}
fn id(runtime: &AppRuntime<App>, id: &str) -> MountedNodeId {
    let authored = ElementId::new(id).unwrap_or_else(|_| unreachable!());
    runtime.index().nodes().iter()
        .find(|node| node.authored_id() == Some(&authored))
        .unwrap_or_else(|| unreachable!("mounted authored owner"))
        .id().clone()
}
fn cmd(runtime: &mut AppRuntime<App>, target: MountedNodeId, command: SemanticCommand) {
    runtime.submit_command(target, command, CommandOrigin::programmatic())
        .unwrap_or_else(|_| unreachable!("current semantic command admitted"));
    settle(runtime);
}
fn publish(runtime: &mut AppRuntime<App>) -> runenui_runtime::SurfacePublication {
    runtime.publish_surface(&SurfaceBuildContext::tight(
        &StyleEnvironment::default(),
        LogicalSize::try_new(600.0, 320.0).unwrap_or_else(|_| unreachable!()),
    )).unwrap_or_else(|error| unreachable!("editable ComboBox publish: {error:?}"))
}
fn active(runtime: &mut AppRuntime<App>) -> Option<String> {
    let publication = publish(runtime);
    let snapshot = publication.semantic_publication().snapshot();
    let owner = snapshot.nodes().iter().find(|node| node.role() == SemanticRole::ComboBox)?;
    let target = owner.relationships().iter()
        .find(|relation| relation.kind() == SemanticRelationshipKind::ActiveDescendant)?
        .target();
    snapshot.nodes().iter().find(|node| node.id() == target)
        .and_then(|node| node.name()).map(str::to_owned)
}

#[test]
fn editable_combo_is_one_focused_m10_editor_and_one_active_descendant_owner() {
    let mut runtime = runtime();
    let owner = id(&runtime, "editable.owner");
    cmd(&mut runtime, owner.clone(), SemanticCommand::RequestFocus);
    let publication = publish(&mut runtime);
    let snapshot = publication.semantic_publication().snapshot();
    let combo = snapshot.nodes().iter().find(|node| node.role() == SemanticRole::ComboBox)
        .unwrap_or_else(|| unreachable!("ComboBox primary"));
    assert!(combo.editable().is_some(), "the ComboBox node itself must own M10 editing");
    assert_eq!(combo.editable_mode(), Some(SemanticEditableMode::SingleLine));
    assert!(!snapshot.nodes().iter().any(|node| node.role() == SemanticRole::EditableText));
    let controls = combo.relationships().iter()
        .find(|rel| rel.kind() == SemanticRelationshipKind::Controls)
        .unwrap_or_else(|| unreachable!("checked ListBox relation"));
    assert_eq!(snapshot.nodes().iter()
        .find(|node| node.id() == controls.target())
        .map(runenui_runtime::SemanticNode::role), Some(SemanticRole::ListBox));
    assert_eq!(active(&mut runtime).as_deref(), Some("Alpha"));
    cmd(&mut runtime, owner.clone(), SemanticCommand::FocusDown);
    assert_eq!(active(&mut runtime).as_deref(), Some("Beta"));
    assert_eq!(runtime.focus().focused_node(), Some(&owner));
    assert_eq!(runtime.state().chosen, 0);
    cmd(&mut runtime, owner.clone(), SemanticCommand::Activate);
    assert_eq!(runtime.state().chosen, 2);
    assert!(!runtime.state().open);
    assert_eq!(runtime.focus().focused_node(), Some(&owner));
}

#[test]
fn committed_input_edits_application_text_without_committing_popup_candidate() {
    let mut runtime = runtime();
    let owner = id(&runtime, "editable.owner");
    cmd(&mut runtime, owner.clone(), SemanticCommand::RequestFocus);
    runtime.submit_text(CommittedTextEvent::new("x", None)
        .unwrap_or_else(|_| unreachable!("nonempty committed source")))
        .unwrap_or_else(|_| unreachable!("focused M10 accepts committed text"));
    settle(&mut runtime);
    assert_eq!(runtime.state().source, "qx");
    assert_eq!(runtime.state().revision, 2);
    assert_eq!(runtime.state().chosen, 0);
    assert!(runtime.state().open);
    assert_eq!(runtime.focus().focused_node(), Some(&owner));
    let snapshot = publish(&mut runtime);
    assert!(snapshot.semantic_publication().snapshot().nodes().iter()
        .any(|node| node.role() == SemanticRole::ComboBox && node.editable().is_some()));
}

fn key(logical: LogicalKey, physical: PhysicalKey, composition: KeyboardCompositionState) -> KeyboardEvent {
    KeyboardEvent::new(KeyboardPhase::Down, physical, logical, KeyModifiers::NONE,
        false, KeyLocation::Standard, composition, None)
}
#[test]
fn composition_and_editor_caret_navigation_do_not_commit_or_move_popup_proxy() {
    let mut runtime = runtime();
    let owner = id(&runtime, "editable.owner");
    cmd(&mut runtime, owner.clone(), SemanticCommand::RequestFocus);
    runtime.submit_keyboard(key(
        LogicalKey::ArrowDown, PhysicalKey::ArrowDown,
        KeyboardCompositionState::Active,
    )).unwrap_or_else(|_| unreachable!());
    settle(&mut runtime);
    runtime.submit_keyboard(key(
        LogicalKey::Enter, PhysicalKey::Enter, KeyboardCompositionState::Active,
    )).unwrap_or_else(|_| unreachable!());
    settle(&mut runtime);
    assert_eq!(active(&mut runtime).as_deref(), Some("Alpha"));
    assert!(runtime.state().open);
    assert_eq!(runtime.state().chosen, 0);
    runtime.submit_keyboard(key(
        LogicalKey::ArrowLeft, PhysicalKey::ArrowLeft,
        KeyboardCompositionState::Inactive,
    )).unwrap_or_else(|_| unreachable!());
    settle(&mut runtime);
    assert_eq!(active(&mut runtime).as_deref(), Some("Alpha"));
    assert_eq!(runtime.focus().focused_node(), Some(&owner));
}

#[test]
fn editable_constructor_rejects_multiline_and_secret_m10_sources() {
    let snapshot = doc(1);
    let caret = TextPosition::new(
        snapshot, "", 0, TextAffinity::Upstream,
    ).unwrap_or_else(|_| unreachable!("empty source boundary"));
    let make = |mode| text_field(
        snapshot, "", TextSelection::collapsed(caret), mode, |_| (),
    ).unwrap_or_else(|_| unreachable!("valid source/mode"));

    assert!(editable_combo_box(
        make(SemanticEditableMode::Multiline),
        "No multiline",
        ElementId::new("editable.invalid.multiline").unwrap_or_else(|_| unreachable!()),
        Vec::<runenui_core::OptionItem<()>>::new(),
        |_| (),
    ).is_err());
    assert!(editable_combo_box(
        make(SemanticEditableMode::SingleLine)
            .password().unwrap_or_else(|_| unreachable!("single-line secret")),
        "No secret",
        ElementId::new("editable.invalid.secret").unwrap_or_else(|_| unreachable!()),
        Vec::<runenui_core::OptionItem<()>>::new(),
        |_| (),
    ).is_err());
}
