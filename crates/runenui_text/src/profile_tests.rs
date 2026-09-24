use std::{sync::Arc, time::Instant};

use runenui_core::{
    __runtime::RuntimeNamespace, CompositionRange, FontFamily, LogicalLength, TextDocumentId,
    TextDocumentRevision, TextDocumentSnapshot, TextRange, Typography,
};

use crate::{
    FontSourcePolicy, TextConstraints, TextLayoutDecision, TextLayoutState, TextPreeditProjection,
    TextRequest, TextSystem,
    test_profile::{self, TextPhaseProfile},
};

const CANTARELL: &[u8] = include_bytes!("../tests/fixtures/Cantarell-Regular.ttf");
const SAMPLE_COUNT: usize = 20;
type TimingField = (&'static str, fn(&TextPhaseProfile) -> u128);
type CountField = (&'static str, fn(&TextPhaseProfile) -> usize);

fn typography() -> Typography {
    Typography::new(
        FontFamily::named("Cantarell")
            .unwrap_or_else(|_| unreachable!("controlled family name is valid")),
        LogicalLength::new(16.0).unwrap_or_else(|_| unreachable!("controlled size is finite")),
    )
}

fn request(text: String) -> TextRequest {
    TextRequest::new(
        text,
        typography(),
        TextConstraints::limited(
            LogicalLength::new(760.0).unwrap_or_else(|_| unreachable!("fixture width is finite")),
        ),
    )
}

const fn snapshot(revision: u64) -> TextDocumentSnapshot {
    TextDocumentSnapshot::new(
        TextDocumentId::new(263),
        TextDocumentRevision::new(revision),
    )
}

fn system() -> TextSystem {
    let mut system = TextSystem::new(FontSourcePolicy::BundledOnly);
    assert!(
        system.register_font_bytes(CANTARELL.to_vec()).is_ok(),
        "controlled Cantarell fixture registers"
    );
    system
}

fn summarize(values: &mut [u128]) -> (u128, u128) {
    values.sort_unstable();
    let median = values[values.len() / 2];
    let p95_index = (values.len() * 95).div_ceil(100).saturating_sub(1);
    (median, values[p95_index])
}

fn report_ns(label: &str, profiles: &[TextPhaseProfile], field: fn(&TextPhaseProfile) -> u128) {
    let mut values = profiles.iter().map(field).collect::<Vec<_>>();
    let (median, p95) = summarize(&mut values);
    eprintln!(
        "issue263_text_profile label={label} n={} median_ns={median} p95_ns={p95}",
        profiles.len()
    );
}

fn report_count(label: &str, profiles: &[TextPhaseProfile], field: fn(&TextPhaseProfile) -> usize) {
    let mut values = profiles.iter().map(field);
    let first = values
        .next()
        .unwrap_or_else(|| unreachable!("profile sample set is non-empty"));
    let (mut minimum, mut maximum) = (first, first);
    for value in values {
        minimum = minimum.min(value);
        maximum = maximum.max(value);
    }
    eprintln!(
        "issue263_text_profile_count label={label} n={} min={minimum} max={maximum}",
        profiles.len()
    );
}

fn report(label: &str, totals: &mut [u128], profiles: &[TextPhaseProfile]) {
    let mut remaining = totals
        .iter()
        .zip(profiles)
        .map(|(total, profile)| {
            total.saturating_sub(
                profile
                    .shape_ns
                    .saturating_add(profile.line_break_align_ns)
                    .saturating_add(profile.artifact_extract_ns)
                    .saturating_add(profile.grapheme_ns)
                    .saturating_add(profile.legal_offsets_ns),
            )
        })
        .collect::<Vec<_>>();
    let (median, p95) = summarize(totals);
    eprintln!(
        "issue263_text_profile label={label}.layout_and_caret_total n={} median_ns={median} p95_ns={p95}",
        profiles.len()
    );
    let timing_fields: [TimingField; 5] = [
        ("shape", |p| p.shape_ns),
        ("line_break_align", |p| p.line_break_align_ns),
        ("artifact_extract", |p| p.artifact_extract_ns),
        ("graphemes", |p| p.grapheme_ns),
        ("legal_offsets", |p| p.legal_offsets_ns),
    ];
    for (suffix, field) in timing_fields {
        report_ns(&format!("{label}.{suffix}"), profiles, field);
    }
    let (remaining_median, remaining_p95) = summarize(&mut remaining);
    eprintln!(
        "issue263_text_profile label={label}.remaining_text_work n={} median_ns={remaining_median} p95_ns={remaining_p95}",
        profiles.len()
    );
    let count_fields: [CountField; 11] = [
        ("shape_calls", |p| p.shape_calls),
        ("line_break_calls", |p| p.line_break_calls),
        ("artifact_extract_calls", |p| p.artifact_extract_calls),
        ("caret_map_calls", |p| p.caret_map_calls),
        ("grapheme_compute_calls", |p| p.grapheme_compute_calls),
        ("legal_offsets_calls", |p| p.legal_offsets_calls),
        ("artifact_lines", |p| p.artifact_lines),
        ("artifact_runs", |p| p.artifact_runs),
        ("artifact_glyphs", |p| p.artifact_glyphs),
        ("artifact_clusters", |p| p.artifact_clusters),
        ("grapheme_boundaries", |p| p.grapheme_boundaries),
    ];
    for (suffix, field) in count_fields {
        report_count(&format!("{label}.{suffix}"), profiles, field);
    }
    report_count(&format!("{label}.legal_offsets"), profiles, |p| {
        p.legal_offsets
    });
}

fn capture(
    system: &mut TextSystem,
    state: &mut TextLayoutState,
    request: &TextRequest,
    revision: u64,
    expected: TextLayoutDecision,
) -> (u128, TextPhaseProfile) {
    test_profile::reset();
    let started = Instant::now();
    let outcome = system
        .layout_text(state, request)
        .unwrap_or_else(|error| unreachable!("controlled profile layout succeeds: {error:?}"));
    assert_eq!(outcome.decision(), expected);
    let map = state
        .caret_map(snapshot(revision))
        .unwrap_or_else(|error| unreachable!("controlled caret map succeeds: {error:?}"));
    let offsets = map.legal_byte_offsets();
    assert!(!offsets.is_empty());
    let total = started.elapsed().as_nanos();
    (total, test_profile::take())
}

#[test]
#[ignore = "opt-in issue 263 release profile; run with --ignored --nocapture"]
#[allow(
    clippy::too_many_lines,
    reason = "one opt-in harness compares text phases across four document scales and edit states"
)]
fn issue_263_text_phase_profile() {
    let fixture = "multiline responsiveness fixture — retained text layout\n";
    let replacement_fixture = "replacement publication fixture — retained text layout state\n";

    for (name, lines) in [
        ("40_lines", 40),
        ("400_lines", 400),
        ("4000_lines", 4000),
        ("16000_lines", 16000),
    ] {
        let text = fixture.repeat(lines);
        let replacement = replacement_fixture.repeat(lines);
        let localized = format!("{text}x");
        let mut first_totals = Vec::with_capacity(SAMPLE_COUNT);
        let mut first_profiles = Vec::with_capacity(SAMPLE_COUNT);
        let mut unchanged_totals = Vec::with_capacity(SAMPLE_COUNT);
        let mut unchanged_profiles = Vec::with_capacity(SAMPLE_COUNT);
        let mut localized_totals = Vec::with_capacity(SAMPLE_COUNT);
        let mut localized_profiles = Vec::with_capacity(SAMPLE_COUNT);
        let mut replacement_totals = Vec::with_capacity(SAMPLE_COUNT);
        let mut replacement_profiles = Vec::with_capacity(SAMPLE_COUNT);

        for _ in 0..SAMPLE_COUNT {
            let mut text_system = system();
            let mut state = TextLayoutState::new();
            let original_request = request(text.clone());

            let (total, profile) = capture(
                &mut text_system,
                &mut state,
                &original_request,
                1,
                TextLayoutDecision::Reshaped,
            );
            first_totals.push(total);
            first_profiles.push(profile);

            let (total, profile) = capture(
                &mut text_system,
                &mut state,
                &original_request,
                1,
                TextLayoutDecision::Reused,
            );
            unchanged_totals.push(total);
            unchanged_profiles.push(profile);

            let localized_request = request(localized.clone());
            let (total, profile) = capture(
                &mut text_system,
                &mut state,
                &localized_request,
                2,
                TextLayoutDecision::Reshaped,
            );
            localized_totals.push(total);
            localized_profiles.push(profile);

            let replacement_request = request(replacement.clone());
            let (total, profile) = capture(
                &mut text_system,
                &mut state,
                &replacement_request,
                3,
                TextLayoutDecision::Reshaped,
            );
            replacement_totals.push(total);
            replacement_profiles.push(profile);
        }

        eprintln!(
            "issue263_text_fixture label={name} lines={lines} bytes={} replacement_bytes={} samples={SAMPLE_COUNT}",
            text.len(),
            replacement.len()
        );
        report(
            &format!("{name}.first_layout"),
            &mut first_totals,
            &first_profiles,
        );
        report(
            &format!("{name}.unchanged_layout"),
            &mut unchanged_totals,
            &unchanged_profiles,
        );
        report(
            &format!("{name}.localized_edit"),
            &mut localized_totals,
            &localized_profiles,
        );
        report(
            &format!("{name}.full_replacement"),
            &mut replacement_totals,
            &replacement_profiles,
        );
    }
}

