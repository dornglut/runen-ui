use std::{cell::RefCell, mem, time::Duration};

use runenui_text::TextLayoutDecision;

#[derive(Clone, Copy, Debug, Default)]
struct RuntimePublicationProfile {
    surface_plan_ns: u128,
    layout_ns: u128,
    paint_ns: u128,
    displayed_text_targets_ns: u128,
    semantic_candidate_ns: u128,
    semantic_plan_ns: u128,
    widget_measure_callback_ns: u128,
    text_request_prepare_ns: u128,
    text_layout_ns: u128,
    measure_calls: usize,
    reshaped: usize,
    relinebroken: usize,
    reused: usize,
    paint_text_run_items: usize,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct SurfacePublicationTestProfile {
    pub surface_plan_ns: u128,
    pub layout_ns: u128,
    pub paint_ns: u128,
    pub displayed_text_targets_ns: u128,
    pub semantic_candidate_ns: u128,
    pub semantic_plan_ns: u128,
    pub widget_measure_callback_ns: u128,
    pub text_request_prepare_ns: u128,
    pub text_layout_ns: u128,
    pub measure_calls: usize,
    pub reshaped: usize,
    pub relinebroken: usize,
    pub reused: usize,
    pub paint_text_run_items: usize,
}

thread_local! {
    static PROFILE: RefCell<RuntimePublicationProfile> =
        RefCell::new(RuntimePublicationProfile::default());
}

fn add_duration(target: &mut u128, duration: Duration) {
    *target = target.saturating_add(duration.as_nanos());
}

pub(crate) fn reset() {
    PROFILE.with(|profile| *profile.borrow_mut() = RuntimePublicationProfile::default());
}

pub(crate) fn take() -> SurfacePublicationTestProfile {
    let profile = PROFILE.with(|profile| mem::take(&mut *profile.borrow_mut()));
    SurfacePublicationTestProfile {
        surface_plan_ns: profile.surface_plan_ns,
        layout_ns: profile.layout_ns,
        paint_ns: profile.paint_ns,
        displayed_text_targets_ns: profile.displayed_text_targets_ns,
        semantic_candidate_ns: profile.semantic_candidate_ns,
        semantic_plan_ns: profile.semantic_plan_ns,
        widget_measure_callback_ns: profile.widget_measure_callback_ns,
        text_request_prepare_ns: profile.text_request_prepare_ns,
        text_layout_ns: profile.text_layout_ns,
        measure_calls: profile.measure_calls,
        reshaped: profile.reshaped,
        relinebroken: profile.relinebroken,
        reused: profile.reused,
        paint_text_run_items: profile.paint_text_run_items,
    }
}

pub(crate) fn record_surface_plan(duration: Duration) {
    PROFILE.with(|profile| add_duration(&mut profile.borrow_mut().surface_plan_ns, duration));
}

pub(crate) fn record_layout(duration: Duration) {
    PROFILE.with(|profile| add_duration(&mut profile.borrow_mut().layout_ns, duration));
}

pub(crate) fn record_paint(duration: Duration) {
    PROFILE.with(|profile| add_duration(&mut profile.borrow_mut().paint_ns, duration));
}

pub(crate) fn record_displayed_text_targets(duration: Duration) {
    PROFILE.with(|profile| {
        add_duration(&mut profile.borrow_mut().displayed_text_targets_ns, duration);
    });
}

pub(crate) fn record_semantic_candidate(duration: Duration) {
    PROFILE.with(|profile| {
        add_duration(&mut profile.borrow_mut().semantic_candidate_ns, duration);
    });
}

pub(crate) fn record_semantic_plan(duration: Duration) {
    PROFILE.with(|profile| add_duration(&mut profile.borrow_mut().semantic_plan_ns, duration));
}

pub(crate) fn record_measure_callback(duration: Duration) {
    PROFILE.with(|profile| {
        let mut profile = profile.borrow_mut();
        add_duration(&mut profile.widget_measure_callback_ns, duration);
        profile.measure_calls = profile.measure_calls.saturating_add(1);
    });
}

pub(crate) fn record_request_prepare(duration: Duration) {
    PROFILE.with(|profile| add_duration(&mut profile.borrow_mut().text_request_prepare_ns, duration));
}

pub(crate) fn record_text_layout(duration: Duration) {
    PROFILE.with(|profile| add_duration(&mut profile.borrow_mut().text_layout_ns, duration));
}

pub(crate) fn record_text_layout_decision(decision: TextLayoutDecision) {
    PROFILE.with(|profile| {
        let mut profile = profile.borrow_mut();
        match decision {
            TextLayoutDecision::Reshaped => profile.reshaped = profile.reshaped.saturating_add(1),
            TextLayoutDecision::Relinebroken => {
                profile.relinebroken = profile.relinebroken.saturating_add(1);
            }
            TextLayoutDecision::Reused => profile.reused = profile.reused.saturating_add(1),
        }
    });
}

pub(crate) fn record_paint_text_run_items(count: usize) {
    PROFILE.with(|profile| {
        let mut profile = profile.borrow_mut();
        profile.paint_text_run_items = profile.paint_text_run_items.saturating_add(count);
    });
}
