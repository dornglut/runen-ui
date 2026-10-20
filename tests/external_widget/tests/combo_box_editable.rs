#![allow(refining_impl_trait)]
//! Editable ComboBox must be one M10 TextField / M5 ComboBox semantic owner.
use runenui_core::{
    ComboBoxRequest, CommandOrigin, CommittedTextEvent, EditIntent, EditResolution, Effects,
    ElementId, IntoUpdateOutput, KeyLocation, KeyModifiers, KeyboardCompositionState,
    KeyboardEvent, KeyboardPhase, LogicalKey, NoHostProtocol, PhysicalKey, SemanticAutocomplete,
    SemanticCommand, SemanticEditableMode, SemanticRelationshipKind, SemanticRole,
    StyleEnvironment, TextAffinity, TextDocumentId, TextDocumentRevision, TextDocumentSnapshot,
    TextPosition, TextSelection, UiApp, UpdateOutput, View, button, editable_combo_box,
    option_item, row, text_field,
};
use runenui_runtime::{
    AppRuntime, FontFamilyName, GenericFontFamily, LogicalSize, MountedNodeId, PumpBudget,
    SurfaceBuildContext,
};

const FONT: &[u8] =
    include_bytes!("../../../crates/runenui_text/tests/fixtures/Cantarell-Regular.ttf");

fn doc(revision: u64) -> TextDocumentSnapshot {
    TextDocumentSnapshot::new(
        TextDocumentId::new(331),
        TextDocumentRevision::new(revision),
    )
}

enum Action {
    Edit(Box<EditIntent>),
    Popup(ComboBoxRequest),
    Choose(u8),
    ReplaceOption,
    Submit,
    Disable,
    Enable,
}

struct State {
    source: String,
    revision: u64,
    open: bool,
    chosen: u8,
    popup_log: Vec<ComboBoxRequest>,
    replace_option: bool,
    submitted: usize,
    disabled: bool,
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
        )
        .unwrap_or_else(|_| unreachable!("valid authored UTF-8 caret"));
        let editor = text_field(
            snapshot,
            state.source.clone(),
            TextSelection::collapsed(caret),
            SemanticEditableMode::SingleLine,
            |intent| Action::Edit(Box::new(intent)),
        )
        .unwrap_or_else(|_| unreachable!("one checked M10 source"))
        .id("editable.owner")
        .on_submit(|| Action::Submit)
        .disabled(state.disabled);
        let combo = editable_combo_box(
            editor,
            "Search suggestions",
            ElementId::new("editable.list").unwrap_or_else(|_| unreachable!()),
            [
                option_item("Alpha", state.chosen == 1)
                    .id("editable.alpha")
                    .on_activate(|| Action::Choose(1)),
                option_item("Beta", state.chosen == 2)
                    .id(if state.replace_option {
                        "editable.replacement"
                    } else {
                        "editable.beta"
                    })
                    // Public element ID is not the reconciliation lifetime key.
                    .key(if state.replace_option {
                        "editable.replacement"
                    } else {
                        "editable.beta"
                    })
                    .on_activate(|| Action::Choose(2)),
            ],
            Action::Popup,
        )
        .unwrap_or_else(|_| unreachable!("public single-line editor"))
        .open(state.open)
        .autocomplete(SemanticAutocomplete::List);
        row(vec![
            combo.into_element(),
            button("Next").id("editable.next").into_element(),
        ])
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
                    intent.request().clone(),
                    doc(state.revision),
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
            Action::ReplaceOption => {
                state.replace_option = true;
                UpdateOutput::effects(Effects::none())
            }
            Action::Submit => {
                state.submitted += 1;
                UpdateOutput::effects(Effects::none())
            }
            Action::Disable => {
                state.disabled = true;
                UpdateOutput::effects(Effects::none())
            }
            Action::Enable => {
                state.disabled = false;
                UpdateOutput::effects(Effects::none())
            }
        }
    }
}