#[test]
fn retained_layout_reuses_grapheme_boundaries_across_maps_and_relinebreak()
-> Result<(), Box<dyn std::error::Error>> {
    let mut text_system = system();
    let mut state = TextLayoutState::new();
    let text = "one two three four five six seven eight";
    let wide = TextRequest::new(
        text,
        typography(),
        TextConstraints::limited(
            LogicalLength::new(400.0)
                .unwrap_or_else(|_| unreachable!("controlled width is finite")),
        ),
    );
    let narrow = TextRequest::new(
        text,
        typography(),
        TextConstraints::limited(
            LogicalLength::new(72.0).unwrap_or_else(|_| unreachable!("controlled width is finite")),
        ),
    );

    test_profile::reset();
    assert_eq!(
        text_system.layout_text(&mut state, &wide)?.decision(),
        TextLayoutDecision::Reshaped
    );
    let layout_only_profile = test_profile::take();
    assert_eq!(layout_only_profile.caret_map_calls, 0);
    assert_eq!(layout_only_profile.grapheme_compute_calls, 0);

    test_profile::reset();
    let first_map = state.caret_map(snapshot(1))?;
    let _second_map = state.caret_map(snapshot(1))?;
    let first_profile = test_profile::take();
    assert_eq!(first_profile.caret_map_calls, 2);
    assert_eq!(first_profile.grapheme_compute_calls, 1);

    test_profile::reset();
    assert_eq!(
        text_system.layout_text(&mut state, &wide)?.decision(),
        TextLayoutDecision::Reused
    );
    let _reused_map = state.caret_map(snapshot(1))?;
    let exact_reuse_profile = test_profile::take();
    assert_eq!(exact_reuse_profile.caret_map_calls, 1);
    assert_eq!(exact_reuse_profile.grapheme_compute_calls, 0);

    test_profile::reset();
    assert_eq!(
        text_system.layout_text(&mut state, &narrow)?.decision(),
        TextLayoutDecision::Relinebroken
    );
    let relinebroken_map = state.caret_map(snapshot(1))?;
    let relinebroken_profile = test_profile::take();
    assert_eq!(relinebroken_profile.caret_map_calls, 1);
    assert_eq!(relinebroken_profile.grapheme_compute_calls, 0);
    assert!(!first_map.shares_layout_with(&relinebroken_map));

    let changed = TextRequest::new(
        format!("{text} changed"),
        typography(),
        TextConstraints::limited(
            LogicalLength::new(72.0).unwrap_or_else(|_| unreachable!("controlled width is finite")),
        ),
    );
    assert_eq!(
        text_system.layout_text(&mut state, &changed)?.decision(),
        TextLayoutDecision::Reshaped
    );
    test_profile::reset();
    let _changed_map = state.caret_map(snapshot(2))?;
    let changed_profile = test_profile::take();
    assert_eq!(changed_profile.caret_map_calls, 1);
    assert_eq!(changed_profile.grapheme_compute_calls, 1);
    Ok(())
}

#[test]
fn document_and_preedit_maps_share_one_retained_grapheme_cache()
-> Result<(), Box<dyn std::error::Error>> {
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

    let mut text_system = system();
    let mut state = TextLayoutState::new();
    let display_request = request(projection.display_text().to_owned());
    assert_eq!(
        text_system
            .layout_text(&mut state, &display_request)?
            .decision(),
        TextLayoutDecision::Reshaped
    );

    test_profile::reset();
    let _document_map = state.caret_map(snapshot(1))?;
    let _preedit_map = state.preedit_caret_map(projection)?;
    let profile = test_profile::take();
    assert_eq!(profile.caret_map_calls, 2);
    assert_eq!(profile.grapheme_compute_calls, 1);
    Ok(())
}
