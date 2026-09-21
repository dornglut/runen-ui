#![allow(refining_impl_trait)]

use runenui_core::{
    ClipboardClassification, ClipboardText, ClipboardWritePurpose, CommittedTextEvent,
    CompositionRange, EdgeInsets, EditIntent, EditResolution, EditableContribution,
    EditingSessionPolicy, Effects, Element, ElementId, FontFamilyName, FrameworkServiceFailure,
    FrameworkServiceRequest, FrameworkServiceResponse, GenericFontFamily, LogicalLength,
    NoHostProtocol, PaintPrimitive, SceneShape, SemanticAction, SemanticCommand,
    SemanticContribution, SemanticContributionContext, SemanticEditable, SemanticNodeContribution,
    SemanticRole, SemanticState, TextAffinity, TextDocumentId, TextDocumentRevision,
    TextDocumentSnapshot, TextPosition, TextSelection, TextSensitivity, UiApp, UpdateOutput, View,
    Widget, WidgetActivation, WidgetMeasure, WidgetMeasureInput, WidgetTextInput,
};
use runenui_runtime::PumpBudget;
use runenui_testing::{SettleBudget, SettleOutcome, TestHarness};
use std::num::NonZeroUsize;

const CANTARELL: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../crates/runenui_text/tests/fixtures/Cantarell-Regular.ttf"
));

fn register_fixture_font(harness: &mut TestHarness<ExternalClipboardApp>) {
    harness
        .register_text_font_bytes(CANTARELL.to_vec())
        .unwrap_or_else(|_| unreachable!("controlled downstream font is registerable"));
    let family = FontFamilyName::new("Cantarell")
        .unwrap_or_else(|_| unreachable!("controlled downstream font family is valid"));
    harness
        .set_text_generic_family_mapping(GenericFontFamily::SansSerif, &[family])
        .unwrap_or_else(|_| unreachable!("controlled downstream family mapping is valid"));
}

#[derive(Clone)]
struct State {
    text: String,
    revision: u64,
    ordinary_actions: usize,
    equal_action_value_observed: bool,
}

enum Action {
    Edit(Box<EditIntent>),
    Ordinary,
}

impl PartialEq for Action {
    fn eq(&self, _other: &Self) -> bool {
        true
    }
}

impl Eq for Action {}

#[derive(Debug)]
struct ExternalEditor {
    snapshot: TextDocumentSnapshot,
    text: String,
}

impl Widget<Action> for ExternalEditor {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn activation(&self, (): &Self::State) -> WidgetActivation {
        WidgetActivation::NONE
    }

    fn text_input(&self, (): &Self::State) -> WidgetTextInput {
        WidgetTextInput::new(true, true)
    }

    fn editable(&self, (): &Self::State) -> Option<EditableContribution<Action>> {
        let selection = self.selection()?;
        EditableContribution::new(
            self.snapshot,
            self.text.clone(),
            selection,
            TextSensitivity::Public,
            false,
            false,
            EditingSessionPolicy::PreserveExact,
            |intent| Action::Edit(Box::new(intent)),
        )
        .ok()
    }

