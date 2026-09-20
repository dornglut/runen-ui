#![allow(refining_impl_trait)]

use runenui_core::{
    CommandOrigin, CommittedTextEvent, EditIntent, EditResolution, EditableContribution,
    EditingSessionPolicy, Element, NoHostProtocol, SemanticCommand, TextAffinity, TextDocumentId,
    TextDocumentRevision, TextDocumentSnapshot, TextPosition, TextSelection, TextSensitivity,
    UiApp, UpdateOutput, Widget, WidgetActivation, WidgetMeasure, WidgetMeasureInput,
    WidgetTextInput,
};
use runenui_runtime::{AppRuntime, PumpBudget, RuntimeStatus};

#[derive(Clone)]
struct State {
    text: String,
    revision: u64,
}

enum Action {
    Edit(EditIntent),
}

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
        let position = TextPosition::new(
            self.snapshot,
            &self.text,
            self.text.len(),
            TextAffinity::Upstream,
        )
        .ok()?;
        EditableContribution::new(
            self.snapshot,
            self.text.clone(),
            TextSelection::collapsed(position),
            TextSensitivity::Public,
            false,
            false,
            EditingSessionPolicy::PreserveExact,
            Action::Edit,
        )
        .ok()
    }

    fn measure(&self, (): &Self::State, _: WidgetMeasureInput) -> WidgetMeasure {
        WidgetMeasure::Text {
            content: self.text.clone(),
        }
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
        Action::Edit(intent): Self::Action,
    ) -> UpdateOutput<Self::Action, Self::HostProtocol> {
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
    let mut runtime = AppRuntime::<ExternalEditingApp>::mount(State {
        text: "public".to_owned(),
        revision: 0,
    });
    let owner = runtime.index().nodes()[0].id().clone();
    runtime
        .submit_command(
            owner,
            SemanticCommand::RequestFocus,
            CommandOrigin::programmatic(),
        )
        .unwrap_or_else(|_| unreachable!("external editor accepts focus"));
    runtime.pump(PumpBudget::new(8, usize::MAX, usize::MAX, usize::MAX));
    runtime
        .submit_text(
            CommittedTextEvent::new(" API", None)
                .unwrap_or_else(|_| unreachable!("fixture text is non-empty")),
        )
        .unwrap_or_else(|_| unreachable!("external editor accepts committed text"));
    assert_eq!(runtime.state().text, "public");
    runtime.pump(PumpBudget::new(8, usize::MAX, usize::MAX, usize::MAX));
    assert_eq!(runtime.state().text, "public API");
    assert_eq!(runtime.state().revision, 1);
    assert_eq!(runtime.status(), RuntimeStatus::Running);
}