fn runtime() -> AppRuntime<App> {
    let mut runtime = AppRuntime::<App>::mount(State {
        source: "q".into(),
        revision: 1,
        open: true,
        chosen: 0,
        popup_log: Vec::new(),
        replace_option: false,
        submitted: 0,
        disabled: false,
    });
    assert!(runtime.register_text_font_bytes(FONT.to_vec()).is_ok());
    assert!(
        runtime
            .set_text_generic_family_mapping(
                GenericFontFamily::SansSerif,
                &[FontFamilyName::new("Cantarell").unwrap_or_else(|_| unreachable!())],
            )
            .is_ok()
    );
    settle(&mut runtime);
    runtime
}
fn settle(runtime: &mut AppRuntime<App>) {
    assert!(
        runtime
            .pump(PumpBudget::new(256, 256, 256, 256))
            .unwrap_or_else(|error| unreachable!("ComboBox pump: {error:?}"))
            .report()
            .is_quiescent()
    );
}
fn id(runtime: &mut AppRuntime<App>, id: &str) -> MountedNodeId {
    let authored = ElementId::new(id).unwrap_or_else(|_| unreachable!());
    runtime
        .index()
        .nodes()
        .iter()
        .find(|node| node.authored_id() == Some(&authored))
        .unwrap_or_else(|| unreachable!("mounted authored owner"))
        .id()
        .clone()
}
fn cmd(runtime: &mut AppRuntime<App>, target: MountedNodeId, command: SemanticCommand) {
    runtime
        .submit_command(target, command, CommandOrigin::programmatic())
        .unwrap_or_else(|_| unreachable!("current semantic command admitted"));
    settle(runtime);
}
fn publish(runtime: &mut AppRuntime<App>) -> runenui_runtime::SurfacePublication {
    runtime
        .publish_surface(&SurfaceBuildContext::tight(
            &StyleEnvironment::default(),
            LogicalSize::try_new(600.0, 320.0).unwrap_or_else(|_| unreachable!()),
        ))
        .unwrap_or_else(|error| unreachable!("editable ComboBox publish: {error:?}"))
}
fn active(runtime: &mut AppRuntime<App>) -> Option<String> {
    let publication = publish(runtime);
    let snapshot = publication.semantic_publication().snapshot();
    let owner = snapshot
        .nodes()
        .iter()
        .find(|node| node.role() == SemanticRole::ComboBox)?;
    let target = owner
        .relationships()
        .iter()
        .find(|relation| relation.kind() == SemanticRelationshipKind::ActiveDescendant)?
        .target();
    snapshot
        .nodes()
        .iter()
        .find(|node| node.id() == target)
        .and_then(|node| node.name())
        .map(str::to_owned)
}

#[test]
fn editable_combo_is_one_focused_m10_editor_and_one_active_descendant_owner() {
    let mut runtime = runtime();
    let owner = id(&mut runtime, "editable.owner");
    cmd(&mut runtime, owner.clone(), SemanticCommand::RequestFocus);
    let publication = publish(&mut runtime);
    let snapshot = publication.semantic_publication().snapshot();
    let combo = snapshot
        .nodes()
        .iter()
        .find(|node| node.role() == SemanticRole::ComboBox)
        .unwrap_or_else(|| unreachable!("ComboBox primary"));
    assert!(
        combo.editable().is_some(),
        "the ComboBox node itself must own M10 editing"
    );
    assert_eq!(
        combo.editable_mode(),
        Some(SemanticEditableMode::SingleLine)
    );
    assert!(
        !snapshot
            .nodes()
            .iter()
            .any(|node| node.role() == SemanticRole::EditableText)
    );
    let controls = combo
        .relationships()
        .iter()
        .find(|rel| rel.kind() == SemanticRelationshipKind::Controls)
        .unwrap_or_else(|| unreachable!("checked ListBox relation"));
    assert_eq!(
        snapshot
            .nodes()
            .iter()
            .find(|node| node.id() == controls.target())
            .map(runenui_runtime::SemanticNode::role),
        Some(SemanticRole::ListBox)
    );
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
    let owner = id(&mut runtime, "editable.owner");
    cmd(&mut runtime, owner.clone(), SemanticCommand::RequestFocus);
    runtime
        .submit_text(
            CommittedTextEvent::new("x", None)
                .unwrap_or_else(|_| unreachable!("nonempty committed source")),
        )
        .unwrap_or_else(|_| unreachable!("focused M10 accepts committed text"));
    settle(&mut runtime);
    assert_eq!(runtime.state().source, "qx");
    assert_eq!(runtime.state().revision, 2);
    assert_eq!(runtime.state().chosen, 0);
    assert!(runtime.state().open);
    assert_eq!(runtime.focus().focused_node(), Some(&owner));
    let snapshot = publish(&mut runtime);
    assert!(
        snapshot
            .semantic_publication()
            .snapshot()
            .nodes()
            .iter()
            .any(|node| node.role() == SemanticRole::ComboBox && node.editable().is_some())
    );
}