    fn semantics(&self, (): &Self::State, _: SemanticContributionContext) -> SemanticContribution {
        let Some(selection) = self.selection() else {
            return SemanticContribution::empty();
        };
        let Some(editable) = SemanticEditable::new(
            self.snapshot,
            &self.text,
            selection,
            TextSensitivity::Public,
            false,
        ) else {
            return SemanticContribution::empty();
        };
        SemanticContribution::single(
            SemanticNodeContribution::primary(SemanticRole::EditableText)
                .with_state(SemanticState::ENABLED)
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

    fn measure(&self, (): &Self::State, _: WidgetMeasureInput) -> WidgetMeasure {
        WidgetMeasure::Text {
            content: self.text.clone(),
        }
    }
}

impl ExternalEditor {
    fn selection(&self) -> Option<TextSelection> {
        let position = TextPosition::new(
            self.snapshot,
            &self.text,
            self.text.len(),
            TextAffinity::Upstream,
        )
        .ok()?;
        Some(TextSelection::collapsed(position))
    }
}

struct ExternalEditingApp;

impl UiApp for ExternalEditingApp {
    type State = State;
    type Action = Action;
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> Element<Self::Action> {
        Element::new(ExternalEditor {
            snapshot: TextDocumentSnapshot::new(
                TextDocumentId::new(71),
                TextDocumentRevision::new(state.revision),
            ),
            text: state.text.clone(),
        })
        .id("external.editor")
        .key("external.editor")
        .focusable(true)
    }

    fn update(
        state: &mut Self::State,
        action: Self::Action,
    ) -> UpdateOutput<Self::Action, Self::HostProtocol> {
        let Action::Edit(intent) = action else {
            state.ordinary_actions += 1;
            return UpdateOutput::effects(Effects::none());
        };
        state.equal_action_value_observed |= Action::Ordinary == Action::Edit(intent.clone());
        let request = intent.request().clone();
        let range = intent.replacement();
        state
            .text
            .replace_range(range.start()..range.end(), intent.replacement_text());
        state.revision += 1;
        UpdateOutput::edit(EditResolution::accepted(
            request,
            TextDocumentSnapshot::new(
                TextDocumentId::new(71),
                TextDocumentRevision::new(state.revision),
            ),
        ))
    }
}

#[test]
fn downstream_widget_uses_only_public_transactional_editing_contracts() {
    let mut harness = TestHarness::<ExternalEditingApp>::mount(State {
        text: "public".to_owned(),
        revision: 0,
        ordinary_actions: 0,
        equal_action_value_observed: false,
    });
    harness
        .submit_automation_command(
            ElementId::new("external.editor")
                .unwrap_or_else(|_| unreachable!("fixture element ID is valid")),
            SemanticCommand::RequestFocus,
        )
        .unwrap_or_else(|_| unreachable!("external editor accepts focus"));
    let settle_budget = SettleBudget::new(
        NonZeroUsize::new(8).unwrap_or(NonZeroUsize::MIN),
        PumpBudget::new(8, usize::MAX, usize::MAX, usize::MAX),
    );
    assert_eq!(
        harness.run_until_idle(settle_budget).outcome(),
        SettleOutcome::Idle
    );
    harness
        .publish()
        .unwrap_or_else(|_| unreachable!("external editable widget publishes"));
    harness
        .submit_action(Action::Ordinary)
        .unwrap_or_else(|_| unreachable!("ordinary action is admitted"));
    harness
        .submit_text(
            CommittedTextEvent::new(" A", None)
                .unwrap_or_else(|_| unreachable!("fixture text is non-empty")),
        )
        .unwrap_or_else(|_| unreachable!("external editor accepts committed text"));
    harness
        .submit_text(
            CommittedTextEvent::new(" B", None)
                .unwrap_or_else(|_| unreachable!("fixture text is non-empty")),
        )
        .unwrap_or_else(|_| unreachable!("external editor accepts a queued second commit"));
    assert_eq!(harness.state().text, "public");
    assert_eq!(
        harness.run_until_idle(settle_budget).outcome(),
        SettleOutcome::Idle
    );
    assert_eq!(harness.state().text, "public A B");
    assert_eq!(harness.state().revision, 2);
    assert_eq!(harness.state().ordinary_actions, 1);
    assert!(harness.state().equal_action_value_observed);
    assert!(harness.publish().is_ok());

    assert!(harness.trace_replay().is_ok());
}

struct ClipboardState {
    text: String,
    visible: bool,
}

enum ClipboardAction {
    Hide,
}

#[derive(Debug)]
struct ExternalClipboardEditor {
    snapshot: TextDocumentSnapshot,
    text: String,
}

impl Widget<ClipboardAction> for ExternalClipboardEditor {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn activation(&self, (): &Self::State) -> WidgetActivation {
        WidgetActivation::NONE
    }

    fn text_input(&self, (): &Self::State) -> WidgetTextInput {
        WidgetTextInput::new(true, true)
    }

    fn editable(&self, (): &Self::State) -> Option<EditableContribution<ClipboardAction>> {
        let selection = self.selection()?;
        EditableContribution::new(
            self.snapshot,
            self.text.clone(),
            selection,
            TextSensitivity::Public,
            false,
            false,
            EditingSessionPolicy::PreserveExact,
            |_| ClipboardAction::Hide,
        )
        .ok()
    }

    fn semantics(&self, (): &Self::State, _: SemanticContributionContext) -> SemanticContribution {
        let Some(selection) = self.selection() else {
            return SemanticContribution::empty();
        };
        let Some(editable) = SemanticEditable::new(
            self.snapshot,
            &self.text,
            selection,
            TextSensitivity::Public,
            false,
        ) else {
            return SemanticContribution::empty();
        };
        SemanticContribution::single(
            SemanticNodeContribution::primary(SemanticRole::EditableText)
                .with_state(SemanticState::ENABLED)
                .with_editable(editable)
                .with_action(SemanticAction::Copy)
                .with_action(SemanticAction::Paste),
        )
    }

    fn measure(&self, (): &Self::State, _: WidgetMeasureInput) -> WidgetMeasure {
        WidgetMeasure::Text {
            content: self.text.clone(),
        }
    }
}

impl ExternalClipboardEditor {
    fn selection(&self) -> Option<TextSelection> {
        let anchor =
            TextPosition::new(self.snapshot, &self.text, 0, TextAffinity::Downstream).ok()?;
        let active = TextPosition::new(
            self.snapshot,
            &self.text,
            self.text.len(),
            TextAffinity::Upstream,
        )
        .ok()?;
        TextSelection::new(anchor, active).ok()
    }
}

struct ExternalClipboardApp;

impl UiApp for ExternalClipboardApp {
    type State = ClipboardState;
    type Action = ClipboardAction;
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> Element<Self::Action> {
        if !state.visible {
            return runenui_core::text("external editor removed")
                .id("external.clipboard.placeholder")
                .key("external.clipboard.placeholder")
                .into_element();
        }
        Element::new(ExternalClipboardEditor {
            snapshot: TextDocumentSnapshot::new(
                TextDocumentId::new(72),
                TextDocumentRevision::new(0),
            ),
            text: state.text.clone(),
        })
        .id("external.clipboard")
        .key("external.clipboard")
        .padding(EdgeInsets::all(LogicalLength::from(12_u8)))
        .focusable(true)
    }

    fn update(
        state: &mut Self::State,
        action: Self::Action,
    ) -> impl runenui_core::IntoUpdateOutput<Self::Action, Self::HostProtocol> {
        match action {
            ClipboardAction::Hide => state.visible = false,
        }
        UpdateOutput::effects(Effects::none())
    }
}

#[test]
fn downstream_test_harness_completes_typed_fake_framework_services() {
    let mut harness = TestHarness::<ExternalClipboardApp>::mount(ClipboardState {
        text: "public clipboard text".to_owned(),
        visible: true,
    });
    let settle_budget = SettleBudget::new(
        NonZeroUsize::new(8).unwrap_or(NonZeroUsize::MIN),
        PumpBudget::new(8, usize::MAX, usize::MAX, usize::MAX),
    );
    harness
        .submit_automation_command(
            ElementId::new("external.clipboard")
                .unwrap_or_else(|_| unreachable!("fixture element ID is valid")),
            SemanticCommand::RequestFocus,
        )
        .unwrap_or_else(|_| unreachable!("external clipboard owner accepts focus"));
    assert_eq!(
        harness.run_until_idle(settle_budget).outcome(),
        SettleOutcome::Idle
    );
    harness
        .publish()
        .unwrap_or_else(|_| unreachable!("external clipboard owner publishes"));
    harness
        .submit_automation_command(
            ElementId::new("external.clipboard")
                .unwrap_or_else(|_| unreachable!("fixture element ID is valid")),
            SemanticCommand::Copy,
        )
        .unwrap_or_else(|_| unreachable!("public copy command is accepted"));
    assert_eq!(
        harness.run_until_idle(settle_budget).outcome(),
        SettleOutcome::Idle
    );

    let request = harness
        .pending_framework_services()
        .into_iter()
        .find(|service| {
            matches!(
                service.request(),
                FrameworkServiceRequest::ClipboardWriteText {
                    purpose: ClipboardWritePurpose::Copy,
                    ..
                }
            )
        })
        .unwrap_or_else(|| unreachable!("copy exposes a pending public service request"));
    let token = request.token();
    assert!(matches!(
        request.request(),
        FrameworkServiceRequest::ClipboardWriteText { text, .. }
            if text.as_ref() == "public clipboard text"
    ));
    assert!(!format!("{:?}", request.request()).contains("public clipboard text"));
    harness
        .complete_framework_service(&token, FrameworkServiceResponse::ClipboardWriteText(Ok(())))
        .unwrap_or_else(|_| unreachable!("fake clipboard host completion is accepted"));
    assert_eq!(
        harness.run_until_idle(settle_budget).outcome(),
        SettleOutcome::Idle
    );
    assert!(harness.pending_framework_services().iter().all(|service| {
        !matches!(
            service.request(),
            FrameworkServiceRequest::ClipboardWriteText { .. }
        )
    }));
    assert!(!harness.trace_jsonl().contains("public clipboard text"));
    assert!(harness.trace_replay().is_ok());
}

#[test]
fn downstream_publication_correlates_selection_preedit_caret_and_ime_geometry() {
    let mut harness = focused_clipboard_editor();
    assert_selection_and_ime_geometry(&mut harness);
    assert_preedit_paints_and_is_redacted(&mut harness);
}

fn focused_clipboard_editor() -> TestHarness<ExternalClipboardApp> {
    let settle_budget = new_settle_budget();
    let mut harness = TestHarness::<ExternalClipboardApp>::mount(ClipboardState {
        text: "public clipboard text".to_owned(),
        visible: true,
    });
    register_fixture_font(&mut harness);
    harness
        .submit_automation_command(
            ElementId::new("external.clipboard")
                .unwrap_or_else(|_| unreachable!("fixture element ID is valid")),
            SemanticCommand::RequestFocus,
        )
        .unwrap_or_else(|_| unreachable!("external clipboard owner accepts focus"));
    assert_eq!(
        harness.run_until_idle(settle_budget).outcome(),
        SettleOutcome::Idle
    );
    harness
        .publish()
        .unwrap_or_else(|_| unreachable!("selected external editor publishes"));
    harness
}

fn assert_selection_and_ime_geometry(harness: &mut TestHarness<ExternalClipboardApp>) {
    let settle_budget = new_settle_budget();
    assert!(
        harness
            .publication()
            .unwrap_or_else(|| unreachable!("publication is retained"))
            .paint_scene()
            .items()
            .iter()
            .any(|item| item.primitive().as_shaped_text_run().is_some())
    );

    let selection_scene = harness
        .publication()
        .unwrap_or_else(|| unreachable!("publication is retained"))
        .paint_scene();
    let selection_rects = selection_scene
        .items()
        .iter()
        .filter_map(|item| match item.primitive() {
            PaintPrimitive::Fill {
                shape: SceneShape::Rect(rect),
                ..
            } => Some(*rect),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert!(
        selection_rects.len() >= 2,
        "selection and active caret are painted"
    );
    let caret = selection_rects
        .iter()
        .find(|rect| rect.width().to_bits() == 1.0_f32.to_bits())
        .copied()
        .unwrap_or_else(|| unreachable!("one logical unit caret is painted"));
    assert!(
        selection_rects
            .iter()
            .any(|rect| rect.x() >= 12.0 && rect.y() >= 12.0)
    );

    harness.run_until_idle(settle_budget);
    let input_method_requests = harness.pending_framework_services();
    assert!(
        input_method_requests.iter().any(|service| {
            matches!(
                service.request(),
                FrameworkServiceRequest::InputMethod {
                    enabled: true,
                    candidate_area: Some(area),
                    ..
                } if area.x().to_bits() == caret.x().to_bits()
                    && area.y().to_bits() == caret.y().to_bits()
                    && area.height().to_bits() == caret.height().to_bits()
                    && area.width().to_bits() == 0.0_f32.to_bits()
            )
        }),
        "IME candidate geometry must be the same padded retained-layout caret: caret={caret:?}, requests={:?}",
        input_method_requests
            .iter()
            .map(runenui_runtime::FrameworkServiceRef::request)
            .collect::<Vec<_>>()
    );
}

fn assert_preedit_paints_and_is_redacted(harness: &mut TestHarness<ExternalClipboardApp>) {
    let settle_budget = new_settle_budget();
    let composition = harness
        .start_composition(None)
        .unwrap_or_else(|_| unreachable!("focused downstream editor starts composition"));
    let generation = composition.generation().clone();
    assert_eq!(
        harness.run_until_idle(settle_budget).outcome(),
        SettleOutcome::Idle
    );
    let preedit = "compose";
    let selection = CompositionRange::new(preedit, 2, 5)
        .unwrap_or_else(|_| unreachable!("fixture preedit range is valid"));
    harness
        .submit_composition_update(generation, preedit.to_owned(), Some(selection))
        .unwrap_or_else(|_| unreachable!("the public preedit update is admitted"));
    assert_eq!(
        harness.run_until_idle(settle_budget).outcome(),
        SettleOutcome::Idle
    );
    harness
        .publish()
        .unwrap_or_else(|_| unreachable!("correlated preedit publication commits"));
    let preedit_scene = harness
        .publication()
        .unwrap_or_else(|| unreachable!("preedit publication is retained"))
        .paint_scene();
    assert!(
        preedit_scene
            .items()
            .iter()
            .any(|item| item.primitive().as_shaped_text_run().is_some())
    );
    let preedit_fill_count = preedit_scene
        .items()
        .iter()
        .filter(|item| matches!(item.primitive(), PaintPrimitive::Fill { .. }))
        .count();
    assert!(
        preedit_fill_count >= 3,
        "preedit selection, underline, and active caret share the retained layout; fill_count={preedit_fill_count}, scene={preedit_scene:?}"
    );
    assert!(!harness.trace_jsonl().contains(preedit));
    assert!(harness.trace_replay().is_ok());
}

fn new_settle_budget() -> SettleBudget {
    SettleBudget::new(
        NonZeroUsize::new(8).unwrap_or(NonZeroUsize::MIN),
        PumpBudget::new(8, usize::MAX, usize::MAX, usize::MAX),
    )
}

#[test]
fn downstream_test_harness_rejects_a_stale_service_after_owner_removal() {
    let settle_budget = SettleBudget::new(
        NonZeroUsize::new(8).unwrap_or(NonZeroUsize::MIN),
        PumpBudget::new(8, usize::MAX, usize::MAX, usize::MAX),
    );
    let mut harness = TestHarness::<ExternalClipboardApp>::mount(ClipboardState {
        text: "sensitive host payload".to_owned(),
        visible: true,
    });
    harness
        .submit_automation_command(
            ElementId::new("external.clipboard")
                .unwrap_or_else(|_| unreachable!("fixture element ID is valid")),
            SemanticCommand::RequestFocus,
        )
        .unwrap_or_else(|_| unreachable!("external clipboard owner accepts focus"));
    assert_eq!(
        harness.run_until_idle(settle_budget).outcome(),
        SettleOutcome::Idle
    );
    harness
        .publish()
        .unwrap_or_else(|_| unreachable!("external clipboard owner publishes"));
    harness
        .submit_automation_command(
            ElementId::new("external.clipboard")
                .unwrap_or_else(|_| unreachable!("fixture element ID is valid")),
            SemanticCommand::Copy,
        )
        .unwrap_or_else(|_| unreachable!("public copy command is accepted"));
    assert_eq!(
        harness.run_until_idle(settle_budget).outcome(),
        SettleOutcome::Idle
    );
    let token = harness
        .pending_framework_services()
        .into_iter()
        .find(|service| {
            matches!(
                service.request(),
                FrameworkServiceRequest::ClipboardWriteText {
                    purpose: ClipboardWritePurpose::Copy,
                    ..
                }
            )
        })
        .unwrap_or_else(|| unreachable!("copy exposes a typed clipboard write"))
        .token();

    harness
        .submit_action(ClipboardAction::Hide)
        .unwrap_or_else(|_| unreachable!("application removes the current editable owner"));
    assert_eq!(
        harness.run_until_idle(settle_budget).outcome(),
        SettleOutcome::Idle
    );
    assert!(!harness.state().visible);
    assert!(matches!(
        harness.complete_framework_service(
            &token,
            FrameworkServiceResponse::ClipboardWriteText(Ok(())),
        ),
        Err(runenui_runtime::FrameworkServiceResponseError::Stale(_))
    ));
    assert!(!harness.trace_jsonl().contains("sensitive host payload"));
    assert!(harness.trace_replay().is_ok());
}

#[test]
fn downstream_test_harness_replays_paste_success_and_permission_failure() {
    let mut success = focused_editing_harness();
    submit_paste_response(
        &mut success,
        Ok(ClipboardText::new(" host", ClipboardClassification::Public)),
    );
    assert_eq!(success.state().text, "public");
    assert_eq!(
        success.run_until_idle(new_settle_budget()).outcome(),
        SettleOutcome::Idle
    );
    assert_eq!(success.state().text, "public host");
    assert_eq!(success.state().revision, 1);
    assert!(success.focus().focused_node().is_some());
    assert!(success.trace_replay().is_ok());

    let mut denied = focused_editing_harness();
    submit_paste_response(&mut denied, Err(FrameworkServiceFailure::PermissionDenied));
    assert_eq!(
        denied.run_until_idle(new_settle_budget()).outcome(),
        SettleOutcome::Idle
    );
    assert_eq!(denied.state().text, "public");
    assert_eq!(denied.state().revision, 0);
    assert!(
        !denied
            .trace_jsonl()
            .contains("permission denied by native platform")
    );
    assert!(denied.trace_replay().is_ok());
}

fn focused_editing_harness() -> TestHarness<ExternalEditingApp> {
    let mut harness = TestHarness::<ExternalEditingApp>::mount(State {
        text: "public".to_owned(),
        revision: 0,
        ordinary_actions: 0,
        equal_action_value_observed: false,
    });
    harness
        .submit_automation_command(
            ElementId::new("external.editor")
                .unwrap_or_else(|_| unreachable!("fixture element ID is valid")),
            SemanticCommand::RequestFocus,
        )
        .unwrap_or_else(|_| unreachable!("external editor accepts focus"));
    assert_eq!(
        harness.run_until_idle(new_settle_budget()).outcome(),
        SettleOutcome::Idle
    );
    harness
        .publish()
        .unwrap_or_else(|_| unreachable!("external editor publishes"));
    harness
}

fn submit_paste_response(
    harness: &mut TestHarness<ExternalEditingApp>,
    response: Result<ClipboardText, FrameworkServiceFailure>,
) {
    harness
        .submit_automation_command(
            ElementId::new("external.editor")
                .unwrap_or_else(|_| unreachable!("fixture element ID is valid")),
            SemanticCommand::Paste,
        )
        .unwrap_or_else(|_| unreachable!("paste is routed through the editable owner"));
    assert_eq!(
        harness.run_until_idle(new_settle_budget()).outcome(),
        SettleOutcome::Idle
    );
    let token = harness
        .pending_framework_services()
        .into_iter()
        .find(|service| {
            matches!(
                service.request(),
                FrameworkServiceRequest::ClipboardReadText { max_bytes } if *max_bytes > 0
            )
        })
        .unwrap_or_else(|| unreachable!("paste exposes a typed clipboard request"))
        .token();
    harness
        .complete_framework_service(
            &token,
            FrameworkServiceResponse::ClipboardReadText(response),
        )
        .unwrap_or_else(|_| unreachable!("typed clipboard result enters ordinary runtime"));
}
