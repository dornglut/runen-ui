//! A third-party widget must be able to compose the same M10 editing and M8
//! shaping protocols without a second application or editing registry.
use runenui_core::{
    EditIntent, EditableContribution, EditingSessionPolicy, Element, SemanticAction,
    SemanticContribution, SemanticContributionContext, SemanticEditable, SemanticEditableMode,
    SemanticNodeContribution, SemanticRole, SemanticState, TextAffinity, TextDocumentId,
    TextDocumentRevision, TextDocumentSnapshot, TextLeafMeasure, TextLeafWrap, TextNewlinePolicy,
    TextPosition, TextSelection, TextSensitivity, View, Widget, WidgetMeasure, WidgetMeasureInput,
    WidgetTextInput, text_field,
};

struct DownstreamField {
    snapshot: TextDocumentSnapshot,
    source: String,
    selection: TextSelection,
    mode: SemanticEditableMode,
}

impl Widget<EditIntent> for DownstreamField {
    type State = ();

    fn create_state(&self) -> Self::State {}

    fn text_input(&self, _: &Self::State) -> WidgetTextInput {
        WidgetTextInput::new(true, true)
    }

    fn editable(&self, _: &Self::State) -> Option<EditableContribution<EditIntent>> {
        let newline = if self.mode == SemanticEditableMode::SingleLine {
            TextNewlinePolicy::ReplaceWithSpace
        } else {
            TextNewlinePolicy::Preserve
        };
        EditableContribution::new(
            self.snapshot,
            &self.source,
            self.selection,
            TextSensitivity::Public,
            false,
            false,
            EditingSessionPolicy::PreserveExact,
            |intent| intent,
        )
        .ok()
        .map(|contribution| contribution.with_newline_policy(newline))
    }

    fn measure(&self, _: &Self::State, _: WidgetMeasureInput) -> WidgetMeasure {
        let wrap = if self.mode == SemanticEditableMode::SingleLine {
            TextLeafWrap::NoWrap
        } else {
            TextLeafWrap::Wrap
        };
        WidgetMeasure::Text(TextLeafMeasure::new(&self.source).with_wrap_mode(wrap))
    }

    fn semantics(&self, _: &Self::State, _: SemanticContributionContext) -> SemanticContribution {
        let Some(editable) = SemanticEditable::new(
            self.snapshot,
            &self.source,
            self.selection,
            TextSensitivity::Public,
            false,
        ) else {
            return SemanticContribution::empty();
        };
        let node = SemanticNodeContribution::primary(SemanticRole::EditableText)
            .with_state(
                SemanticState::ENABLED
                    .with_disabled(false)
                    .with_read_only(false)
                    .with_required(false),
            )
            .with_editable(editable)
            .with_editable_mode(self.mode)
            .with_action(SemanticAction::MoveBackward)
            .with_action(SemanticAction::MoveForward)
            .with_action(SemanticAction::ExtendBackward)
            .with_action(SemanticAction::ExtendForward)
            .with_action(SemanticAction::SelectAll)
            .with_action(SemanticAction::SetSelection)
            .with_action(SemanticAction::DeleteBackward)
            .with_action(SemanticAction::DeleteForward)
            .with_action(SemanticAction::Undo)
            .with_action(SemanticAction::Redo)
            .with_action(SemanticAction::ReplaceSelection);
        SemanticContribution::single(node)
    }
}

#[test]
fn downstream_text_field_matches_standard_m10_and_m8_contracts() {
    let snapshot = TextDocumentSnapshot::new(
        TextDocumentId::new(105),
        TextDocumentRevision::new(7),
    );
    let source = "abc";
    let caret = TextPosition::new(snapshot, source, source.len(), TextAffinity::Upstream)
        .unwrap_or_else(|_| unreachable!("fixture caret is valid"));
    let selection = TextSelection::collapsed(caret);
    for mode in [
        SemanticEditableMode::SingleLine,
        SemanticEditableMode::Multiline,
    ] {
        let downstream: Element<EditIntent> = Element::new(DownstreamField {
            snapshot,
            source: source.to_owned(),
            selection,
            mode,
        })
        .focusable(true);
        let standard = text_field(snapshot, source, selection, mode, |intent| intent)
            .unwrap_or_else(|_| unreachable!("fixture is a valid field"))
            .into_element();
        let (_, _, _, _, _, _, _, _, downstream, _) =
            downstream.into_runtime_parts().into_parts();
        let (_, _, _, _, _, _, _, _, standard, _) =
            standard.into_runtime_parts().into_parts();
        let downstream_state = downstream.create_state();
        let standard_state = standard.create_state();
        assert_eq!(
            downstream.text_input(&downstream_state),
            standard.text_input(&standard_state),
        );
        let a = downstream
            .editable(&downstream_state)
            .unwrap_or_else(|_| unreachable!("downstream state matches"))
            .unwrap_or_else(|| unreachable!("downstream composes M10"));
        let b = standard
            .editable(&standard_state)
            .unwrap_or_else(|_| unreachable!("standard state matches"))
            .unwrap_or_else(|| unreachable!("standard composes M10"));
        assert_eq!(a.snapshot(), b.snapshot());
        assert_eq!(a.text(), b.text());
        assert_eq!(a.initial_selection(), b.initial_selection());
        assert_eq!(a.newline_policy(), b.newline_policy());
        assert_eq!(a.session_policy(), b.session_policy());
        let input = WidgetMeasureInput::new(
            None,
            None,
            runenui_core::WidgetAvailableSpace::MaxContent,
            runenui_core::WidgetAvailableSpace::MaxContent,
        );
        assert_eq!(
            downstream
                .measure(&downstream_state, input)
                .unwrap_or_else(|_| unreachable!("downstream measurement is valid")),
            standard
                .measure(&standard_state, input)
                .unwrap_or_else(|_| unreachable!("standard measurement is valid")),
        );
        assert_eq!(
            downstream
                .semantics(&downstream_state, SemanticContributionContext::default())
                .unwrap_or_else(|_| unreachable!("downstream semantics are valid")),
            standard
                .semantics(&standard_state, SemanticContributionContext::default())
                .unwrap_or_else(|_| unreachable!("standard semantics are valid")),
        );
    }
}
