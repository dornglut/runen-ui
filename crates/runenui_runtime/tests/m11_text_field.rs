#![allow(clippy::panic)]

use runenui_core::{
    CommandOrigin, CommittedTextEvent, EditIntent, EditResolution, Effects, IntoUpdateOutput,
    KeyLocation, KeyModifiers, KeyboardCompositionState, KeyboardEvent, KeyboardPhase, LogicalKey,
    NoHostProtocol, PhysicalKey, SemanticCommand, SemanticEditableMode, TextAffinity,
    TextDocumentId, TextDocumentRevision, TextDocumentSnapshot, TextPosition, TextSelection, UiApp,
    UpdateOutput, View,
};
use runenui_runtime::{AppRuntime, PumpBudget};

#[derive(Clone)]
struct FormState {
    text: String,
    revision: u64,
    selection: usize,
    mode: SemanticEditableMode,
    submits: usize,
}

enum Action {
    Edit(EditIntent),
    Submit,
}

struct FormApp;

impl UiApp for FormApp {
    type State = FormState;
    type Action = Action;
    type HostProtocol = NoHostProtocol;

    fn root(state: &Self::State) -> impl View<Self::Action> {
        let snapshot = snapshot(state.revision);
        let selection = TextPosition::new(
            snapshot,
            &state.text,
            state.selection,
            if state.selection == state.text.len() && !state.text.is_empty() {
                TextAffinity::Upstream
            } else {
                TextAffinity::Downstream
            },
        )
        .unwrap_or_else(|_| unreachable!("application selection remains valid"));
        runenui_core::text_field(
            snapshot,
            state.text.clone(),
            TextSelection::collapsed(selection),
            state.mode,
            Action::Edit,
        )
        .unwrap_or_else(|_| unreachable!("application field source remains valid"))
        .id("form.field")
        .placeholder("Type here")
        .on_submit(|| Action::Submit)
    }

    fn update(
        state: &mut Self::State,
        action: Self::Action,
    ) -> impl IntoUpdateOutput<Self::Action, Self::HostProtocol> {
        match action {
            Action::Edit(intent) => {
                let span = intent.replacement();
                state
                    .text
                    .replace_range(span.start()..span.end(), intent.replacement_text());
                state.selection = intent.proposed_selection().active();
                state.revision += 1;
                UpdateOutput::edit(EditResolution::accepted(
                    intent.request().clone(),
                    snapshot(state.revision),
                ))
            }
            Action::Submit => {
                state.submits += 1;
                UpdateOutput::effects(Effects::none())
            }
        }
    }
}

fn snapshot(revision: u64) -> TextDocumentSnapshot {
    TextDocumentSnapshot::new(TextDocumentId::new(24), TextDocumentRevision::new(revision))
}

fn app(mode: SemanticEditableMode) -> AppRuntime<FormApp> {
    AppRuntime::mount(FormState {
        text: "ab".to_owned(),
        revision: 0,
        selection: 2,
        mode,
        submits: 0,
    })
}

fn focus(runtime: &mut AppRuntime<FormApp>) {
    let owner = runtime.index().nodes()[0].id().clone();
    runtime
        .submit_command(
            owner,
            SemanticCommand::RequestFocus,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("field is focusable"));
    runtime.pump(PumpBudget::new(16, usize::MAX, usize::MAX, usize::MAX));
}

fn commit(runtime: &mut AppRuntime<FormApp>, text: &str) {
    runtime
        .submit_text(
            CommittedTextEvent::new(text, None)
                .unwrap_or_else(|_| unreachable!("fixture committed text is nonempty")),
        )
        .unwrap_or_else(|_| unreachable!("active field admits text"));
    runtime.pump(PumpBudget::new(16, usize::MAX, usize::MAX, usize::MAX));
}

#[test]
fn public_single_line_edit_normalizes_m10_text_and_enter_submits_without_mutation() {
    let mut runtime = app(SemanticEditableMode::SingleLine);
    focus(&mut runtime);
    commit(&mut runtime, "X\r\nY\nZ\rQ");
    assert_eq!(runtime.state().text, "abX Y Z Q");
    assert_eq!(runtime.state().revision, 1);
    assert_eq!(runtime.state().selection, runtime.state().text.len());

    runtime
        .submit_keyboard(KeyboardEvent::new(
            KeyboardPhase::Down,
            PhysicalKey::Code("Enter".to_owned()),
            LogicalKey::Enter,
            KeyModifiers::NONE,
            false,
            KeyLocation::Standard,
            KeyboardCompositionState::Inactive,
            None,
        ))
        .unwrap_or_else(|_| unreachable!("Enter is routed"));
    runtime.pump(PumpBudget::new(16, usize::MAX, usize::MAX, usize::MAX));
    assert_eq!(runtime.state().submits, 1);
    assert_eq!(runtime.state().revision, 1);
    assert_eq!(runtime.state().text, "abX Y Z Q");
}

#[test]
fn public_multiline_field_preserves_committed_newlines_through_m10() {
    let mut runtime = app(SemanticEditableMode::Multiline);
    focus(&mut runtime);
    commit(&mut runtime, "X\nY");
    assert_eq!(runtime.state().text, "abX\nY");
    assert_eq!(runtime.state().revision, 1);
    assert_eq!(runtime.state().submits, 0);
}
