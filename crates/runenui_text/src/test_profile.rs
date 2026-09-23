use std::{cell::RefCell, mem, time::Duration};

#[derive(Clone, Copy, Debug, Default)]
pub(super) struct TextPhaseProfile {
    pub(super) shape_ns: u128,
    pub(super) line_break_align_ns: u128,
    pub(super) artifact_extract_ns: u128,
    pub(super) grapheme_ns: u128,
    pub(super) legal_offsets_ns: u128,
    pub(super) shape_calls: usize,
    pub(super) line_break_calls: usize,
    pub(super) artifact_extract_calls: usize,
    pub(super) caret_map_calls: usize,
    pub(super) legal_offsets_calls: usize,
    pub(super) artifact_lines: usize,
    pub(super) artifact_runs: usize,
    pub(super) artifact_glyphs: usize,
    pub(super) artifact_clusters: usize,
    pub(super) grapheme_boundaries: usize,
    pub(super) legal_offsets: usize,
}

thread_local! {
    static PROFILE: RefCell<TextPhaseProfile> = RefCell::new(TextPhaseProfile::default());
}

const fn add_duration(target: &mut u128, duration: Duration) {
    *target = target.saturating_add(duration.as_nanos());
}

pub(super) fn reset() {
    PROFILE.with(|profile| *profile.borrow_mut() = TextPhaseProfile::default());
}

pub(super) fn take() -> TextPhaseProfile {
    PROFILE.with(|profile| mem::take(&mut *profile.borrow_mut()))
}

pub(super) fn record_shape(duration: Duration) {
    PROFILE.with(|profile| {
        let mut profile = profile.borrow_mut();
        add_duration(&mut profile.shape_ns, duration);
        profile.shape_calls = profile.shape_calls.saturating_add(1);
    });
}

pub(super) fn record_line_break_align(duration: Duration) {
    PROFILE.with(|profile| {
        let mut profile = profile.borrow_mut();
        add_duration(&mut profile.line_break_align_ns, duration);
        profile.line_break_calls = profile.line_break_calls.saturating_add(1);
    });
}

pub(super) fn record_artifact_extract(duration: Duration) {
    PROFILE.with(|profile| {
        let mut profile = profile.borrow_mut();
        add_duration(&mut profile.artifact_extract_ns, duration);
        profile.artifact_extract_calls = profile.artifact_extract_calls.saturating_add(1);
    });
}

pub(super) fn record_artifact_counts(lines: usize, runs: usize, glyphs: usize, clusters: usize) {
    PROFILE.with(|profile| {
        let mut profile = profile.borrow_mut();
        profile.artifact_lines = profile.artifact_lines.saturating_add(lines);
        profile.artifact_runs = profile.artifact_runs.saturating_add(runs);
        profile.artifact_glyphs = profile.artifact_glyphs.saturating_add(glyphs);
        profile.artifact_clusters = profile.artifact_clusters.saturating_add(clusters);
    });
}

pub(super) fn record_graphemes(duration: Duration, boundaries: usize) {
    PROFILE.with(|profile| {
        let mut profile = profile.borrow_mut();
        add_duration(&mut profile.grapheme_ns, duration);
        profile.caret_map_calls = profile.caret_map_calls.saturating_add(1);
        profile.grapheme_boundaries = profile.grapheme_boundaries.saturating_add(boundaries);
    });
}

pub(super) fn record_legal_offsets(duration: Duration, offsets: usize) {
    PROFILE.with(|profile| {
        let mut profile = profile.borrow_mut();
        add_duration(&mut profile.legal_offsets_ns, duration);
        profile.legal_offsets_calls = profile.legal_offsets_calls.saturating_add(1);
        profile.legal_offsets = profile.legal_offsets.saturating_add(offsets);
    });
}