fn key(
    logical: LogicalKey,
    physical: PhysicalKey,
    composition: KeyboardCompositionState,
) -> KeyboardEvent {
    KeyboardEvent::new(
        KeyboardPhase::Down,
        physical,
        logical,
        KeyModifiers::NONE,
        false,
        KeyLocation::Standard,
        composition,
        None,
    )
}
#[test]
fn composition_and_editor_caret_navigation_do_not_commit_or_move_popup_proxy() {
    let mut runtime = runtime();
    let owner = id(&mut runtime, "editable.owner");
    cmd(&mut runtime, owner.clone(), SemanticCommand::RequestFocus);
    runtime
        .submit_keyboard(key(
            LogicalKey::ArrowDown,
            PhysicalKey::ArrowDown,
            KeyboardCompositionState::Active,
        ))
        .unwrap_or_else(|_| unreachable!());
    settle(&mut runtime);
    runtime
        .submit_keyboard(key(
            LogicalKey::Enter,
            PhysicalKey::Enter,
            KeyboardCompositionState::Active,
        ))
        .unwrap_or_else(|_| unreachable!());
    settle(&mut runtime);
    assert_eq!(active(&mut runtime).as_deref(), Some("Alpha"));
    assert!(runtime.state().open);
    assert_eq!(runtime.state().chosen, 0);
    runtime
        .submit_keyboard(key(
            LogicalKey::ArrowLeft,
            PhysicalKey::ArrowLeft,
            KeyboardCompositionState::Inactive,
        ))
        .unwrap_or_else(|_| unreachable!());
    settle(&mut runtime);
    assert_eq!(active(&mut runtime).as_deref(), Some("Alpha"));
    assert_eq!(runtime.focus().focused_node(), Some(&owner));
}

#[test]
fn editable_constructor_rejects_multiline_and_secret_m10_sources() {
    let snapshot = doc(1);
    let caret = TextPosition::new(snapshot, "", 0, TextAffinity::Upstream)
        .unwrap_or_else(|_| unreachable!("empty source boundary"));
    let make = |mode| {
        text_field(snapshot, "", TextSelection::collapsed(caret), mode, |_| ())
            .unwrap_or_else(|_| unreachable!("valid source/mode"))
    };

    assert!(
        editable_combo_box(
            make(SemanticEditableMode::Multiline),
            "No multiline",
            ElementId::new("editable.invalid.multiline").unwrap_or_else(|_| unreachable!()),
            Vec::<runenui_core::OptionItem<()>>::new(),
            |_| (),
        )
        .is_err()
    );
    assert!(
        editable_combo_box(
            make(SemanticEditableMode::SingleLine)
                .password()
                .unwrap_or_else(|_| unreachable!("single-line secret")),
            "No secret",
            ElementId::new("editable.invalid.secret").unwrap_or_else(|_| unreachable!()),
            Vec::<runenui_core::OptionItem<()>>::new(),
            |_| (),
        )
        .is_err()
    );
}

#[test]
fn escape_dismisses_without_mutating_committed_value_or_real_focus() {
    let mut runtime = runtime();
    let owner = id(&mut runtime, "editable.owner");
    cmd(&mut runtime, owner.clone(), SemanticCommand::RequestFocus);
    cmd(&mut runtime, owner.clone(), SemanticCommand::FocusDown);
    assert_eq!(active(&mut runtime).as_deref(), Some("Beta"));
    runtime
        .submit_keyboard(key(
            LogicalKey::Escape,
            PhysicalKey::Escape,
            KeyboardCompositionState::Inactive,
        ))
        .unwrap_or_else(|_| unreachable!("escape ingress"));
    settle(&mut runtime);
    assert!(!runtime.state().open);
    assert_eq!(runtime.state().chosen, 0);
    assert_eq!(runtime.state().source, "q");
    assert_eq!(runtime.state().popup_log, [ComboBoxRequest::Close]);
    assert_eq!(runtime.focus().focused_node(), Some(&owner));
    assert_eq!(active(&mut runtime), None);
}

