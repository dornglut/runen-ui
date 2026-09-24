use std::{error::Error, sync::Arc, time::Instant};

use runenui_core::{
    __runtime::RuntimeNamespace, CompositionRange, FontFamily, FontFamilyName, GenericFontFamily,
    LogicalLength, TextDocumentId, TextDocumentRevision, TextDocumentSnapshot, TextRange,
    Typography,
};

use crate::{
    FontSourcePolicy, TextCaretMap, TextConstraints, TextLayoutState, TextParagraphStyle,
    TextPreeditProjection, TextRequest, TextSystem,
};

const CANTARELL: &[u8] = include_bytes!("../tests/fixtures/Cantarell-Regular.ttf");
const DEVANAGARI: &[u8] = include_bytes!("../tests/fixtures/RunenUIFixtureDevanagari-Regular.ttf");
const ARABIC: &[u8] = include_bytes!("../tests/fixtures/RunenUIFixtureArabic-Regular.ttf");
const SAMPLE_COUNT: usize = 20;

fn snapshot(revision: u64) -> TextDocumentSnapshot {
    TextDocumentSnapshot::new(
        TextDocumentId::new(266),
        TextDocumentRevision::new(revision),
    )
}

fn typography(family: &str) -> Result<Typography, Box<dyn Error>> {
    Ok(Typography::new(
        FontFamily::named(family)?,
        LogicalLength::new(20.0)?,
    ))
}

fn corpus_system() -> Result<TextSystem, Box<dyn Error>> {
    let mut system = TextSystem::new(FontSourcePolicy::BundledOnly);
    for font in [CANTARELL, DEVANAGARI, ARABIC] {
        assert!(system.register_font_bytes(font.to_vec())? > 0);
    }
    let families = [
        FontFamilyName::new("Cantarell")?,
        FontFamilyName::new("RunenUI Fixture Devanagari")?,
        FontFamilyName::new("RunenUI Fixture Arabic")?,
    ];
    assert!(system.set_generic_family_mapping(GenericFontFamily::SansSerif, &families)?);
    Ok(system)
}

fn map_for(
    system: &mut TextSystem,
    text: &str,
    typography: Typography,
    width: Option<f32>,
    revision: u64,
) -> Result<TextCaretMap, Box<dyn Error>> {
    let constraints = match width {
        Some(width) => TextConstraints::limited(LogicalLength::new(width)?),
        None => TextConstraints::unbounded(),
    };
    let mut state = TextLayoutState::new();
    system.layout_text(
        &mut state,
        &TextRequest::new(text, typography, constraints)
            .with_paragraph_style(TextParagraphStyle::default()),
    )?;
    Ok(state.caret_map(snapshot(revision))?)
}

fn assert_candidate_matches_oracle(label: &str, map: &TextCaretMap) {
    let (oracle, exhaustive_validations) = map.legal_byte_offsets_exhaustive_for_test();
    let (
        candidate,
        candidate_offsets,
        candidate_validations,
        _raw_cluster_count,
        artifact_max_end,
    ) = map.legal_byte_offsets_layout_candidate_for_test();
    assert_eq!(candidate, oracle, "candidate mismatch for {label}");
    assert!(candidate_offsets <= map.grapheme_boundary_count_for_test());
    assert!(candidate_validations <= exhaustive_validations);
    assert_eq!(
        artifact_max_end,
        map.display_text().len(),
        "retained artifact must cover the full displayed source for {label}"
    );
}

#[test]
fn layout_candidate_matches_exhaustive_oracle_across_controlled_corpus()
-> Result<(), Box<dyn Error>> {
    let mut system = corpus_system()?;

    for (label, text, family, width) in [
        ("empty", "", "Cantarell", None),
        ("ascii", "plain ascii text", "Cantarell", None),
        (
            "multiline",
            "line one\nline two\nline three",
            "Cantarell",
            None,
        ),
        (
            "wrapped_ascii",
            "wrapped words wrapped words wrapped words",
            "Cantarell",
            Some(90.0),
        ),
        (
            "combining_emoji",
            "e\u{301} office 👩\u{200d}💻",
            "Cantarell",
            None,
        ),
        (
            "devanagari_ligature",
            "क्षि",
            "RunenUI Fixture Devanagari",
            None,
        ),
        (
            "devanagari_wrapped",
            "कक्षा क्षि",
            "RunenUI Fixture Devanagari",
            Some(80.0),
        ),
        ("arabic", "سلام", "RunenUI Fixture Arabic", None),
        (
            "arabic_wrapped",
            "سلام عالم",
            "RunenUI Fixture Arabic",
            Some(80.0),
        ),
    ] {
        let map = map_for(&mut system, text, typography(family)?, width, 1)?;
        assert_candidate_matches_oracle(label, &map);
    }

    let mixed = map_for(
        &mut system,
        "abc אבג سلام xyz\nsecond line",
        Typography::default(),
        Some(120.0),
        2,
    )?;
    assert_candidate_matches_oracle("mixed_bidi", &mixed);
    Ok(())
}

