#![allow(refining_impl_trait)]
#![allow(
    clippy::expect_used,
    clippy::ignored_unit_patterns,
    clippy::large_enum_variant,
    clippy::panic,
    clippy::struct_excessive_bools,
    clippy::too_many_lines
)]

use runenui_core::{
    ClipboardClassification, ClipboardText, ClipboardWritePurpose, CommandOrigin,
    CommittedTextEvent, EditChangeMap, EditGroupHint, EditIntent, EditKind, EditRequestId,
    EditResolution, EditableContribution, Effects, Element, EventContext, FrameworkServiceFailure,
    FrameworkServiceRequest, FrameworkServiceResponse, FrameworkServiceResponseKind,
    HitContribution, HitContributionContext, KeyLocation, KeyModifiers, KeyboardCompositionState,
    KeyboardEvent, KeyboardPhase, LogicalKey, LogicalPoint, LogicalRect, NoHostProtocol,
    PhysicalKey, PointerButton, PointerButtons, PointerDeviceKind, PointerEvent, PointerId,
    PointerPhase, SemanticAction, SemanticCommand, SemanticContribution,
    SemanticContributionContext, SemanticEditable, SemanticNodeContribution, SemanticRole,
    SemanticState, StyleEnvironment, SurfaceInputContext, TextAffinity, TextDocumentId,
    TextDocumentRevision, TextDocumentSnapshot, TextPosition, TextRange, TextSelection,
    TextSensitivity, UiApp, UiEvent, UpdateOutput, Widget, WidgetActivation, WidgetEventOutput,
    WidgetMeasure, WidgetMeasureInput, WidgetTextInput,
};
use runenui_runtime::{
    AppRuntime, FontFamilyName, GenericFontFamily, LogicalSize, PumpBudget, RuntimeConfig,
    RuntimeLimits, RuntimeStatus, RuntimeTerminalReason, SubmitSemanticActionErrorKind,
    SubmitTextErrorKind, SurfaceBuildContext, TraceConfig, TraceFrameworkServiceKind,
    TraceFrameworkServiceOutcome, TracePayloadCapture, TraceRecordKind, TraceReplay,
};

const CANTARELL: &[u8] = include_bytes!("../../runenui_text/tests/fixtures/Cantarell-Regular.ttf");

#[derive(Debug)]
struct Editor;

#[derive(Clone)]
struct Document {
    text: String,
    revision: u64,
    selection: usize,
    reject_next: bool,
    rejected_chain: bool,
    transform_next: bool,
    ordinary_count: usize,
    sensitivity: TextSensitivity,
    history: Vec<String>,
    redo: Vec<String>,
    editor_visible: bool,
    prevent_default: bool,
    prevent_pointer_down: bool,
    saved_request: Option<EditRequestId>,
    read_only: bool,
    disabled: bool,
    observed_edits: Vec<ObservedEdit>,
    emit_equal_ordinary_after_edit: bool,
    equal_action_value_observed: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ObservedEdit {
    kind: EditKind,
    inverse_range: Option<core::ops::Range<usize>>,
    inverse_text: Option<String>,
    group: Option<EditGroupHint>,
    has_predecessor: bool,
}

enum Action {
    Edit(EditIntent),
    Ordinary,
    RemoveEditor,
}

impl PartialEq for Action {
    fn eq(&self, _other: &Self) -> bool {
        true
    }
}

impl Eq for Action {}

impl Widget<Action> for Editor {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn activation(&self, (): &Self::State) -> WidgetActivation {
        WidgetActivation::NONE
    }

    fn text_input(&self, (): &Self::State) -> WidgetTextInput {
        WidgetTextInput::new(true, true)
    }

    fn editable(&self, (): &Self::State) -> Option<EditableContribution<Action>> {
        None
    }
}

#[derive(Debug)]
struct BoundEditor {
    snapshot: TextDocumentSnapshot,
    text: String,
    selection: TextSelection,
    sensitivity: TextSensitivity,
    prevent_default: bool,
    prevent_pointer_down: bool,
    read_only: bool,
    disabled: bool,
}

impl Widget<Action> for BoundEditor {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn activation(&self, (): &Self::State) -> WidgetActivation {
        if self.disabled {
            WidgetActivation::disabled()
        } else {
            WidgetActivation::NONE
        }
    }

    fn text_input(&self, (): &Self::State) -> WidgetTextInput {
        WidgetTextInput::new(true, true)
    }

    fn event(
        &mut self,
        (): &mut Self::State,
        event: &UiEvent,
        context: &mut EventContext<'_, Action>,
    ) -> WidgetEventOutput {
        if self.prevent_default
            && (event.as_committed_text().is_some()
                || event
                    .as_semantic_command()
                    .is_some_and(|event| event.command() != SemanticCommand::RequestFocus))
            || event.as_pointer().is_some_and(|pointer| {
                self.prevent_pointer_down && pointer.phase() == PointerPhase::Down
            })
        {
            context.prevent_default();
        }
        WidgetEventOutput::none()
    }

    fn editable(&self, (): &Self::State) -> Option<EditableContribution<Action>> {
        EditableContribution::new(
            self.snapshot,
            self.text.clone(),
            self.selection,
            self.sensitivity,
            self.read_only,
            self.disabled,
            runenui_core::EditingSessionPolicy::PreserveExact,
            Action::Edit,
        )
        .ok()
    }

    fn measure(&self, (): &Self::State, _: WidgetMeasureInput) -> WidgetMeasure {
        WidgetMeasure::Text {
            content: self.text.clone(),
        }
    }

    fn hit_test(&self, (): &Self::State, context: HitContributionContext) -> HitContribution {
        let size = context.local_size();
        HitContribution::single_rect(
            LogicalRect::try_new(0.0, 0.0, size.width(), size.height())
                .unwrap_or_else(|_| unreachable!("validated local size yields valid hit geometry")),
        )
    }

    fn semantics(&self, (): &Self::State, _: SemanticContributionContext) -> SemanticContribution {
        let editable = SemanticEditable::new(
            self.snapshot,
            &self.text,
            self.selection,
            self.sensitivity,
            self.read_only,
        )
        .unwrap_or_else(|| unreachable!("application semantic selection is checked"));
        SemanticContribution::single(
            SemanticNodeContribution::primary(SemanticRole::EditableText)
                .with_state(
                    SemanticState::ENABLED
                        .with_read_only(self.read_only)
                        .with_disabled(self.disabled),
                )
                .with_editable(editable)
                .with_action(SemanticAction::MoveBackward)
                .with_action(SemanticAction::MoveForward)
                .with_action(SemanticAction::ExtendBackward)
                .with_action(SemanticAction::ExtendForward)
                .with_action(SemanticAction::SelectAll)
                .with_action(SemanticAction::DeleteBackward)
                .with_action(SemanticAction::DeleteForward)
                .with_action(SemanticAction::Undo)
                .with_action(SemanticAction::Redo)
                .with_action(SemanticAction::Copy)
                .with_action(SemanticAction::Cut)
                .with_action(SemanticAction::Paste)
                .with_action(SemanticAction::SetSelection)
                .with_action(SemanticAction::ReplaceSelection),
        )
    }
}

struct App;

fn editor_root(state: &Document) -> Element<Action> {
    if !state.editor_visible {
        return Element::new(Editor).id("placeholder").key("placeholder");
    }
    let snapshot = TextDocumentSnapshot::new(
        TextDocumentId::new(1),
        TextDocumentRevision::new(state.revision),
    );
    let position = TextPosition::new(
        snapshot,
        &state.text,
        state.selection,
        if state.selection == state.text.len() && !state.text.is_empty() {
            TextAffinity::Upstream
        } else {
            TextAffinity::Downstream
        },
    )
    .unwrap_or_else(|_| unreachable!("application selection is checked"));
    Element::new(BoundEditor {
        snapshot,
        text: state.text.clone(),
        selection: TextSelection::collapsed(position),
        sensitivity: state.sensitivity,
        prevent_default: state.prevent_default,
        prevent_pointer_down: state.prevent_pointer_down,
        read_only: state.read_only,
        disabled: state.disabled,
    })
    .id("editor")
    .key("editor")
    .focusable(true)
}

impl UiApp for App {
    type State = Document;
    type Action = Action;
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> Element<Self::Action> {
        editor_root(state)
    }