#[test]
fn tab_uses_m4_real_focus_and_dismisses_without_accepting_the_active_option() {
    let mut runtime = runtime();
    let owner = id(&mut runtime, "editable.owner");
    let next = id(&mut runtime, "editable.next");
    cmd(&mut runtime, owner, SemanticCommand::RequestFocus);
    let owner = id(&mut runtime, "editable.owner");
    cmd(&mut runtime, owner, SemanticCommand::FocusDown);
    assert_eq!(active(&mut runtime).as_deref(), Some("Beta"));
    runtime
        .submit_keyboard(key(
            LogicalKey::Tab,
            PhysicalKey::Tab,
            KeyboardCompositionState::Inactive,
        ))
        .unwrap_or_else(|_| unreachable!("Tab ingress"));
    settle(&mut runtime);
    let focus_observed = runtime.focus().focused_node().cloned();
    let focus_authored = runtime
        .index()
        .nodes()
        .iter()
        .find(|node| Some(node.id()) == focus_observed.as_ref())
        .and_then(|node| node.authored_id())
        .map(|authored| authored.as_str().to_owned());
    let next_after = id(&mut runtime, "editable.next");
    eprintln!(
        "UI486-TAB after settling: authored={focus_authored:?} target_is_original_next={} target_is_current_next={} focus_reason={:?} app_open={} chosen={}",
        focus_observed.as_ref() == Some(&next),
        focus_observed.as_ref() == Some(&next_after),
        runtime.focus().reason(),
        runtime.state().open,
        runtime.state().chosen
    );
    assert_eq!(runtime.focus().focused_node(), Some(&next));
    assert!(!runtime.state().open);
    assert_eq!(runtime.state().chosen, 0);
}

#[test]
fn replacing_exact_active_option_retires_proxy_without_sibling_retargeting() {
    let mut runtime = runtime();
    let owner = id(&mut runtime, "editable.owner");
    cmd(&mut runtime, owner.clone(), SemanticCommand::RequestFocus);
    cmd(&mut runtime, owner.clone(), SemanticCommand::FocusDown);
    assert_eq!(active(&mut runtime).as_deref(), Some("Beta"));
    let previous = id(&mut runtime, "editable.beta");
    runtime
        .submit_action(Action::ReplaceOption)
        .unwrap_or_else(|_| unreachable!("application replaces authored Option"));
    settle(&mut runtime);
    assert!(
        runtime
            .index()
            .nodes()
            .iter()
            .all(|node| node.id() != &previous)
    );
    assert_eq!(
        active(&mut runtime),
        None,
        "new Option at the same list position cannot impersonate removed identity"
    );
    cmd(&mut runtime, owner.clone(), SemanticCommand::Activate);
    assert_eq!(runtime.state().chosen, 0);
    assert_eq!(runtime.focus().focused_node(), Some(&owner));
}

#[test]
fn open_editable_enter_accepts_only_option_action_and_does_not_submit_textfield() {
    let mut runtime = runtime();
    let owner = id(&mut runtime, "editable.owner");
    cmd(&mut runtime, owner.clone(), SemanticCommand::RequestFocus);
    cmd(&mut runtime, owner.clone(), SemanticCommand::FocusDown);
    assert_eq!(active(&mut runtime).as_deref(), Some("Beta"));
    runtime
        .submit_keyboard(key(
            LogicalKey::Enter,
            PhysicalKey::Enter,
            KeyboardCompositionState::Inactive,
        ))
        .unwrap_or_else(|_| unreachable!("open ComboBox Enter is admitted"));
    settle(&mut runtime);
    assert_eq!(runtime.state().chosen, 2);
    assert!(!runtime.state().open);
    assert_eq!(
        runtime.state().submitted,
        0,
        "the ComboBox Option acceptance must suppress the M10 TextField submit callback"
    );
    assert_eq!(runtime.state().source, "q");
    assert_eq!(runtime.focus().focused_node(), Some(&owner));
}

