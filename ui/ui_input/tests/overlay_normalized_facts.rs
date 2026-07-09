use ui_input::{
    FocusInputFact, FocusTargetId, NormalizedInputFact, NormalizedInputSample, PointerEventKind,
    PointerInputFact,
};
use ui_math::UiPoint;

#[test]
fn overlay_proof_uses_existing_normalized_input_facts_without_runtime_semantics() {
    let sample = NormalizedInputSample::new("sample.overlay.open")
        .with_fact(NormalizedInputFact::Pointer(PointerInputFact::new(
            PointerEventKind::Down,
            UiPoint::new(24.0, 32.0),
        )))
        .with_fact(NormalizedInputFact::Focus(FocusInputFact::target(
            FocusTargetId(42),
        )));

    assert_eq!(sample.fact_kinds(), vec!["pointer", "focus"]);
}