    fn update(
        state: &mut Self::State,
        action: Self::Action,
    ) -> UpdateOutput<Self::Action, Self::HostProtocol> {
        match action {
            Action::Ordinary => {
                state.ordinary_count += 1;
                UpdateOutput::effects(Effects::none())
            }
            Action::RemoveEditor => {
                state.editor_visible = false;
                UpdateOutput::effects(Effects::none())
            }
            Action::Edit(intent) => {
                state.equal_action_value_observed =
                    Action::Ordinary == Action::Edit(intent.clone());
                state.observed_edits.push(ObservedEdit {
                    kind: intent.kind(),
                    inverse_range: intent.inverse().map(runenui_core::EditInverse::replacement),
                    inverse_text: intent
                        .inverse()
                        .map(|inverse| inverse.replacement_text().to_owned()),
                    group: intent.group(),
                    has_predecessor: intent.predecessor().is_some(),
                });
                let request = intent.request().clone();
                if !state.editor_visible {
                    return UpdateOutput::edit(EditResolution::rejected(
                        request,
                        TextDocumentSnapshot::new(
                            TextDocumentId::new(1),
                            TextDocumentRevision::new(state.revision),
                        ),
                    ));
                }
                if intent.kind() == EditKind::Undo {
                    let current = state.text.clone();
                    let Some(previous) = state.history.pop() else {
                        return UpdateOutput::edit(EditResolution::rejected(
                            request,
                            TextDocumentSnapshot::new(
                                TextDocumentId::new(1),
                                TextDocumentRevision::new(state.revision),
                            ),
                        ));
                    };
                    let replaced = TextRange::new(
                        TextDocumentSnapshot::new(
                            TextDocumentId::new(1),
                            TextDocumentRevision::new(state.revision),
                        ),
                        &current,
                        0,
                        current.len(),
                    )
                    .unwrap_or_else(|_| unreachable!("whole history range is checked"));
                    state.redo.push(current);
                    state.text = previous;
                    state.selection = state.text.len();
                    state.revision += 1;
                    return UpdateOutput::edit(EditResolution::transformed(
                        request,
                        TextDocumentSnapshot::new(
                            TextDocumentId::new(1),
                            TextDocumentRevision::new(state.revision),
                        ),
                        EditChangeMap::new(replaced, state.text.len()),
                    ));
                }
                if intent.kind() == EditKind::Redo {
                    let current = state.text.clone();
                    let Some(next) = state.redo.pop() else {
                        return UpdateOutput::edit(EditResolution::rejected(
                            request,
                            TextDocumentSnapshot::new(
                                TextDocumentId::new(1),
                                TextDocumentRevision::new(state.revision),
                            ),
                        ));
                    };
                    let replaced = TextRange::new(
                        TextDocumentSnapshot::new(
                            TextDocumentId::new(1),
                            TextDocumentRevision::new(state.revision),
                        ),
                        &current,
                        0,
                        current.len(),
                    )
                    .unwrap_or_else(|_| unreachable!("whole redo range is checked"));
                    state.history.push(current);
                    state.text = next;
                    state.selection = state.text.len();
                    state.revision += 1;
                    return UpdateOutput::edit(EditResolution::transformed(
                        request,
                        TextDocumentSnapshot::new(
                            TextDocumentId::new(1),
                            TextDocumentRevision::new(state.revision),
                        ),
                        EditChangeMap::new(replaced, state.text.len()),
                    ));
                }
                if state.reject_next || (state.rejected_chain && intent.predecessor().is_some()) {
                    state.reject_next = false;
                    state.rejected_chain = true;
                    return UpdateOutput::edit(EditResolution::rejected(
                        request,
                        TextDocumentSnapshot::new(
                            TextDocumentId::new(1),
                            TextDocumentRevision::new(state.revision),
                        ),
                    ));
                }
                let range = intent.replacement();
                if state.transform_next {
                    state.transform_next = false;
                    let mut proposed = state.text.clone();
                    proposed.replace_range(range.start()..range.end(), intent.replacement_text());
                    let transformed_replacement = intent.replacement_text().to_uppercase();
                    state.redo.clear();
                    state.history.push(state.text.clone());
                    state
                        .text
                        .replace_range(range.start()..range.end(), &transformed_replacement);
                    state.selection = range.start() + transformed_replacement.len();
                    state.revision += 1;
                    let replaced = TextRange::new(
                        range.snapshot(),
                        &proposed,
                        range.start(),
                        range.start() + intent.replacement_text().len(),
                    )
                    .unwrap_or_else(|_| unreachable!("transformed fixture range is checked"));
                    return UpdateOutput::edit(EditResolution::transformed(
                        request,
                        TextDocumentSnapshot::new(
                            TextDocumentId::new(1),
                            TextDocumentRevision::new(state.revision),
                        ),
                        EditChangeMap::new(replaced, transformed_replacement.len()),
                    ));
                }
                state.redo.clear();
                state.history.push(state.text.clone());
                state
                    .text
                    .replace_range(range.start()..range.end(), intent.replacement_text());
                state.selection = intent.proposed_selection().active();
                state.revision += 1;
                let resolution = EditResolution::accepted(
                    request,
                    TextDocumentSnapshot::new(
                        TextDocumentId::new(1),
                        TextDocumentRevision::new(state.revision),
                    ),
                );
                if state.emit_equal_ordinary_after_edit {
                    state.emit_equal_ordinary_after_edit = false;
                    UpdateOutput::edit_with_effects(Effects::action(Action::Ordinary), resolution)
                } else {
                    UpdateOutput::edit(resolution)
                }
            }
        }
    }
}

fn mounted() -> AppRuntime<App> {
    AppRuntime::mount(Document {
        text: "ab".into(),
        revision: 0,
        selection: 2,
        reject_next: false,
        rejected_chain: false,
        transform_next: false,
        ordinary_count: 0,
        sensitivity: TextSensitivity::Public,
        history: Vec::new(),
        redo: Vec::new(),
        editor_visible: true,
        prevent_default: false,
        prevent_pointer_down: false,
        saved_request: None,
        read_only: false,
        disabled: false,
        observed_edits: Vec::new(),
        emit_equal_ordinary_after_edit: false,
        equal_action_value_observed: false,
    })
}

fn focus_generic<TestApp>(runtime: &mut AppRuntime<TestApp>)
where
    TestApp: UiApp<State = Document, Action = Action, HostProtocol = NoHostProtocol>,
{
    let target = runtime.index().nodes()[0].id().clone();
    runtime
        .submit_command(
            target,
            SemanticCommand::RequestFocus,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("live editor accepts focus"));
    runtime.pump(PumpBudget::new(8, usize::MAX, usize::MAX, usize::MAX));
}

fn focus(runtime: &mut AppRuntime<App>) {
    focus_generic(runtime);
}

fn keyboard_down(runtime: &mut AppRuntime<App>, logical: LogicalKey, modifiers: KeyModifiers) {
    runtime
        .submit_keyboard(KeyboardEvent::new(
            KeyboardPhase::Down,
            PhysicalKey::Code(format!("{logical:?}")),
            logical,
            modifiers,
            false,
            KeyLocation::Standard,
            KeyboardCompositionState::Inactive,
            None,
        ))
        .unwrap_or_else(|error| panic!("keyboard command is submitted: {error:?}"));
    runtime.pump(PumpBudget::new(16, usize::MAX, usize::MAX, usize::MAX));
}

fn publish_editor(runtime: &mut AppRuntime<App>) -> SurfaceInputContext {
    let environment = StyleEnvironment::default();
    let context = SurfaceBuildContext::tight(
        &environment,
        LogicalSize::try_new(200.0, 40.0)
            .unwrap_or_else(|_| unreachable!("test surface is finite")),
    );
    runtime
        .publish_surface(&context)
        .unwrap_or_else(|error| panic!("editable surface publishes: {error:?}"))
        .input_context()
        .clone()
}

fn select_all(runtime: &mut AppRuntime<App>) {
    let owner = runtime.index().nodes()[0].id().clone();
    runtime
        .submit_command(
            owner,
            SemanticCommand::SelectAll,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|error| panic!("select-all command is queued: {error:?}"));
    runtime.pump(PumpBudget::new(16, usize::MAX, usize::MAX, usize::MAX));
}

fn pending_service(
    runtime: &AppRuntime<App>,
    predicate: impl Fn(&FrameworkServiceRequest) -> bool,
) -> runenui_runtime::FrameworkServiceRef<'_> {
    runtime
        .pending_framework_services()
        .into_iter()
        .find(|service| predicate(service.request()))
        .unwrap_or_else(|| unreachable!("matching framework service is pending"))
}

fn has_pending_clipboard_service(runtime: &AppRuntime<App>) -> bool {
    runtime.pending_framework_services().iter().any(|service| {
        matches!(
            service.request(),
            FrameworkServiceRequest::ClipboardReadText { .. }
                | FrameworkServiceRequest::ClipboardWriteText { .. }
        )
    })
}