#[test]
fn closed_editable_enter_reuses_existing_textfield_submit_once_without_option_commit() {
    let mut runtime = runtime();
    let owner = id(&mut runtime, "editable.owner");
    cmd(&mut runtime, owner.clone(), SemanticCommand::RequestFocus);
    runtime
        .submit_action(Action::Choose(1))
        .unwrap_or_else(|_| unreachable!("app closes popup with one chosen Option"));
    settle(&mut runtime);
    assert!(!runtime.state().open);
    runtime
        .submit_keyboard(key(
            LogicalKey::Enter,
            PhysicalKey::Enter,
            KeyboardCompositionState::Inactive,
        ))
        .unwrap_or_else(|_| unreachable!("closed TextField Enter admitted"));
    settle(&mut runtime);
    assert_eq!(
        runtime.state().submitted,
        1,
        "closed ComboBox leaves ordinary M10 TextField submit semantics intact"
    );
    assert_eq!(runtime.state().chosen, 1);
    assert_eq!(runtime.state().source, "q");
    assert_eq!(runtime.focus().focused_node(), Some(&owner));
}

#[test]
fn disabling_open_editable_combo_withdraws_popup_but_preserves_application_text_and_open_intent() {
    let mut runtime = runtime();
    let owner = id(&mut runtime, "editable.owner");
    let old_option = id(&mut runtime, "editable.alpha");
    let old_popup = id(&mut runtime, "editable.list");
    cmd(&mut runtime, owner.clone(), SemanticCommand::RequestFocus);
    runtime
        .submit_action(Action::Disable)
        .unwrap_or_else(|_| unreachable!("application may disable its editor"));
    settle(&mut runtime);

    assert!(
        runtime.state().open,
        "framework must not mutate app-owned popup intent"
    );
    assert_eq!(runtime.state().source, "q");
    assert_eq!(runtime.state().revision, 1);
    assert_eq!(runtime.state().chosen, 0);
    assert!(runtime.state().popup_log.is_empty());
    assert!(
        runtime
            .index()
            .nodes()
            .iter()
            .all(|node| node.id() != &old_option && node.id() != &old_popup),
        "disabled editor cannot retain an undismissable modal popup"
    );

    let publication = publish(&mut runtime);
    let semantics = publication.semantic_publication().snapshot();
    let combo = semantics
        .nodes()
        .iter()
        .find(|node| node.role() == SemanticRole::ComboBox)
        .unwrap_or_else(|| unreachable!("one disabled M10 ComboBox semantic owner"));
    assert!(combo.state().disabled());
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
    assert!(
        !runtime
            .index()
            .node(&owner)
            .is_some_and(|node| node.is_focusable())
    );
    assert_eq!(runtime.state().submitted, 0);

    runtime
        .submit_action(Action::Enable)
        .unwrap_or_else(|_| unreachable!("application may re-enable its editor"));
    settle(&mut runtime);
    assert_ne!(id(&mut runtime, "editable.alpha"), old_option);
    assert_ne!(id(&mut runtime, "editable.list"), old_popup);
    let publication = publish(&mut runtime);
    let semantics = publication.semantic_publication().snapshot();
    let combo = semantics
        .nodes()
        .iter()
        .find(|node| node.role() == SemanticRole::ComboBox)
        .unwrap_or_else(|| unreachable!("one reenabled M10 ComboBox semantic owner"));
    assert!(!combo.state().disabled());
    assert_eq!(combo.state().expanded(), Some(true));
    assert!(
        combo
            .relationships()
            .iter()
            .any(|rel| rel.kind() == SemanticRelationshipKind::Controls)
    );
    assert_eq!(runtime.state().source, "q");
    assert_eq!(runtime.state().revision, 1);
    assert_eq!(runtime.state().chosen, 0);
    assert!(runtime.state().popup_log.is_empty());
    assert_eq!(runtime.state().submitted, 0);
}