#[test]
fn layout_candidate_matches_exhaustive_oracle_for_preedit_projection() -> Result<(), Box<dyn Error>>
{
    let document = "abXYZcd";
    let replacement = TextRange::new(snapshot(1), document, 2, 5)?;
    let namespace = RuntimeNamespace::__runtime_new();
    let generation = namespace.__runtime_composition_generation(9);
    let selected = CompositionRange::new("かな", 0, "か".len())?;
    let projection = Arc::new(TextPreeditProjection::new(
        snapshot(1),
        document,
        replacement,
        generation,
        "かな",
        Some(selected),
    )?);

    let mut system = corpus_system()?;
    let mut state = TextLayoutState::new();
    system.layout_text(
        &mut state,
        &TextRequest::new(
            projection.display_text(),
            typography("Cantarell")?,
            TextConstraints::unbounded(),
        ),
    )?;
    let map = state.preedit_caret_map(projection)?;
    assert_candidate_matches_oracle("preedit", &map);
    Ok(())
}

fn summarize(values: &mut [u128]) -> (u128, u128) {
    values.sort_unstable();
    let median = values[values.len() / 2];
    let p95_index = (values.len() * 95).div_ceil(100).saturating_sub(1);
    (median, values[p95_index])
}

#[test]
#[ignore = "opt-in issue 266 differential release profile; run with --ignored --nocapture"]
fn issue_266_legal_offset_candidate_profile() -> Result<(), Box<dyn Error>> {
    let fixture = "multiline responsiveness fixture — retained text layout\n";
    for (label, lines) in [
        ("40_lines", 40),
        ("400_lines", 400),
        ("4000_lines", 4000),
        ("16000_lines", 16000),
    ] {
        let text = fixture.repeat(lines);
        let mut oracle_times = Vec::with_capacity(SAMPLE_COUNT);
        let mut candidate_times = Vec::with_capacity(SAMPLE_COUNT);
        let mut oracle_validations = None;
        let mut candidate_offsets = None;
        let mut candidate_validations = None;
        let mut raw_cluster_count = None;
        let mut artifact_max_end = None;
        let mut legal_offsets = None;

        for sample in 0..SAMPLE_COUNT {
            let mut system = corpus_system()?;
            let map = map_for(
                &mut system,
                &text,
                typography("Cantarell")?,
                Some(760.0),
                sample as u64 + 1,
            )?;

            let started = Instant::now();
            let (oracle, current_oracle_validations) = map.legal_byte_offsets_exhaustive_for_test();
            oracle_times.push(started.elapsed().as_nanos());

            let started = Instant::now();
            let (
                candidate,
                current_candidate_offsets,
                current_candidate_validations,
                current_raw_cluster_count,
                current_artifact_max_end,
            ) = map.legal_byte_offsets_layout_candidate_for_test();
            candidate_times.push(started.elapsed().as_nanos());

            assert_eq!(candidate, oracle);
            oracle_validations = Some(current_oracle_validations);
            candidate_offsets = Some(current_candidate_offsets);
            candidate_validations = Some(current_candidate_validations);
            raw_cluster_count = Some(current_raw_cluster_count);
            artifact_max_end = Some(current_artifact_max_end);
            legal_offsets = Some(candidate.len());
        }

        let (oracle_median, oracle_p95) = summarize(&mut oracle_times);
        let (candidate_median, candidate_p95) = summarize(&mut candidate_times);
        eprintln!(
            "issue266_legal_offset_profile label={label} lines={lines} bytes={} samples={SAMPLE_COUNT} oracle_median_ns={oracle_median} oracle_p95_ns={oracle_p95} candidate_median_ns={candidate_median} candidate_p95_ns={candidate_p95} oracle_validations={} candidate_offsets={} candidate_validations={} raw_cluster_count={} artifact_max_end={} legal_offsets={}",
            text.len(),
            oracle_validations.unwrap_or_default(),
            candidate_offsets.unwrap_or_default(),
            candidate_validations.unwrap_or_default(),
            raw_cluster_count.unwrap_or_default(),
            artifact_max_end.unwrap_or_default(),
            legal_offsets.unwrap_or_default(),
        );
    }
    Ok(())
}