fn complete_pending_state_services(runtime: &mut AppRuntime<App>) {
    let pending = runtime.pending_framework_services();
    let tokens = pending
        .iter()
        .filter_map(|service| match service.request() {
            FrameworkServiceRequest::InputMethod { .. }
            | FrameworkServiceRequest::Cursor { .. } => {
                Some((service.token(), service.request().response_kind()))
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    for (token, kind) in tokens {
        let response = match kind {
            FrameworkServiceResponseKind::InputMethod => {
                FrameworkServiceResponse::InputMethod(Ok(()))
            }
            FrameworkServiceResponseKind::Cursor => FrameworkServiceResponse::Cursor(Ok(())),
            _ => unreachable!("only state synchronization services are completed"),
        };
        runtime
            .complete_framework_service(&token, response)
            .unwrap_or_else(|error| panic!("fake host queues state-service result: {error:?}"));
    }
    runtime.pump(PumpBudget::new(32, usize::MAX, usize::MAX, usize::MAX));
}

fn install_controlled_font(runtime: &mut AppRuntime<App>) {
    assert!(
        runtime
            .register_text_font_bytes(CANTARELL.to_vec())
            .unwrap_or_else(|error| panic!("controlled font registers: {error:?}"))
            > 0
    );
    assert!(
        runtime
            .set_text_generic_family_mapping(
                GenericFontFamily::SansSerif,
                &[FontFamilyName::new("Cantarell")
                    .unwrap_or_else(|_| unreachable!("fixture family name is valid"))],
            )
            .unwrap_or_else(|error| panic!("controlled generic mapping installs: {error:?}"))
    );
}

#[test]
fn committed_text_queues_one_edit_action_and_acceptance_reconciles_authoritative_state() {
    let mut runtime = mounted();
    focus(&mut runtime);
    runtime
        .submit_text(
            CommittedTextEvent::new("é", None)
                .unwrap_or_else(|_| unreachable!("text is non-empty")),
        )
        .unwrap_or_else(|_| unreachable!("focused editor accepts text"));
    assert_eq!(runtime.state().text, "ab");
    runtime.pump(PumpBudget::new(8, usize::MAX, usize::MAX, usize::MAX));
    assert_eq!(runtime.state().text, "abé");
    assert_eq!(runtime.state().revision, 1);
    assert_eq!(runtime.state().history, ["ab"]);
    assert_eq!(runtime.status(), RuntimeStatus::Running);
}

#[test]
fn equal_opaque_edit_and_effect_action_values_keep_their_private_envelope_origins() {
    let mut runtime = AppRuntime::<App>::mount(Document {
        emit_equal_ordinary_after_edit: true,
        ..mounted().state().clone()
    });
    focus(&mut runtime);
    runtime
        .submit_text(
            CommittedTextEvent::new("x", None)
                .unwrap_or_else(|_| unreachable!("text is non-empty")),
        )
        .unwrap_or_else(|error| panic!("edit is queued: {error:?}"));
    runtime.pump(PumpBudget::new(16, usize::MAX, usize::MAX, usize::MAX));

    assert_eq!(runtime.state().text, "abx");
    assert!(runtime.state().equal_action_value_observed);
    assert_eq!(runtime.state().ordinary_count, 1);
    assert_eq!(runtime.status(), RuntimeStatus::Running);
}

#[test]
fn logical_character_keys_do_not_insert_and_prevented_committed_text_enqueues_no_edit() {
    let mut runtime = mounted();
    focus(&mut runtime);
    runtime
        .submit_keyboard(KeyboardEvent::new(
            KeyboardPhase::Down,
            PhysicalKey::Code("KeyX".to_owned()),
            LogicalKey::Character("x".to_owned()),
            KeyModifiers::NONE,
            false,
            KeyLocation::Standard,
            KeyboardCompositionState::Inactive,
            None,
        ))
        .unwrap_or_else(|error| panic!("logical character key is routed: {error:?}"));
    runtime.pump(PumpBudget::new(8, usize::MAX, usize::MAX, usize::MAX));
    assert_eq!(runtime.state().text, "ab");
    assert!(runtime.state().history.is_empty());
    let owner = runtime.index().nodes()[0].id().clone();
    runtime
        .submit_command(owner, SemanticCommand::Copy, CommandOrigin::programmatic())
        .unwrap_or_else(|error| panic!("clipboard vocabulary remains routable: {error:?}"));
    runtime.pump(PumpBudget::new(8, usize::MAX, usize::MAX, usize::MAX));
    assert!(runtime.trace().kinds().any(|kind| matches!(
        kind,
        TraceRecordKind::EditingDefaultUnavailable {
            command: SemanticCommand::Copy
        }
    )));
    assert_eq!(runtime.state().text, "ab");
    assert!(runtime.state().history.is_empty());

    let mut prevented = AppRuntime::<App>::mount(Document {
        prevent_default: true,
        ..mounted().state().clone()
    });
    focus(&mut prevented);
    prevented
        .submit_text(
            CommittedTextEvent::new("x", None)
                .unwrap_or_else(|_| unreachable!("text is non-empty")),
        )
        .unwrap_or_else(|error| panic!("cancelable text event is routed: {error:?}"));
    prevented.pump(PumpBudget::new(8, usize::MAX, usize::MAX, usize::MAX));
    assert_eq!(prevented.state().text, "ab");
    assert!(prevented.state().history.is_empty());
    assert_eq!(prevented.__editing_session_counts_for_test(), (1, 0));
    assert_eq!(prevented.status(), RuntimeStatus::Running);
}

#[test]
fn clipboard_copy_cut_and_paste_use_typed_fake_host_results_and_queue_front_revalidation() {
    let mut copy = mounted();
    install_controlled_font(&mut copy);
    focus(&mut copy);
    publish_editor(&mut copy);
    select_all(&mut copy);
    let owner = copy.index().nodes()[0].id().clone();
    copy.submit_command(owner, SemanticCommand::Copy, CommandOrigin::programmatic())
        .unwrap_or_else(|error| panic!("copy default is admitted: {error:?}"));
    copy.pump(PumpBudget::new(16, usize::MAX, usize::MAX, usize::MAX));
    let request = pending_service(&copy, |request| {
        matches!(
            request,
            FrameworkServiceRequest::ClipboardWriteText {
                purpose: ClipboardWritePurpose::Copy,
                ..
            }
        )
    });
    let token = request.token();
    assert!(matches!(
        request.request(),
        FrameworkServiceRequest::ClipboardWriteText {
            text,
            purpose: ClipboardWritePurpose::Copy
        } if text.as_ref() == "ab"
    ));
    assert!(!format!("{:?}", request.request()).contains("ab"));
    copy.complete_framework_service(&token, FrameworkServiceResponse::ClipboardWriteText(Ok(())))
        .unwrap_or_else(|error| panic!("copy result queues: {error:?}"));
    assert_eq!(copy.state().text, "ab");
    copy.pump(PumpBudget::new(16, usize::MAX, usize::MAX, usize::MAX));
    assert_eq!(copy.state().text, "ab");
    assert_eq!(copy.status(), RuntimeStatus::Running);

    let mut failed_cut = mounted();
    install_controlled_font(&mut failed_cut);
    focus(&mut failed_cut);
    publish_editor(&mut failed_cut);
    select_all(&mut failed_cut);
    let owner = failed_cut.index().nodes()[0].id().clone();
    failed_cut
        .submit_command(owner, SemanticCommand::Cut, CommandOrigin::programmatic())
        .unwrap_or_else(|error| panic!("cut default is admitted: {error:?}"));
    failed_cut.pump(PumpBudget::new(16, usize::MAX, usize::MAX, usize::MAX));
    let token = pending_service(&failed_cut, |request| {
        matches!(
            request,
            FrameworkServiceRequest::ClipboardWriteText {
                purpose: ClipboardWritePurpose::Cut,
                ..
            }
        )
    })
    .token();
    failed_cut
        .complete_framework_service(
            &token,
            FrameworkServiceResponse::ClipboardWriteText(Err(
                FrameworkServiceFailure::PermissionDenied,
            )),
        )
        .unwrap_or_else(|error| panic!("cut failure is a typed response: {error:?}"));
    failed_cut.pump(PumpBudget::new(16, usize::MAX, usize::MAX, usize::MAX));
    assert_eq!(failed_cut.state().text, "ab");
    assert!(failed_cut.state().observed_edits.is_empty());
    assert!(failed_cut.trace().kinds().any(|kind| matches!(
        kind,
        TraceRecordKind::FrameworkServiceResponseOutcome {
            service: TraceFrameworkServiceKind::ClipboardWriteText(ClipboardWritePurpose::Cut),
            outcome: TraceFrameworkServiceOutcome::Failed(
                FrameworkServiceFailure::PermissionDenied
            ),
        }
    )));
    let service_trace = failed_cut.trace().export_jsonl();
    assert!(service_trace.contains("\"name\":\"framework_service_response_outcome\""));
    assert!(service_trace.contains("\"failure\":\"permission_denied\""));

    let mut cut = mounted();
    install_controlled_font(&mut cut);
    focus(&mut cut);
    publish_editor(&mut cut);
    select_all(&mut cut);
    let owner = cut.index().nodes()[0].id().clone();
    cut.submit_command(owner, SemanticCommand::Cut, CommandOrigin::programmatic())
        .unwrap_or_else(|error| panic!("cut default is admitted: {error:?}"));
    cut.pump(PumpBudget::new(16, usize::MAX, usize::MAX, usize::MAX));
    let token = pending_service(&cut, |request| {
        matches!(
            request,
            FrameworkServiceRequest::ClipboardWriteText {
                purpose: ClipboardWritePurpose::Cut,
                ..
            }
        )
    })
    .token();
    cut.complete_framework_service(&token, FrameworkServiceResponse::ClipboardWriteText(Ok(())))
        .unwrap_or_else(|error| panic!("cut success is queued before editing: {error:?}"));
    assert_eq!(cut.state().text, "ab");
    cut.pump(PumpBudget::new(16, usize::MAX, usize::MAX, usize::MAX));
    assert_eq!(cut.state().text, "");
    assert_eq!(cut.state().observed_edits[0].kind, EditKind::Cut);

    let mut paste = mounted();
    install_controlled_font(&mut paste);
    focus(&mut paste);
    publish_editor(&mut paste);
    let owner = paste.index().nodes()[0].id().clone();
    paste
        .submit_command(owner, SemanticCommand::Paste, CommandOrigin::programmatic())
        .unwrap_or_else(|error| panic!("paste default is admitted: {error:?}"));
    paste.pump(PumpBudget::new(16, usize::MAX, usize::MAX, usize::MAX));
    let pending = pending_service(
        &paste,
        |request| matches!(request, FrameworkServiceRequest::ClipboardReadText { max_bytes } if *max_bytes > 0),
    );
    let token = pending.token();
    paste
        .complete_framework_service(
            &token,
            FrameworkServiceResponse::ClipboardReadText(Ok(ClipboardText::new(
                "x",
                ClipboardClassification::Public,
            ))),
        )
        .unwrap_or_else(|error| {
            panic!("paste data is queued, not inserted at callback time: {error:?}")
        });
    assert_eq!(paste.state().text, "ab");
    paste.pump(PumpBudget::new(16, usize::MAX, usize::MAX, usize::MAX));
    assert_eq!(paste.state().text, "abx");
    assert_eq!(paste.state().observed_edits[0].kind, EditKind::Paste);

    let mut unavailable = mounted();
    install_controlled_font(&mut unavailable);
    focus(&mut unavailable);
    publish_editor(&mut unavailable);
    let owner = unavailable.index().nodes()[0].id().clone();
    unavailable
        .submit_command(owner, SemanticCommand::Paste, CommandOrigin::programmatic())
        .unwrap_or_else(|error| panic!("paste default is admitted: {error:?}"));
    unavailable.pump(PumpBudget::new(16, usize::MAX, usize::MAX, usize::MAX));
    let token = pending_service(&unavailable, |request| {
        matches!(request, FrameworkServiceRequest::ClipboardReadText { .. })
    })
    .token();
    unavailable
        .complete_framework_service(
            &token,
            FrameworkServiceResponse::ClipboardReadText(Err(FrameworkServiceFailure::Unavailable)),
        )
        .unwrap_or_else(|error| panic!("unavailable is not converted into empty text: {error:?}"));
    unavailable.pump(PumpBudget::new(16, usize::MAX, usize::MAX, usize::MAX));
    assert_eq!(unavailable.state().text, "ab");
    assert!(unavailable.state().observed_edits.is_empty());
}

#[test]
fn clipboard_copy_is_suppressed_secret_and_unclassified_paste_is_rejected() {
    let mut secret = AppRuntime::<App>::mount(Document {
        sensitivity: TextSensitivity::Secret,
        ..mounted().state().clone()
    });
    install_controlled_font(&mut secret);
    focus(&mut secret);
    publish_editor(&mut secret);
    select_all(&mut secret);
    let owner = secret.index().nodes()[0].id().clone();
    secret
        .submit_command(owner, SemanticCommand::Copy, CommandOrigin::programmatic())
        .unwrap_or_else(|error| panic!("secret copy is routed for safe rejection: {error:?}"));
    secret.pump(PumpBudget::new(16, usize::MAX, usize::MAX, usize::MAX));
    assert!(!has_pending_clipboard_service(&secret));
    assert_eq!(secret.state().text, "ab");

    let mut runtime = mounted();
    install_controlled_font(&mut runtime);
    focus(&mut runtime);
    publish_editor(&mut runtime);
    let owner = runtime.index().nodes()[0].id().clone();
    runtime
        .submit_command(owner, SemanticCommand::Paste, CommandOrigin::programmatic())
        .unwrap_or_else(|error| panic!("paste is routed: {error:?}"));
    runtime.pump(PumpBudget::new(16, usize::MAX, usize::MAX, usize::MAX));
    let token = pending_service(&runtime, |request| {
        matches!(request, FrameworkServiceRequest::ClipboardReadText { .. })
    })
    .token();
    let response = FrameworkServiceResponse::ClipboardReadText(Ok(ClipboardText::new(
        "z",
        ClipboardClassification::Unclassified,
    )));
    runtime
        .complete_framework_service(&token, response.clone())
        .unwrap_or_else(|error| panic!("first result is admitted: {error:?}"));
    assert!(
        runtime
            .complete_framework_service(&token, response)
            .is_err()
    );
    runtime.pump(PumpBudget::new(16, usize::MAX, usize::MAX, usize::MAX));
    assert_eq!(runtime.state().text, "ab");
    assert!(runtime.state().observed_edits.is_empty());
    assert_eq!(runtime.status(), RuntimeStatus::Running);
}

#[test]
fn repeated_clipboard_writes_keep_their_fifo_service_effects() {
    let mut runtime = mounted();
    install_controlled_font(&mut runtime);
    focus(&mut runtime);
    publish_editor(&mut runtime);
    select_all(&mut runtime);
    let owner = runtime.index().nodes()[0].id().clone();
    for _ in 0..2 {
        runtime
            .submit_command(
                owner.clone(),
                SemanticCommand::Copy,
                CommandOrigin::programmatic(),
            )
            .unwrap_or_else(|error| panic!("copy default is admitted: {error:?}"));
    }
    runtime.pump(PumpBudget::new(32, usize::MAX, usize::MAX, usize::MAX));

    let tokens = runtime
        .pending_framework_services()
        .into_iter()
        .filter(|service| {
            matches!(
                service.request(),
                FrameworkServiceRequest::ClipboardWriteText {
                    purpose: ClipboardWritePurpose::Copy,
                    ..
                }
            )
        })
        .map(|service| service.token())
        .collect::<Vec<_>>();
    assert_eq!(tokens.len(), 2);
    for token in tokens {
        runtime
            .complete_framework_service(
                &token,
                FrameworkServiceResponse::ClipboardWriteText(Ok(())),
            )
            .unwrap_or_else(|error| panic!("both committed copies remain live: {error:?}"));
    }
    runtime.pump(PumpBudget::new(32, usize::MAX, usize::MAX, usize::MAX));

    assert_eq!(
        runtime
            .trace()
            .kinds()
            .filter(|kind| matches!(
                kind,
                TraceRecordKind::FrameworkServiceResponseOutcome {
                    service: TraceFrameworkServiceKind::ClipboardWriteText(
                        ClipboardWritePurpose::Copy
                    ),
                    outcome: TraceFrameworkServiceOutcome::Succeeded,
                }
            ))
            .count(),
        2
    );
    assert_eq!(runtime.status(), RuntimeStatus::Running);
}

#[test]
fn sensitive_clipboard_payload_can_only_enter_secret_document() {
    let mut secret = AppRuntime::<App>::mount(Document {
        sensitivity: TextSensitivity::Secret,
        ..mounted().state().clone()
    });
    install_controlled_font(&mut secret);
    focus(&mut secret);
    publish_editor(&mut secret);
    let owner = secret.index().nodes()[0].id().clone();
    secret
        .submit_command(owner, SemanticCommand::Paste, CommandOrigin::programmatic())
        .unwrap_or_else(|error| panic!("secret paste default is routed: {error:?}"));
    secret.pump(PumpBudget::new(16, usize::MAX, usize::MAX, usize::MAX));
    let token = pending_service(&secret, |request| {
        matches!(request, FrameworkServiceRequest::ClipboardReadText { .. })
    })
    .token();
    secret
        .complete_framework_service(
            &token,
            FrameworkServiceResponse::ClipboardReadText(Ok(ClipboardText::new(
                "z",
                ClipboardClassification::Sensitive,
            ))),
        )
        .unwrap_or_else(|error| {
            panic!("sensitive payload is admitted to secret document: {error:?}")
        });
    secret.pump(PumpBudget::new(16, usize::MAX, usize::MAX, usize::MAX));
    assert_eq!(secret.state().text, "abz");
    assert_eq!(secret.state().observed_edits[0].kind, EditKind::Paste);
}

#[test]
fn service_effect_staged_for_old_surface_is_not_exposed_after_publication_changes() {
    let mut runtime = mounted();
    install_controlled_font(&mut runtime);
    focus(&mut runtime);
    publish_editor(&mut runtime);
    select_all(&mut runtime);
    complete_pending_state_services(&mut runtime);
    let owner = runtime.index().nodes()[0].id().clone();
    runtime
        .submit_command(owner, SemanticCommand::Copy, CommandOrigin::programmatic())
        .unwrap_or_else(|error| panic!("copy command is admitted: {error:?}"));

    let routed = runtime.pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX));
    assert_eq!(routed.processed_envelopes(), 1);
    assert!(routed.remaining_queued_envelopes() > 0);
    assert!(!has_pending_clipboard_service(&runtime));

    publish_editor(&mut runtime);
    runtime.pump(PumpBudget::new(16, usize::MAX, usize::MAX, usize::MAX));
    assert!(!has_pending_clipboard_service(&runtime));
    assert_eq!(runtime.status(), RuntimeStatus::Running);
}

#[test]
fn clipboard_completion_is_revalidated_after_prior_queued_edits_and_cross_thread_delivery_is_once_only()
 {
    let mut stale = mounted();
    install_controlled_font(&mut stale);
    focus(&mut stale);
    publish_editor(&mut stale);
    let owner = stale.index().nodes()[0].id().clone();
    stale
        .submit_command(owner, SemanticCommand::Paste, CommandOrigin::programmatic())
        .unwrap_or_else(|error| panic!("paste default is admitted: {error:?}"));
    stale.pump(PumpBudget::new(16, usize::MAX, usize::MAX, usize::MAX));
    let token = pending_service(&stale, |request| {
        matches!(request, FrameworkServiceRequest::ClipboardReadText { .. })
    })
    .token();
    stale
        .submit_text(
            CommittedTextEvent::new("y", None)
                .unwrap_or_else(|_| unreachable!("fixture text is valid")),
        )
        .unwrap_or_else(|error| panic!("prior FIFO edit is accepted: {error:?}"));
    stale
        .complete_framework_service(
            &token,
            FrameworkServiceResponse::ClipboardReadText(Ok(ClipboardText::new(
                "x",
                ClipboardClassification::Public,
            ))),
        )
        .unwrap_or_else(|error| {
            panic!("clipboard completion queues behind the prior edit: {error:?}")
        });
    stale.pump(PumpBudget::new(24, usize::MAX, usize::MAX, usize::MAX));
    assert_eq!(stale.state().text, "aby");
    assert_eq!(stale.state().observed_edits.len(), 1);
    assert_eq!(stale.state().observed_edits[0].kind, EditKind::Insert);

    let mut threaded = mounted();
    install_controlled_font(&mut threaded);
    focus(&mut threaded);
    publish_editor(&mut threaded);
    let owner = threaded.index().nodes()[0].id().clone();
    threaded
        .submit_command(owner, SemanticCommand::Paste, CommandOrigin::programmatic())
        .unwrap_or_else(|error| panic!("paste default is admitted: {error:?}"));
    threaded.pump(PumpBudget::new(16, usize::MAX, usize::MAX, usize::MAX));
    let token = pending_service(&threaded, |request| {
        matches!(request, FrameworkServiceRequest::ClipboardReadText { .. })
    })
    .token();
    let completion = threaded
        .framework_service_response_completion(
            &token,
            FrameworkServiceResponse::ClipboardReadText(Ok(ClipboardText::new(
                "q",
                ClipboardClassification::Public,
            ))),
        )
        .unwrap_or_else(|error| panic!("cross-thread response is bound to the request: {error:?}"));
    std::thread::spawn(move || completion.submit())
        .join()
        .unwrap_or_else(|_| panic!("fake host thread completes without panicking"))
        .unwrap_or_else(|error| panic!("completion ingress accepts one response: {error:?}"));
    assert!(
        threaded
            .complete_framework_service(
                &token,
                FrameworkServiceResponse::ClipboardReadText(Err(
                    FrameworkServiceFailure::Unavailable,
                )),
            )
            .is_err()
    );
    threaded.pump(PumpBudget::new(16, usize::MAX, usize::MAX, usize::MAX));
    assert_eq!(threaded.state().text, "abq");
    assert_eq!(threaded.status(), RuntimeStatus::Running);
}

#[test]
fn committed_focus_derives_ime_candidate_geometry_and_text_cursor_services() {
    let mut runtime = mounted();
    install_controlled_font(&mut runtime);
    focus(&mut runtime);
    let surface_context = publish_editor(&mut runtime);
    runtime.pump(PumpBudget::new(16, usize::MAX, usize::MAX, usize::MAX));

    let owner = runtime.index().nodes()[0].id().clone();
    let input_method = pending_service(&runtime, |request| {
        matches!(
            request,
            FrameworkServiceRequest::InputMethod {
                enabled: true,
                candidate_area: Some(_),
                ..
            }
        )
    });
    assert_eq!(input_method.binding().owner(), &owner);
    assert!(input_method.binding().editing_session().is_some());
    assert!(input_method.binding().document_snapshot().is_some());
    assert!(input_method.binding().surface_context().is_some());

    complete_pending_state_services(&mut runtime);
    let pointer = PointerEvent::new(
        PointerId::new(5).unwrap_or_else(|| unreachable!("fixture pointer is non-zero")),
        PointerDeviceKind::Mouse,
        PointerPhase::Move,
        LogicalPoint::new(100.0, 20.0)
            .unwrap_or_else(|_| unreachable!("fixture pointer position is finite")),
        surface_context.clone(),
    );
    runtime
        .submit_pointer(pointer)
        .unwrap_or_else(|error| panic!("text cursor hit is routed: {error:?}"));
    runtime.pump(PumpBudget::new(16, usize::MAX, usize::MAX, usize::MAX));
    let cursor = pending_service(&runtime, |request| {
        matches!(request, FrameworkServiceRequest::Cursor { .. })
    });
    assert_eq!(cursor.binding().owner(), &owner);
    assert_eq!(cursor.binding().surface_context(), Some(&surface_context));
    assert!(matches!(
        cursor.request(),
        FrameworkServiceRequest::Cursor {
            shape: runenui_core::CursorShape::Text,
            visible: true
        }
    ));
    complete_pending_state_services(&mut runtime);
    assert_eq!(runtime.status(), RuntimeStatus::Running);
}

#[test]
fn pointer_text_selection_uses_displayed_map_and_cancels_once_on_owner_removal() {
    let config = RuntimeConfig::default().with_trace_config(TraceConfig::new(512));
    let mut runtime = AppRuntime::<App>::mount_with_config(mounted().state().clone(), config);
    install_controlled_font(&mut runtime);
    focus(&mut runtime);
    let surface_context = publish_editor(&mut runtime);
    let pointer_id =
        PointerId::new(31).unwrap_or_else(|| unreachable!("fixture pointer is non-zero"));
    let point = LogicalPoint::new(1.0, 18.0)
        .unwrap_or_else(|_| unreachable!("pointer fixture position is finite"));
    let pointer = PointerEvent::new(
        pointer_id,
        PointerDeviceKind::Mouse,
        PointerPhase::Down,
        point,
        surface_context.clone(),
    )
    .with_buttons(PointerButtons::new([PointerButton::Primary]))
    .with_changed_button(PointerButton::Primary);
    runtime
        .submit_pointer(pointer)
        .unwrap_or_else(|error| panic!("displayed editable hit is submitted: {error:?}"));
    runtime.pump(PumpBudget::new(16, usize::MAX, usize::MAX, usize::MAX));

    assert!(runtime.trace().records().any(|record| matches!(
        record.kind(),
        TraceRecordKind::PointerTextSelectionStarted { pointer_id } if pointer_id.get() == 31
    )));
    assert!(
        runtime
            .trace()
            .export_jsonl()
            .contains("\"name\":\"pointer_text_selection_started\"")
    );

    let drag_point = LogicalPoint::new(400.0, 18.0)
        .unwrap_or_else(|_| unreachable!("pointer drag position is finite"));
    runtime
        .submit_pointer(
            PointerEvent::new(
                pointer_id,
                PointerDeviceKind::Mouse,
                PointerPhase::Move,
                drag_point,
                surface_context.clone(),
            )
            .with_buttons(PointerButtons::new([PointerButton::Primary])),
        )
        .unwrap_or_else(|error| panic!("displayed pointer drag is submitted: {error:?}"));
    runtime.pump(PumpBudget::new(16, usize::MAX, usize::MAX, usize::MAX));
    assert!(runtime.trace().records().any(|record| matches!(
        record.kind(),
        TraceRecordKind::PointerTextSelectionUpdated { pointer_id: id } if *id == pointer_id
    )));
    assert!(
        runtime
            .trace()
            .export_jsonl()
            .contains("\"name\":\"pointer_text_selection_updated\"")
    );

    runtime
        .submit_pointer(
            PointerEvent::new(
                pointer_id,
                PointerDeviceKind::Mouse,
                PointerPhase::Up,
                LogicalPoint::new(500.0, 18.0)
                    .unwrap_or_else(|_| unreachable!("outside release position is finite")),
                surface_context,
            )
            .with_changed_button(PointerButton::Primary),
        )
        .unwrap_or_else(|error| {
            panic!("captured release outside the viewport is submitted: {error:?}")
        });
    runtime.pump(PumpBudget::new(16, usize::MAX, usize::MAX, usize::MAX));
    assert!(runtime.trace().records().any(|record| matches!(
        record.kind(),
        TraceRecordKind::PointerTextSelectionEnded { pointer_id: id } if *id == pointer_id
    )));

    let environment = StyleEnvironment::default();
    let publication = runtime
        .publish_surface(&SurfaceBuildContext::tight(
            &environment,
            LogicalSize::try_new(200.0, 40.0)
                .unwrap_or_else(|_| unreachable!("test surface is finite")),
        ))
        .unwrap_or_else(|error| panic!("pointer selection republishes: {error:?}"));
    let selection = publication
        .semantic_publication()
        .snapshot()
        .nodes()
        .first()
        .and_then(|node| node.editable())
        .unwrap_or_else(|| unreachable!("editable semantic state remains published"))
        .selection();
    assert_eq!(selection.anchor().byte_offset(), 0);
    assert_eq!(selection.active().byte_offset(), 2);
    assert!(!selection.is_collapsed());

    let cancellation_pointer =
        PointerId::new(33).unwrap_or_else(|| unreachable!("fixture pointer is non-zero"));
    runtime
        .submit_pointer(
            PointerEvent::new(
                cancellation_pointer,
                PointerDeviceKind::Mouse,
                PointerPhase::Down,
                point,
                publication.input_context().clone(),
            )
            .with_buttons(PointerButtons::new([PointerButton::Primary]))
            .with_changed_button(PointerButton::Primary),
        )
        .unwrap_or_else(|error| panic!("a fresh captured selection starts: {error:?}"));
    runtime.pump(PumpBudget::new(16, usize::MAX, usize::MAX, usize::MAX));

    runtime
        .submit_action(Action::RemoveEditor)
        .unwrap_or_else(|_| unreachable!("owner removal is queued"));
    runtime.pump(PumpBudget::new(16, usize::MAX, usize::MAX, usize::MAX));
    assert_eq!(
        runtime
            .trace()
            .records()
            .filter(|record| matches!(
                record.kind(),
                TraceRecordKind::PointerTextSelectionCancelled { pointer_id }
                    if pointer_id == &cancellation_pointer
            ))
            .count(),
        1
    );
    assert_eq!(runtime.status(), RuntimeStatus::Running);
}

#[test]
fn default_prevented_pointer_down_suppresses_selection_start() {
    let mut down_state = mounted().state().clone();
    down_state.prevent_pointer_down = true;
    let mut down_runtime = AppRuntime::<App>::mount(down_state);
    install_controlled_font(&mut down_runtime);
    focus(&mut down_runtime);
    let down_context = publish_editor(&mut down_runtime);
    let down_pointer = PointerId::new(35).unwrap_or_else(|| unreachable!("pointer ID is nonzero"));
    let origin =
        LogicalPoint::new(1.0, 18.0).unwrap_or_else(|_| unreachable!("pointer origin is finite"));
    down_runtime
        .submit_pointer(
            PointerEvent::new(
                down_pointer,
                PointerDeviceKind::Mouse,
                PointerPhase::Down,
                origin,
                down_context,
            )
            .with_buttons(PointerButtons::new([PointerButton::Primary]))
            .with_changed_button(PointerButton::Primary),
        )
        .unwrap_or_else(|error| panic!("default-prevented primary Down is routed: {error:?}"));
    down_runtime.pump(PumpBudget::new(16, usize::MAX, usize::MAX, usize::MAX));
    assert!(!down_runtime.trace().records().any(|record| matches!(
        record.kind(),
        TraceRecordKind::PointerTextSelectionStarted { pointer_id } if pointer_id.get() == 35
    )));
}

#[test]
fn shift_primary_click_preserves_selection_anchor_across_the_active_endpoint() {
    let mut state = mounted().state().clone();
    state.selection = 2;
    let mut runtime = AppRuntime::<App>::mount(state);
    install_controlled_font(&mut runtime);
    focus(&mut runtime);
    let surface_context = publish_editor(&mut runtime);
    let pointer_id =
        PointerId::new(32).unwrap_or_else(|| unreachable!("fixture pointer is non-zero"));
    let point = LogicalPoint::new(1.0, 18.0)
        .unwrap_or_else(|_| unreachable!("shift-click point is finite"));
    runtime
        .submit_pointer(
            PointerEvent::new(
                pointer_id,
                PointerDeviceKind::Mouse,
                PointerPhase::Down,
                point,
                surface_context.clone(),
            )
            .with_buttons(PointerButtons::new([PointerButton::Primary]))
            .with_changed_button(PointerButton::Primary)
            .with_modifiers(KeyModifiers::SHIFT),
        )
        .unwrap_or_else(|error| panic!("shift-click is admitted: {error:?}"));
    runtime.pump(PumpBudget::new(16, usize::MAX, usize::MAX, usize::MAX));
    runtime
        .submit_pointer(
            PointerEvent::new(
                pointer_id,
                PointerDeviceKind::Mouse,
                PointerPhase::Up,
                point,
                surface_context,
            )
            .with_changed_button(PointerButton::Primary)
            .with_modifiers(KeyModifiers::SHIFT),
        )
        .unwrap_or_else(|error| panic!("shift-click release is admitted: {error:?}"));
    runtime.pump(PumpBudget::new(16, usize::MAX, usize::MAX, usize::MAX));

    let environment = StyleEnvironment::default();
    let publication = runtime
        .publish_surface(&SurfaceBuildContext::tight(
            &environment,
            LogicalSize::try_new(200.0, 40.0)
                .unwrap_or_else(|_| unreachable!("test surface is finite")),
        ))
        .unwrap_or_else(|error| panic!("shift-click selection republishes: {error:?}"));
    let selection = publication
        .semantic_publication()
        .snapshot()
        .nodes()
        .first()
        .and_then(|node| node.editable())
        .unwrap_or_else(|| unreachable!("editable semantic state remains published"))
        .selection();
    assert_eq!(selection.anchor().byte_offset(), 2);
    assert_eq!(selection.active().byte_offset(), 0);
    assert!(!selection.is_collapsed());
}

#[test]
fn touch_text_selection_wins_only_after_its_validated_threshold_and_uses_displayed_map() {
    let config = RuntimeConfig::default().with_trace_config(TraceConfig::new(512));
    let mut runtime = AppRuntime::<App>::mount_with_config(mounted().state().clone(), config);
    install_controlled_font(&mut runtime);
    focus(&mut runtime);
    let surface_context = publish_editor(&mut runtime);
    let pointer_id =
        PointerId::new(32).unwrap_or_else(|| unreachable!("fixture pointer is non-zero"));
    let down_point = LogicalPoint::new(1.0, 18.0)
        .unwrap_or_else(|_| unreachable!("touch fixture position is finite"));
    runtime
        .submit_pointer(
            PointerEvent::new(
                pointer_id,
                PointerDeviceKind::Touch,
                PointerPhase::Down,
                down_point,
                surface_context.clone(),
            )
            .with_buttons(PointerButtons::new([PointerButton::Primary]))
            .with_changed_button(PointerButton::Primary),
        )
        .unwrap_or_else(|error| panic!("touch selection down is admitted: {error:?}"));
    runtime.pump(PumpBudget::new(16, usize::MAX, usize::MAX, usize::MAX));
    assert!(!runtime.trace().records().any(|record| matches!(
        record.kind(),
        TraceRecordKind::TouchGestureWon {
            pointer_id: id,
            ..
        } if *id == pointer_id
    )));

    let move_point = LogicalPoint::new(24.0, 18.0)
        .unwrap_or_else(|_| unreachable!("touch fixture position is finite"));
    runtime
        .submit_pointer(
            PointerEvent::new(
                pointer_id,
                PointerDeviceKind::Touch,
                PointerPhase::Move,
                move_point,
                surface_context,
            )
            .with_buttons(PointerButtons::new([PointerButton::Primary])),
        )
        .unwrap_or_else(|error| panic!("touch selection threshold move is admitted: {error:?}"));
    runtime.pump(PumpBudget::new(16, usize::MAX, usize::MAX, usize::MAX));

    assert!(runtime.trace().records().any(|record| matches!(
        record.kind(),
        TraceRecordKind::TouchGestureWon {
            pointer_id: id,
            gesture: runenui_runtime::TraceTouchGestureKind::TextSelection,
        } if *id == pointer_id
    )));
    assert!(runtime.trace().records().any(|record| matches!(
        record.kind(),
        TraceRecordKind::PointerTextSelectionStarted { pointer_id: id } if *id == pointer_id
    )));
    let environment = StyleEnvironment::default();
    let publication = runtime
        .publish_surface(&SurfaceBuildContext::tight(
            &environment,
            LogicalSize::try_new(200.0, 40.0)
                .unwrap_or_else(|_| unreachable!("test surface is finite")),
        ))
        .unwrap_or_else(|error| panic!("touch selection republishes: {error:?}"));
    let selection = publication
        .semantic_publication()
        .snapshot()
        .nodes()
        .first()
        .and_then(|node| node.editable())
        .unwrap_or_else(|| unreachable!("editable semantic state remains published"))
        .selection();
    assert!(!selection.is_collapsed());
    assert_eq!(runtime.status(), RuntimeStatus::Running);
}

#[test]
fn undo_and_redo_are_application_owned_transactions_over_committed_history_only() {
    let mut runtime = mounted();
    focus(&mut runtime);
    runtime
        .submit_text(
            CommittedTextEvent::new("x", None)
                .unwrap_or_else(|_| unreachable!("text is non-empty")),
        )
        .unwrap_or_else(|_| unreachable!("edit is queued"));
    runtime.pump(PumpBudget::new(8, usize::MAX, usize::MAX, usize::MAX));
    assert_eq!(runtime.state().text, "abx");
    assert_eq!(runtime.state().history, ["ab"]);

    let owner = runtime.index().nodes()[0].id().clone();
    runtime
        .submit_command(owner, SemanticCommand::Undo, CommandOrigin::programmatic())
        .unwrap_or_else(|_| unreachable!("undo follows the canonical command route"));
    runtime.pump(PumpBudget::new(8, usize::MAX, usize::MAX, usize::MAX));
    assert_eq!(runtime.state().text, "ab");
    assert!(runtime.state().history.is_empty());
    assert_eq!(runtime.state().redo, ["abx"]);
    assert_eq!(runtime.state().revision, 2);
    assert_eq!(runtime.status(), RuntimeStatus::Running);

    let owner = runtime.index().nodes()[0].id().clone();
    runtime
        .submit_command(owner, SemanticCommand::Redo, CommandOrigin::programmatic())
        .unwrap_or_else(|_| unreachable!("redo follows the canonical command route"));
    runtime.pump(PumpBudget::new(8, usize::MAX, usize::MAX, usize::MAX));
    assert_eq!(runtime.state().text, "abx");
    assert_eq!(runtime.state().history, ["ab"]);
    assert!(runtime.state().redo.is_empty());
    assert_eq!(runtime.state().revision, 3);
    assert_eq!(runtime.status(), RuntimeStatus::Running);

    let mut rejected = AppRuntime::<App>::mount(Document {
        reject_next: true,
        ..mounted().state().clone()
    });
    focus(&mut rejected);
    rejected
        .submit_text(
            CommittedTextEvent::new("x", None)
                .unwrap_or_else(|_| unreachable!("text is non-empty")),
        )
        .unwrap_or_else(|_| unreachable!("edit is queued"));
    rejected.pump(PumpBudget::new(8, usize::MAX, usize::MAX, usize::MAX));
    assert!(rejected.state().history.is_empty());
    assert!(rejected.state().redo.is_empty());
}

#[test]
fn backward_and_forward_deletion_use_the_same_transactional_route() {
    let environment = StyleEnvironment::default();
    let context = SurfaceBuildContext::tight(
        &environment,
        LogicalSize::try_new(200.0, 40.0)
            .unwrap_or_else(|_| unreachable!("test surface is finite")),
    );
    let mut backward = mounted();
    install_controlled_font(&mut backward);
    focus(&mut backward);
    backward
        .publish_surface(&context)
        .unwrap_or_else(|error| panic!("backward-delete surface publishes: {error:?}"));
    let owner = backward.index().nodes()[0].id().clone();
    backward
        .submit_command(
            owner,
            SemanticCommand::DeleteBackward,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|error| panic!("backward delete is routed: {error:?}"));
    backward.pump(PumpBudget::new(8, usize::MAX, usize::MAX, usize::MAX));
    assert_eq!(backward.state().text, "a");
    assert_eq!(backward.state().selection, 1);
    assert_eq!(backward.state().history, ["ab"]);

    let mut forward = mounted();
    install_controlled_font(&mut forward);
    focus(&mut forward);
    forward
        .publish_surface(&context)
        .unwrap_or_else(|error| panic!("forward-delete surface publishes: {error:?}"));
    let owner = forward.index().nodes()[0].id().clone();
    forward
        .submit_command(
            owner.clone(),
            SemanticCommand::MoveBackward,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|error| panic!("backward movement is routed: {error:?}"));
    forward.pump(PumpBudget::new(8, usize::MAX, usize::MAX, usize::MAX));
    forward
        .submit_command(
            owner,
            SemanticCommand::DeleteForward,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|error| panic!("forward delete is routed: {error:?}"));
    forward.pump(PumpBudget::new(8, usize::MAX, usize::MAX, usize::MAX));
    assert_eq!(forward.state().text, "a");
    assert_eq!(forward.state().selection, 1);
    assert_eq!(forward.state().history, ["ab"]);
    assert_eq!(forward.status(), RuntimeStatus::Running);
}

#[test]
fn native_backspace_and_normalized_select_all_use_the_keyboard_default_route() {
    let environment = StyleEnvironment::default();
    let context = SurfaceBuildContext::tight(
        &environment,
        LogicalSize::try_new(200.0, 40.0)
            .unwrap_or_else(|_| unreachable!("test surface is finite")),
    );
    let mut runtime = mounted();
    install_controlled_font(&mut runtime);
    focus(&mut runtime);
    runtime
        .publish_surface(&context)
        .unwrap_or_else(|error| panic!("keyboard editing surface publishes: {error:?}"));
    runtime
        .submit_keyboard(KeyboardEvent::new(
            KeyboardPhase::Down,
            PhysicalKey::Code(String::from("Backspace")),
            LogicalKey::Backspace,
            KeyModifiers::NONE,
            false,
            KeyLocation::Standard,
            KeyboardCompositionState::Inactive,
            None,
        ))
        .unwrap_or_else(|error| panic!("native Backspace is routed: {error:?}"));
    runtime.pump(PumpBudget::new(16, usize::MAX, usize::MAX, usize::MAX));
    assert_eq!(runtime.state().text, "a");
    assert_eq!(runtime.state().history, ["ab"]);
    runtime
        .publish_surface(&context)
        .unwrap_or_else(|error| panic!("post-delete selection map publishes: {error:?}"));

    runtime
        .submit_keyboard(KeyboardEvent::new(
            KeyboardPhase::Down,
            PhysicalKey::Code(String::from("KeyA")),
            LogicalKey::Command(SemanticCommand::SelectAll),
            KeyModifiers::META,
            false,
            KeyLocation::Standard,
            KeyboardCompositionState::Inactive,
            None,
        ))
        .unwrap_or_else(|error| panic!("normalized Select All shortcut is routed: {error:?}"));
    runtime.pump(PumpBudget::new(16, usize::MAX, usize::MAX, usize::MAX));
    let publication = runtime
        .publish_surface(&context)
        .unwrap_or_else(|error| panic!("selected text republishes: {error:?}"));
    let selection = publication
        .semantic_publication()
        .snapshot()
        .nodes()
        .first()
        .and_then(|node| node.editable())
        .unwrap_or_else(|| unreachable!("editable semantic selection is published"))
        .selection();
    assert_eq!(
        selection
            .anchor()
            .byte_offset()
            .min(selection.active().byte_offset()),
        0
    );
    assert_eq!(
        selection
            .anchor()
            .byte_offset()
            .max(selection.active().byte_offset()),
        1
    );
    assert!(!selection.is_collapsed());

    let initial_text = runtime.state().text.clone();
    runtime
        .submit_keyboard(KeyboardEvent::new(
            KeyboardPhase::Down,
            PhysicalKey::Code(String::from("Escape")),
            LogicalKey::Escape,
            KeyModifiers::NONE,
            false,
            KeyLocation::Standard,
            KeyboardCompositionState::Inactive,
            None,
        ))
        .unwrap_or_else(|error| panic!("Escape reaches the editor default route: {error:?}"));
    runtime.pump(PumpBudget::new(16, usize::MAX, usize::MAX, usize::MAX));
    let collapsed = runtime
        .publish_surface(&context)
        .unwrap_or_else(|error| panic!("collapsed selection republishes: {error:?}"))
        .semantic_publication()
        .snapshot()
        .nodes()
        .first()
        .and_then(|node| node.editable())
        .unwrap_or_else(|| unreachable!("editable semantics remain published"))
        .selection();
    assert!(
        collapsed.is_collapsed(),
        "Escape collapses the text selection"
    );
    assert_eq!(collapsed.active().byte_offset(), 0);
    assert_eq!(
        runtime.state().text,
        initial_text,
        "Escape never inserts text"
    );

    runtime
        .submit_keyboard(KeyboardEvent::new(
            KeyboardPhase::Down,
            PhysicalKey::Code(String::from("Escape")),
            LogicalKey::Escape,
            KeyModifiers::NONE,
            false,
            KeyLocation::Standard,
            KeyboardCompositionState::Inactive,
            None,
        ))
        .unwrap_or_else(|error| panic!("collapsed Escape remains routable: {error:?}"));
    runtime.pump(PumpBudget::new(16, usize::MAX, usize::MAX, usize::MAX));
    let still_collapsed = runtime
        .publish_surface(&context)
        .unwrap_or_else(|error| panic!("unchanged selection republishes: {error:?}"))
        .semantic_publication()
        .snapshot()
        .nodes()
        .first()
        .and_then(|node| node.editable())
        .unwrap_or_else(|| unreachable!("editable semantics remain published"))
        .selection();
    assert!(
        still_collapsed.is_collapsed(),
        "Escape on a caret does not move or mutate text"
    );
    assert_eq!(runtime.state().text, initial_text);
}

#[test]
fn editable_vertical_keys_move_and_extend_without_retargeting_focus_navigation() {
    let mut state = mounted().state().clone();
    state.text = "one two\nthree four\nfive six".to_owned();
    state.selection = 2;
    let mut runtime = AppRuntime::<App>::mount(state);
    install_controlled_font(&mut runtime);
    focus(&mut runtime);
    publish_editor(&mut runtime);

    keyboard_down(&mut runtime, LogicalKey::ArrowDown, KeyModifiers::NONE);
    let first_down = runtime
        .publish_surface(&SurfaceBuildContext::tight(
            &StyleEnvironment::default(),
            LogicalSize::try_new(200.0, 40.0)
                .unwrap_or_else(|_| unreachable!("test surface is finite")),
        ))
        .unwrap_or_else(|error| panic!("first vertical move publishes: {error:?}"));
    let selection = first_down
        .semantic_publication()
        .snapshot()
        .nodes()
        .first()
        .and_then(|node| node.editable())
        .unwrap_or_else(|| unreachable!("editable state remains published"))
        .selection();
    assert!(selection.is_collapsed());
    assert!((8..19).contains(&selection.active().byte_offset()));

    keyboard_down(&mut runtime, LogicalKey::ArrowDown, KeyModifiers::NONE);
    let second_down = runtime
        .publish_surface(&SurfaceBuildContext::tight(
            &StyleEnvironment::default(),
            LogicalSize::try_new(200.0, 40.0)
                .unwrap_or_else(|_| unreachable!("test surface is finite")),
        ))
        .unwrap_or_else(|error| panic!("repeated vertical move publishes: {error:?}"));
    let third_line_caret = second_down
        .semantic_publication()
        .snapshot()
        .nodes()
        .first()
        .and_then(|node| node.editable())
        .unwrap_or_else(|| unreachable!("editable state remains published"))
        .selection()
        .active();
    assert!(third_line_caret.byte_offset() >= 19);

    keyboard_down(&mut runtime, LogicalKey::ArrowUp, KeyModifiers::SHIFT);
    let extended = runtime
        .publish_surface(&SurfaceBuildContext::tight(
            &StyleEnvironment::default(),
            LogicalSize::try_new(200.0, 40.0)
                .unwrap_or_else(|_| unreachable!("test surface is finite")),
        ))
        .unwrap_or_else(|error| panic!("vertical extension publishes: {error:?}"));
    let extended_selection = extended
        .semantic_publication()
        .snapshot()
        .nodes()
        .first()
        .and_then(|node| node.editable())
        .unwrap_or_else(|| unreachable!("editable state remains published"))
        .selection();
    assert_eq!(
        extended_selection.anchor().byte_offset(),
        third_line_caret.byte_offset()
    );
    assert!((8..19).contains(&extended_selection.active().byte_offset()));
    assert!(!extended_selection.is_collapsed());
}

#[test]
fn owner_removal_retires_live_authority_drains_queued_rejection_and_keeps_app_history() {
    let mut runtime = mounted();
    focus(&mut runtime);
    runtime
        .submit_text(
            CommittedTextEvent::new("x", None)
                .unwrap_or_else(|_| unreachable!("text is non-empty")),
        )
        .unwrap_or_else(|_| unreachable!("edit ingress is queued"));
    runtime
        .submit_action(Action::RemoveEditor)
        .unwrap_or_else(|_| unreachable!("removal is queued after ingress"));
    runtime.pump(PumpBudget::new(16, usize::MAX, usize::MAX, usize::MAX));
    assert_eq!(runtime.state().text, "ab");
    assert!(runtime.state().history.is_empty());
    assert!(!runtime.state().editor_visible);
    assert_eq!(runtime.__editing_session_counts_for_test(), (0, 0));
    assert_eq!(runtime.status(), RuntimeStatus::Running);

    let mut committed = mounted();
    focus(&mut committed);
    committed
        .submit_text(
            CommittedTextEvent::new("x", None)
                .unwrap_or_else(|_| unreachable!("text is non-empty")),
        )
        .unwrap_or_else(|_| unreachable!("edit is queued"));
    committed.pump(PumpBudget::new(8, usize::MAX, usize::MAX, usize::MAX));
    committed
        .submit_action(Action::RemoveEditor)
        .unwrap_or_else(|_| unreachable!("removal is queued"));
    committed.pump(PumpBudget::new(8, usize::MAX, usize::MAX, usize::MAX));
    assert_eq!(committed.state().history, ["ab"]);
    assert_eq!(committed.state().text, "abx");
    assert_eq!(committed.__editing_session_counts_for_test(), (0, 0));
}

#[test]
fn shutdown_cancels_queued_edit_and_retires_all_editing_authority() {
    let mut runtime = mounted();
    focus(&mut runtime);
    runtime
        .submit_text(
            CommittedTextEvent::new("x", None)
                .unwrap_or_else(|_| unreachable!("text is non-empty")),
        )
        .unwrap_or_else(|error| panic!("edit ingress is queued: {error:?}"));
    let routed = runtime.pump(PumpBudget::new(1, usize::MAX, usize::MAX, usize::MAX));
    assert_eq!(routed.remaining_queued_envelopes(), 1);
    assert_eq!(runtime.state().text, "ab");

    let report = runtime.shutdown();
    assert_eq!(report.cancelled_queued_envelopes(), 1);
    assert_eq!(runtime.__editing_session_counts_for_test(), (0, 0));
    assert_eq!(runtime.state().text, "ab");
    assert_eq!(runtime.status(), RuntimeStatus::Closed);
}

#[test]
fn rejected_prefix_restores_authoritative_projection_without_mutating_document() {
    let mut runtime = AppRuntime::<App>::mount(Document {
        reject_next: true,
        ..mounted().state().clone()
    });
    focus(&mut runtime);
    runtime
        .submit_text(
            CommittedTextEvent::new("x", None)
                .unwrap_or_else(|_| unreachable!("text is non-empty")),
        )
        .unwrap_or_else(|_| unreachable!("focused editor accepts text"));
    runtime.pump(PumpBudget::new(8, usize::MAX, usize::MAX, usize::MAX));
    assert_eq!(runtime.state().text, "ab");
    assert_eq!(runtime.state().revision, 0);
    assert_eq!(runtime.status(), RuntimeStatus::Running);
}

#[test]
fn burst_edits_keep_fifo_predecessors_and_rejected_suffix_drains_without_retargeting() {
    let mut runtime = AppRuntime::<App>::mount(Document {
        reject_next: true,
        ..mounted().state().clone()
    });
    focus(&mut runtime);
    for text in ["x", "y"] {
        runtime
            .submit_text(
                CommittedTextEvent::new(text, None)
                    .unwrap_or_else(|_| unreachable!("text is non-empty")),
            )
            .unwrap_or_else(|_| unreachable!("bounded burst is accepted"));
    }
    runtime.pump(PumpBudget::new(16, usize::MAX, usize::MAX, usize::MAX));
    assert_eq!(runtime.state().text, "ab");
    assert_eq!(runtime.state().revision, 0);
    assert_eq!(runtime.status(), RuntimeStatus::Running);

    runtime
        .submit_text(
            CommittedTextEvent::new("z", None)
                .unwrap_or_else(|_| unreachable!("text is non-empty")),
        )
        .unwrap_or_else(|_| unreachable!("drained suffix reopens ingress"));
}

#[test]
fn transformed_prefix_rebases_a_non_overlapping_dependent_suffix() {
    let mut runtime = AppRuntime::<App>::mount(Document {
        transform_next: true,
        ..mounted().state().clone()
    });
    focus(&mut runtime);
    for text in ["x", "y"] {
        runtime
            .submit_text(
                CommittedTextEvent::new(text, None)
                    .unwrap_or_else(|_| unreachable!("text is non-empty")),
            )
            .unwrap_or_else(|_| unreachable!("bounded burst is accepted"));
    }
    runtime.pump(PumpBudget::new(16, usize::MAX, usize::MAX, usize::MAX));
    assert_eq!(runtime.state().text, "abXy");
    assert_eq!(runtime.state().revision, 2);
    assert_eq!(runtime.status(), RuntimeStatus::Running);
}

#[test]
fn inverse_and_grouping_hints_are_deterministic_but_history_remains_application_owned() {
    let mut runtime = AppRuntime::<App>::mount(Document {
        transform_next: true,
        ..mounted().state().clone()
    });
    focus(&mut runtime);
    for text in ["x", "y"] {
        runtime
            .submit_text(
                CommittedTextEvent::new(text, None)
                    .unwrap_or_else(|_| unreachable!("text is non-empty")),
            )
            .unwrap_or_else(|error| panic!("edit is queued: {error:?}"));
    }
    runtime.pump(PumpBudget::new(16, usize::MAX, usize::MAX, usize::MAX));

    let observations = &runtime.state().observed_edits;
    assert_eq!(observations.len(), 2);
    assert_eq!(observations[0].kind, EditKind::Insert);
    assert_eq!(observations[0].inverse_range, Some(2..3));
    assert_eq!(observations[0].inverse_text.as_deref(), Some(""));
    assert!(!observations[0].has_predecessor);
    assert!(observations[0].group.is_some());
    assert_eq!(observations[1].kind, EditKind::Insert);
    assert_eq!(observations[1].inverse_range, Some(3..4));
    assert_eq!(observations[1].inverse_text.as_deref(), Some(""));
    assert!(observations[1].has_predecessor);
    assert_eq!(observations[1].group, observations[0].group);
    assert_eq!(runtime.state().history, ["ab", "abX"]);

    let mut rejected = AppRuntime::<App>::mount(Document {
        reject_next: true,
        ..mounted().state().clone()
    });
    focus(&mut rejected);
    rejected
        .submit_text(
            CommittedTextEvent::new("z", None)
                .unwrap_or_else(|_| unreachable!("text is non-empty")),
        )
        .unwrap_or_else(|error| panic!("rejected edit is queued: {error:?}"));
    rejected.pump(PumpBudget::new(8, usize::MAX, usize::MAX, usize::MAX));
    assert_eq!(rejected.state().observed_edits.len(), 1);
    assert!(rejected.state().observed_edits[0].inverse_range.is_some());
    assert!(rejected.state().observed_edits[0].group.is_some());
    assert!(rejected.state().history.is_empty());
}

#[test]
fn ordinary_action_cannot_return_an_edit_resolution() {
    struct InvalidApp;
    impl UiApp for InvalidApp {
        type State = ();
        type Action = Action;
        type HostProtocol = NoHostProtocol;

        fn root(_: &Self::State) -> Element<Self::Action> {
            Element::new(Editor)
        }

        fn update(
            _: &mut Self::State,
            _: Self::Action,
        ) -> UpdateOutput<Self::Action, Self::HostProtocol> {
            let namespace = runenui_core::__runtime::RuntimeNamespace::__runtime_new();
            UpdateOutput::edit(EditResolution::accepted(
                namespace.__runtime_edit_request_id(1),
                TextDocumentSnapshot::new(TextDocumentId::new(1), TextDocumentRevision::ZERO),
            ))
        }
    }

    let mut runtime = AppRuntime::<InvalidApp>::mount(());
    runtime
        .submit_action(Action::Ordinary)
        .unwrap_or_else(|_| unreachable!("ordinary action is queued"));
    runtime.pump(PumpBudget::new(8, usize::MAX, usize::MAX, usize::MAX));
    assert_eq!(
        runtime.status(),
        RuntimeStatus::Terminal(RuntimeTerminalReason::Poisoned)
    );
}

#[test]
fn missing_edit_resolution_poisons_after_application_mutation_without_claiming_rollback() {
    struct MissingResolutionApp;
    impl UiApp for MissingResolutionApp {
        type State = Document;
        type Action = Action;
        type HostProtocol = NoHostProtocol;

        fn root(state: &Self::State) -> Element<Self::Action> {
            editor_root(state)
        }

        fn update(
            state: &mut Self::State,
            action: Self::Action,
        ) -> UpdateOutput<Self::Action, Self::HostProtocol> {
            match action {
                Action::Ordinary | Action::RemoveEditor => UpdateOutput::effects(Effects::none()),
                Action::Edit(intent) => {
                    let range = intent.replacement();
                    state
                        .text
                        .replace_range(range.start()..range.end(), intent.replacement_text());
                    state.selection = intent.proposed_selection().active();
                    state.revision += 1;
                    UpdateOutput::effects(Effects::none())
                }
            }
        }
    }

    let mut runtime = AppRuntime::<MissingResolutionApp>::mount(mounted().state().clone());
    focus_generic(&mut runtime);
    runtime
        .submit_text(
            CommittedTextEvent::new("x", None)
                .unwrap_or_else(|_| unreachable!("text is non-empty")),
        )
        .unwrap_or_else(|_| unreachable!("edit is queued"));
    runtime.pump(PumpBudget::new(8, usize::MAX, usize::MAX, usize::MAX));
    assert_eq!(runtime.state().text, "abx");
    assert_eq!(runtime.state().revision, 1);
    assert_eq!(
        runtime.status(),
        RuntimeStatus::Terminal(RuntimeTerminalReason::Poisoned)
    );
}

#[test]
fn inconsistent_resolution_refuses_effect_commit_before_terminal_poison() {
    struct InconsistentResolutionApp;
    impl UiApp for InconsistentResolutionApp {
        type State = Document;
        type Action = Action;
        type HostProtocol = NoHostProtocol;

        fn root(state: &Self::State) -> Element<Self::Action> {
            editor_root(state)
        }

        fn update(
            state: &mut Self::State,
            action: Self::Action,
        ) -> UpdateOutput<Self::Action, Self::HostProtocol> {
            match action {
                Action::Ordinary | Action::RemoveEditor => {
                    state.ordinary_count += 1;
                    UpdateOutput::effects(Effects::none())
                }
                Action::Edit(intent) => UpdateOutput::edit_with_effects(
                    Effects::action(Action::Ordinary),
                    EditResolution::accepted(
                        intent.request().clone(),
                        TextDocumentSnapshot::new(
                            TextDocumentId::new(1),
                            TextDocumentRevision::new(state.revision),
                        ),
                    ),
                ),
            }
        }
    }

    let mut runtime = AppRuntime::<InconsistentResolutionApp>::mount(mounted().state().clone());
    focus_generic(&mut runtime);
    runtime
        .submit_text(
            CommittedTextEvent::new("x", None)
                .unwrap_or_else(|_| unreachable!("text is non-empty")),
        )
        .unwrap_or_else(|_| unreachable!("edit is queued"));
    runtime.pump(PumpBudget::new(8, usize::MAX, usize::MAX, usize::MAX));
    assert_eq!(runtime.state().ordinary_count, 0);
    assert_eq!(runtime.state().text, "ab");
    assert_eq!(
        runtime.status(),
        RuntimeStatus::Terminal(RuntimeTerminalReason::Poisoned)
    );
}

#[test]
fn foreign_resolution_request_poisons_after_application_mutation() {
    struct ForeignResolutionApp;
    impl UiApp for ForeignResolutionApp {
        type State = Document;
        type Action = Action;
        type HostProtocol = NoHostProtocol;

        fn root(state: &Self::State) -> Element<Self::Action> {
            editor_root(state)
        }

        fn update(
            state: &mut Self::State,
            action: Self::Action,
        ) -> UpdateOutput<Self::Action, Self::HostProtocol> {
            match action {
                Action::Ordinary | Action::RemoveEditor => UpdateOutput::effects(Effects::none()),
                Action::Edit(intent) => {
                    let request = intent.request().clone();
                    let returned_request = state
                        .saved_request
                        .replace(request.clone())
                        .unwrap_or(request);
                    let range = intent.replacement();
                    state
                        .text
                        .replace_range(range.start()..range.end(), intent.replacement_text());
                    state.selection = intent.proposed_selection().active();
                    state.revision += 1;
                    UpdateOutput::edit(EditResolution::accepted(
                        returned_request,
                        TextDocumentSnapshot::new(
                            TextDocumentId::new(1),
                            TextDocumentRevision::new(state.revision),
                        ),
                    ))
                }
            }
        }
    }

    let mut runtime = AppRuntime::<ForeignResolutionApp>::mount(mounted().state().clone());
    focus_generic(&mut runtime);
    for text in ["x", "y"] {
        runtime
            .submit_text(
                CommittedTextEvent::new(text, None)
                    .unwrap_or_else(|_| unreachable!("text is non-empty")),
            )
            .unwrap_or_else(|error| panic!("edit is queued: {error:?}"));
        runtime.pump(PumpBudget::new(8, usize::MAX, usize::MAX, usize::MAX));
    }
    assert_eq!(runtime.state().text, "abxy");
    assert_eq!(
        runtime.status(),
        RuntimeStatus::Terminal(RuntimeTerminalReason::Poisoned)
    );
}

#[test]
fn editable_semantics_use_the_retained_caret_map_and_runtime_selection() {
    let mut runtime = mounted();
    install_controlled_font(&mut runtime);
    focus(&mut runtime);
    let environment = StyleEnvironment::default();
    let context = SurfaceBuildContext::tight(
        &environment,
        LogicalSize::try_new(200.0, 40.0)
            .unwrap_or_else(|_| unreachable!("test surface is finite")),
    );
    let first = runtime
        .publish_surface(&context)
        .unwrap_or_else(|error| panic!("editable surface publishes: {error:?}"));
    let first_node = first
        .semantic_publication()
        .snapshot()
        .nodes()
        .first()
        .unwrap_or_else(|| unreachable!("editor publishes one semantic node"));
    let editable = first_node
        .editable()
        .unwrap_or_else(|| panic!("correlated editable facts publish: {first_node:?}"));
    assert_eq!(editable.value(), Some("ab"));
    assert_eq!(editable.caret_offsets(), Some([0, 1, 2].as_slice()));
    assert_eq!(editable.selection().active().byte_offset(), 2);
    for unavailable in [
        SemanticAction::Copy,
        SemanticAction::Cut,
        SemanticAction::Paste,
    ] {
        assert!(!first_node.supported_actions().contains(&unavailable));
        assert!(
            runtime
                .submit_semantic_action(runenui_core::SemanticActionRequest::new(
                    first.semantic_publication().snapshot().surface_id().clone(),
                    first_node.id().clone(),
                    unavailable,
                ))
                .is_err()
        );
    }

    runtime
        .submit_semantic_action(runenui_core::SemanticActionRequest::new(
            first.semantic_publication().snapshot().surface_id().clone(),
            first_node.id().clone(),
            SemanticAction::MoveBackward,
        ))
        .unwrap_or_else(|_| unreachable!("published editable action is admitted"));
    runtime.pump(PumpBudget::new(8, usize::MAX, usize::MAX, usize::MAX));
    assert_eq!(runtime.state().text, "ab");
    let stale = runtime
        .submit_semantic_action(runenui_core::SemanticActionRequest::new(
            first.semantic_publication().snapshot().surface_id().clone(),
            first_node.id().clone(),
            SemanticAction::MoveForward,
        ))
        .expect_err("a selection change invalidates the previously published authority");
    assert_eq!(stale.kind(), SubmitSemanticActionErrorKind::StaleAuthority);

    let second = runtime
        .publish_surface(&context)
        .unwrap_or_else(|error| panic!("moved selection republishes: {error:?}"));
    let moved = second.semantic_publication().snapshot().nodes()[0]
        .editable()
        .unwrap_or_else(|| unreachable!("editable projection remains correlated"));
    assert_eq!(moved.selection().active().byte_offset(), 1);
    let revision_zero_selection = moved.selection();
    runtime
        .submit_semantic_action(runenui_core::SemanticActionRequest::set_selection(
            second
                .semantic_publication()
                .snapshot()
                .surface_id()
                .clone(),
            second.semantic_publication().snapshot().nodes()[0]
                .id()
                .clone(),
            revision_zero_selection,
        ))
        .unwrap_or_else(|error| panic!("exact retained selection is admitted: {error:?}"));
    runtime.pump(PumpBudget::new(8, usize::MAX, usize::MAX, usize::MAX));
    let selected = runtime
        .publish_surface(&context)
        .unwrap_or_else(|error| panic!("selected semantic surface republishes: {error:?}"));
    runtime
        .submit_semantic_action(runenui_core::SemanticActionRequest::replace_selection(
            selected
                .semantic_publication()
                .snapshot()
                .surface_id()
                .clone(),
            selected.semantic_publication().snapshot().nodes()[0]
                .id()
                .clone(),
            "Z",
        ))
        .unwrap_or_else(|error| panic!("semantic replacement is admitted: {error:?}"));
    runtime.pump(PumpBudget::new(8, usize::MAX, usize::MAX, usize::MAX));
    assert_eq!(runtime.state().text, "aZb");
    let third = runtime
        .publish_surface(&context)
        .unwrap_or_else(|error| panic!("edited semantic surface publishes: {error:?}"));
    let stale_selection = runtime
        .submit_semantic_action(runenui_core::SemanticActionRequest::set_selection(
            third.semantic_publication().snapshot().surface_id().clone(),
            third.semantic_publication().snapshot().nodes()[0]
                .id()
                .clone(),
            revision_zero_selection,
        ))
        .expect_err("selection coordinates from the prior document revision are rejected");
    assert_eq!(
        stale_selection.kind(),
        SubmitSemanticActionErrorKind::UnavailableAction
    );
    assert_eq!(runtime.status(), RuntimeStatus::Running);
}

#[test]
fn read_only_and_disabled_editable_admission_fail_closed() {
    let environment = StyleEnvironment::default();
    let context = SurfaceBuildContext::tight(
        &environment,
        LogicalSize::try_new(200.0, 40.0)
            .unwrap_or_else(|_| unreachable!("test surface is finite")),
    );

    let mut read_only = AppRuntime::<App>::mount(Document {
        read_only: true,
        ..mounted().state().clone()
    });
    install_controlled_font(&mut read_only);
    focus(&mut read_only);
    let rejected_text = read_only
        .submit_text(
            CommittedTextEvent::new("x", None)
                .unwrap_or_else(|_| unreachable!("text is non-empty")),
        )
        .expect_err("read-only editing rejects committed text before routing");
    assert_eq!(
        rejected_text.kind(),
        SubmitTextErrorKind::EditingUnavailable
    );
    let publication = read_only
        .publish_surface(&context)
        .unwrap_or_else(|error| panic!("read-only surface publishes: {error:?}"));
    let node = &publication.semantic_publication().snapshot().nodes()[0];
    assert!(
        node.supported_actions()
            .contains(&SemanticAction::SetSelection)
    );
    assert!(
        !node
            .supported_actions()
            .contains(&SemanticAction::ReplaceSelection)
    );
    let error = read_only
        .submit_semantic_action(runenui_core::SemanticActionRequest::replace_selection(
            publication
                .semantic_publication()
                .snapshot()
                .surface_id()
                .clone(),
            node.id().clone(),
            "x",
        ))
        .expect_err("read-only semantic replacement is not advertised or admitted");
    assert_eq!(
        error.kind(),
        SubmitSemanticActionErrorKind::UnsupportedAction
    );

    let mut disabled = AppRuntime::<App>::mount(Document {
        disabled: true,
        ..mounted().state().clone()
    });
    install_controlled_font(&mut disabled);
    let publication = disabled
        .publish_surface(&context)
        .unwrap_or_else(|error| panic!("disabled surface publishes: {error:?}"));
    let node = &publication.semantic_publication().snapshot().nodes()[0];
    let error = disabled
        .submit_semantic_action(runenui_core::SemanticActionRequest::replace_selection(
            publication
                .semantic_publication()
                .snapshot()
                .surface_id()
                .clone(),
            node.id().clone(),
            "x",
        ))
        .expect_err("disabled semantic replacement is rejected at exact admission");
    assert_eq!(
        error.kind(),
        SubmitSemanticActionErrorKind::UnavailableAction
    );
    assert_eq!(disabled.state().text, "ab");
}

#[test]
fn extend_and_select_all_update_only_the_owner_local_runtime_selection() {
    let mut runtime = mounted();
    install_controlled_font(&mut runtime);
    focus(&mut runtime);
    let environment = StyleEnvironment::default();
    let context = SurfaceBuildContext::tight(
        &environment,
        LogicalSize::try_new(200.0, 40.0)
            .unwrap_or_else(|_| unreachable!("test surface is finite")),
    );
    runtime
        .publish_surface(&context)
        .unwrap_or_else(|error| panic!("selection surface publishes: {error:?}"));
    let owner = runtime.index().nodes()[0].id().clone();
    runtime
        .submit_command(
            owner.clone(),
            SemanticCommand::ExtendBackward,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|error| panic!("selection extension is routed: {error:?}"));
    runtime.pump(PumpBudget::new(8, usize::MAX, usize::MAX, usize::MAX));
    let extended = runtime
        .publish_surface(&context)
        .unwrap_or_else(|error| panic!("extended selection publishes: {error:?}"));
    let selection = extended.semantic_publication().snapshot().nodes()[0]
        .editable()
        .unwrap_or_else(|| unreachable!("editable selection publishes"))
        .selection();
    assert_eq!(selection.anchor().byte_offset(), 2);
    assert_eq!(selection.active().byte_offset(), 1);
    assert_eq!(runtime.state().selection, 2);

    runtime
        .submit_command(
            owner,
            SemanticCommand::SelectAll,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|error| panic!("select-all is routed: {error:?}"));
    runtime.pump(PumpBudget::new(8, usize::MAX, usize::MAX, usize::MAX));
    let selected = runtime
        .publish_surface(&context)
        .unwrap_or_else(|error| panic!("select-all publishes: {error:?}"));
    let selection = selected.semantic_publication().snapshot().nodes()[0]
        .editable()
        .unwrap_or_else(|| unreachable!("editable selection publishes"))
        .selection();
    assert_eq!(selection.anchor().byte_offset(), 0);
    assert_eq!(selection.active().byte_offset(), 2);
    assert_eq!(runtime.state().selection, 2);
}

#[test]
fn preedit_uses_the_retained_layout_without_committing_and_commit_inserts_once() {
    let mut runtime = mounted();
    install_controlled_font(&mut runtime);
    focus(&mut runtime);
    let environment = StyleEnvironment::default();
    let context = SurfaceBuildContext::tight(
        &environment,
        LogicalSize::try_new(200.0, 40.0)
            .unwrap_or_else(|_| unreachable!("test surface is finite")),
    );
    let initial = runtime
        .publish_surface(&context)
        .unwrap_or_else(|error| panic!("initial surface publishes: {error:?}"));
    let start = runtime
        .start_composition(None)
        .unwrap_or_else(|error| panic!("focused editor starts composition: {error:?}"));
    runtime.pump(PumpBudget::new(8, usize::MAX, usize::MAX, usize::MAX));
    runtime
        .submit_composition_update(start.generation().clone(), "xy".to_owned(), None)
        .unwrap_or_else(|error| panic!("matching preedit update is queued: {error:?}"));
    runtime.pump(PumpBudget::new(8, usize::MAX, usize::MAX, usize::MAX));

    let staged = runtime
        .publish_surface(&context)
        .unwrap_or_else(|error| panic!("preedit surface publishes: {error:?}"));
    runtime.pump(PumpBudget::new(16, usize::MAX, usize::MAX, usize::MAX));
    assert_ne!(staged.paint_scene(), initial.paint_scene());
    assert_eq!(runtime.state().text, "ab");
    assert!(
        staged.semantic_publication().snapshot().nodes()[0]
            .editable()
            .is_none(),
        "transient preedit is withheld from durable semantic ranges"
    );
    let input_method = pending_service(&runtime, |request| {
        matches!(
            request,
            FrameworkServiceRequest::InputMethod {
                enabled: true,
                candidate_area: Some(_),
                composition: Some(_),
            }
        )
    });
    assert_eq!(
        input_method.binding().composition(),
        Some(start.generation())
    );

    runtime
        .submit_text(
            CommittedTextEvent::new("Z", None)
                .unwrap_or_else(|_| unreachable!("text is non-empty")),
        )
        .unwrap_or_else(|error| panic!("composition commit text is queued once: {error:?}"));
    runtime
        .submit_composition_end(start.generation().clone())
        .unwrap_or_else(|error| panic!("matching composition ends: {error:?}"));
    runtime.pump(PumpBudget::new(16, usize::MAX, usize::MAX, usize::MAX));
    assert_eq!(runtime.state().text, "abZ");
    assert_eq!(runtime.state().revision, 1);
    assert_eq!(runtime.status(), RuntimeStatus::Running);
}

#[test]
fn owner_removal_cancels_preedit_without_a_document_mutation() {
    let mut runtime = mounted();
    install_controlled_font(&mut runtime);
    focus(&mut runtime);
    let start = runtime
        .start_composition(None)
        .unwrap_or_else(|error| panic!("focused editor starts composition: {error:?}"));
    runtime.pump(PumpBudget::new(8, usize::MAX, usize::MAX, usize::MAX));
    runtime
        .submit_composition_update(start.generation().clone(), "xy".to_owned(), None)
        .unwrap_or_else(|error| panic!("matching preedit update is queued: {error:?}"));
    runtime.pump(PumpBudget::new(8, usize::MAX, usize::MAX, usize::MAX));

    runtime
        .submit_action(Action::RemoveEditor)
        .unwrap_or_else(|_| unreachable!("editor removal is queued"));
    runtime.pump(PumpBudget::new(16, usize::MAX, usize::MAX, usize::MAX));
    assert!(!runtime.state().editor_visible);
    assert_eq!(runtime.state().text, "ab");
    assert_eq!(runtime.state().revision, 0);
    assert_eq!(runtime.__editing_session_counts_for_test(), (0, 0));
    assert_eq!(runtime.status(), RuntimeStatus::Running);
}

#[test]
fn pending_capacity_accounts_for_already_queued_committed_text_without_partial_work() {
    let config =
        RuntimeConfig::default().with_limits(RuntimeLimits::default().with_pending_edits(1));
    let mut runtime = AppRuntime::<App>::mount_with_config(mounted().state().clone(), config);
    focus(&mut runtime);
    runtime
        .submit_text(
            CommittedTextEvent::new("x", None)
                .unwrap_or_else(|_| unreachable!("text is non-empty")),
        )
        .unwrap_or_else(|_| unreachable!("first reserved edit fits"));
    let rejected = runtime
        .submit_text(
            CommittedTextEvent::new("y", None)
                .unwrap_or_else(|_| unreachable!("text is non-empty")),
        )
        .expect_err("second queued edit exceeds the bounded projection");
    assert_eq!(rejected.kind(), SubmitTextErrorKind::EditingCapacity);
    assert_eq!(rejected.event().text(), "y");
    assert_eq!(runtime.state().text, "ab");
    runtime.pump(PumpBudget::new(8, usize::MAX, usize::MAX, usize::MAX));
    assert_eq!(runtime.state().text, "abx");
    assert_eq!(runtime.status(), RuntimeStatus::Running);
}

#[test]
fn request_exhaustion_accounts_for_queued_reservations_and_is_recoverable() {
    let mut runtime = mounted();
    focus(&mut runtime);
    runtime.__seed_next_edit_request_for_test(Some(u64::MAX));
    runtime
        .submit_text(
            CommittedTextEvent::new("x", None)
                .unwrap_or_else(|_| unreachable!("text is non-empty")),
        )
        .unwrap_or_else(|_| unreachable!("last request identity may be reserved once"));
    let rejected = runtime
        .submit_text(
            CommittedTextEvent::new("y", None)
                .unwrap_or_else(|_| unreachable!("text is non-empty")),
        )
        .expect_err("non-wrapping request space is exhausted");
    assert_eq!(rejected.kind(), SubmitTextErrorKind::EditRequestExhausted);
    assert_eq!(runtime.state().text, "ab");
    runtime.pump(PumpBudget::new(8, usize::MAX, usize::MAX, usize::MAX));
    assert_eq!(runtime.state().text, "abx");
    assert_eq!(runtime.status(), RuntimeStatus::Running);
}

#[test]
fn secret_edit_and_preedit_payloads_stay_absent_from_full_capture_trace_and_replay() {
    const DOCUMENT_SECRET: &str = "vault-document-4f914b";
    const PREEDIT_SECRET: &str = "preedit-secret-78c2a1";
    const COMMIT_SECRET: &str = "commit-secret-0d681e";

    let state = Document {
        text: DOCUMENT_SECRET.to_owned(),
        selection: DOCUMENT_SECRET.len(),
        sensitivity: TextSensitivity::Secret,
        ..mounted().state().clone()
    };
    let config = RuntimeConfig::default().with_trace_config(
        TraceConfig::new(512).with_payload_capture(TracePayloadCapture::FullText),
    );
    let mut runtime = AppRuntime::<App>::mount_with_config(state, config);
    install_controlled_font(&mut runtime);
    focus(&mut runtime);
    let start = runtime
        .start_composition(None)
        .unwrap_or_else(|error| panic!("secret editor starts composition: {error:?}"));
    runtime.pump(PumpBudget::new(8, usize::MAX, usize::MAX, usize::MAX));
    runtime
        .submit_composition_update(start.generation().clone(), PREEDIT_SECRET.to_owned(), None)
        .unwrap_or_else(|error| panic!("secret preedit is queued: {error:?}"));
    runtime.pump(PumpBudget::new(8, usize::MAX, usize::MAX, usize::MAX));
    runtime
        .cancel_composition(start.generation().clone())
        .unwrap_or_else(|error| panic!("secret preedit is cancelled: {error:?}"));
    runtime.pump(PumpBudget::new(8, usize::MAX, usize::MAX, usize::MAX));
    runtime
        .submit_text(
            CommittedTextEvent::new(COMMIT_SECRET, None)
                .unwrap_or_else(|_| unreachable!("text is non-empty")),
        )
        .unwrap_or_else(|error| panic!("secret committed text is queued: {error:?}"));
    runtime.pump(PumpBudget::new(8, usize::MAX, usize::MAX, usize::MAX));

    let environment = StyleEnvironment::default();
    let publication = runtime
        .publish_surface(&SurfaceBuildContext::tight(
            &environment,
            LogicalSize::try_new(320.0, 40.0)
                .unwrap_or_else(|_| unreachable!("test surface is finite")),
        ))
        .unwrap_or_else(|error| panic!("secret semantic surface publishes: {error:?}"));
    let semantic = &publication.semantic_publication().snapshot().nodes()[0];
    let editable = semantic
        .editable()
        .unwrap_or_else(|| unreachable!("secret editor retains checked range facts"));
    assert_eq!(editable.sensitivity(), TextSensitivity::Secret);
    assert_eq!(editable.value(), None);
    assert!(!semantic.supported_actions().contains(&SemanticAction::Copy));
    assert!(!semantic.supported_actions().contains(&SemanticAction::Cut));

    let jsonl = runtime.trace().export_jsonl();
    assert!(jsonl.contains("\"category\":\"transactional_edit\""));
    assert!(jsonl.contains("\"name\":\"edit_resolution_verified\""));
    assert!(jsonl.contains("\"outcome\":\"accepted\""));
    assert!(jsonl.contains("\"request\":\"e-"));
    assert!(jsonl.contains("\"session\":\"es-"));
    for secret in [DOCUMENT_SECRET, PREEDIT_SECRET, COMMIT_SECRET] {
        assert!(!jsonl.contains(secret), "secret literal leaked into trace");
    }
    TraceReplay::parse_jsonl(&jsonl)
        .unwrap_or_else(|error| panic!("redacted edit trace remains replayable: {error:?}"));
}