#[test]
fn editable_combo_native_enter_escape_claims_are_event_local_and_ime_safe() {
    use runenui_runtime::{
        InputArbitrationRecord, UiInputClaimReason, UiInputConflict, UiInputFinality,
    };

    for (logical, physical, composition, claim, chosen, open) in [
        (
            LogicalKey::Enter,
            PhysicalKey::Enter,
            KeyboardCompositionState::Inactive,
            true,
            1,
            false,
        ),
        (
            LogicalKey::Escape,
            PhysicalKey::Escape,
            KeyboardCompositionState::Inactive,
            true,
            0,
            false,
        ),
        (
            LogicalKey::Enter,
            PhysicalKey::Enter,
            KeyboardCompositionState::Active,
            false,
            0,
            true,
        ),
    ] {
        let mut runtime = runtime();
        let owner = id(&mut runtime, "editable.owner");
        cmd(&mut runtime, owner.clone(), SemanticCommand::RequestFocus);
        let before = runtime
            .input_ownership()
            .unwrap_or_else(|_| unreachable!("editable text owner snapshot"));
        let receipt = runtime
            .submit_keyboard(key(logical, physical, composition))
            .unwrap_or_else(|_| unreachable!("native editable key admitted"));
        let batch = runtime
            .pump(PumpBudget::new(1, 0, 0, 0))
            .unwrap_or_else(|error| unreachable!("editable native settlement: {error:?}"));
        let [InputArbitrationRecord::InputSettled(settled)] = batch.ordered_records() else {
            unreachable!("one reached keyboard input")
        };
        assert_eq!(settled.sequence(), receipt.sequence());
        assert_eq!(settled.scope(), before.scope());
        let UiInputFinality::Committed(facts) = settled.finality() else {
            unreachable!("nonterminal editable key")
        };
        assert_eq!(facts.conflict(), UiInputConflict::ExclusiveUi);
        assert!(facts.reasons().contains(&UiInputClaimReason::TextOwner));
        assert_eq!(
            facts
                .reasons()
                .contains(&UiInputClaimReason::ExplicitWidgetClaim),
            claim,
            "IME must not silently become an explicit ComboBox acceptance"
        );
        assert_eq!(batch.processed_through(), Some(receipt.sequence()));
        assert_eq!(
            runtime.state().chosen,
            0,
            "native input is not an application update"
        );
        assert_eq!(runtime.focus().focused_node(), Some(&owner));
        settle(&mut runtime);
        assert_eq!(runtime.state().chosen, chosen);
        assert_eq!(runtime.state().open, open);
        assert_eq!(runtime.state().source, "q");
        assert_eq!(runtime.state().submitted, 0);
    }
}

#[test]
fn diagnostic_tab_native_finality_and_focus_default_are_distinct() {
    use runenui_runtime::{
        InputArbitrationRecord, UiInputFinality,
    };

    let mut runtime = runtime();
    let owner = id(&mut runtime, "editable.owner");
    cmd(&mut runtime, owner.clone(), SemanticCommand::RequestFocus);
    cmd(&mut runtime, owner.clone(), SemanticCommand::FocusDown);
    assert_eq!(active(&mut runtime).as_deref(), Some("Beta"));
    let receipt = runtime
        .submit_keyboard(key(
            LogicalKey::Tab,
            PhysicalKey::Tab,
            KeyboardCompositionState::Inactive,
        ))
        .unwrap_or_else(|_| unreachable!("Tab input accepted"));
    let batch = runtime
        .pump(PumpBudget::new(1, 0, 0, 0))
        .unwrap_or_else(|error| unreachable!("one native Tab envelope: {error:?}"));
    assert_eq!(batch.processed_through(), Some(receipt.sequence()));
    let [InputArbitrationRecord::InputSettled(settled)] = batch.ordered_records() else {
        unreachable!("Tab has one exactly settled native receipt")
    };
    let UiInputFinality::Committed(facts) = settled.finality() else {
        unreachable!("native Tab was unexpectedly rejected")
    };
    eprintln!(
        "UI486-TAB NATIVE: conflict={:?} reasons={:?} default={:?} focus_after_input={:?}",
        facts.conflict(),
        facts.reasons(),
        facts.default_disposition(),
        runtime.focus().reason()
    );
    settle(&mut runtime);
    let final_focus = runtime.focus().focused_node().cloned();
    let authored = runtime
        .index()
        .nodes()
        .iter()
        .find(|node| Some(node.id()) == final_focus.as_ref())
        .and_then(|node| node.authored_id())
        .map(|id| id.as_str().to_owned());
    eprintln!(
        "UI486-TAB SETTLED: focus={authored:?} reason={:?} chosen={} open={}",
        runtime.focus().reason(),
        runtime.state().chosen,
        runtime.state().open
    );
}
