use ui_input::{
    FocusDirection, FocusInputFact, Key, KeyState, KeyboardInputFact, NormalizedInputFact,
    NormalizedInputSample, PointerButton, PointerEventKind, PointerInputFact, SemanticInputFact,
    SemanticInputSource, TextIntentFact, UiSemanticAction,
};
use ui_math::UiPoint;

#[test]
fn normalized_input_sample_records_pointer_keyboard_focus_semantic_and_text_intent_facts() {
    let sample = NormalizedInputSample::new("runenwerk.ui.input.normalized.sample")
        .with_fact(NormalizedInputFact::Pointer(
            PointerInputFact::new(PointerEventKind::Down, UiPoint::new(10.0, 12.0))
                .with_button(PointerButton::Primary)
                .with_click_count(1),
        ))
        .with_fact(NormalizedInputFact::Keyboard(KeyboardInputFact::new(
            Key::Enter,
            KeyState::Pressed,
        )))
        .with_fact(NormalizedInputFact::Focus(FocusInputFact::traversal(
            FocusDirection::Next,
        )))
        .with_fact(NormalizedInputFact::Semantic(SemanticInputFact::new(
            ui_input::SemanticActionEvent::new(
                SemanticInputSource::Keyboard,
                UiSemanticAction::Activate,
            ),
        )))
        .with_fact(NormalizedInputFact::TextIntent(
            TextIntentFact::insert_text("probe"),
        ));

    assert_eq!(
        sample.fact_kinds(),
        vec!["pointer", "keyboard", "focus", "semantic", "text-intent"]
    );
}
