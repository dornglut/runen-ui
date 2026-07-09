use ui_input::{
    NormalizedInputFact, NormalizedInputSample, TextCompositionFact, TextEditFact, TextPosition,
    TextRange, TextSelectionFact,
};

#[test]
fn normalized_input_sample_records_text_editing_fact_families() {
    let sample = NormalizedInputSample::new("runenwerk.ui.input.text-editing")
        .with_fact(NormalizedInputFact::TextEdit(TextEditFact::insert_text(
            "abc",
        )))
        .with_fact(NormalizedInputFact::TextComposition(
            TextCompositionFact::start("e")
                .with_range(TextRange::collapsed(TextPosition::grapheme(3))),
        ))
        .with_fact(NormalizedInputFact::TextSelection(
            TextSelectionFact::range(TextRange::new(
                TextPosition::grapheme(0),
                TextPosition::grapheme(3),
            )),
        ));

    assert_eq!(
        sample.fact_kinds(),
        vec!["text-edit", "text-composition", "text-selection"]
    );
}

#[test]
fn text_positions_remain_domain_shaped_not_byte_offsets() {
    let range = TextRange::collapsed(TextPosition::grapheme(2));

    assert!(range.is_collapsed());
    assert_eq!(range.anchor.unit.as_str(), "grapheme");
}
