//! Static render-frame adapter for generic interaction visual proof.
//!
//! `InteractionVisualProof` is semantic visible proof data. This module turns
//! that proof into a deterministic renderer-neutral `UiFrame` that existing
//! static mount, gallery, or story infrastructure can inspect or render.
//!
//! The adapter does not execute host commands, mutate product state, create
//! overlays/layering behavior, or perform text editing.

use ui_math::{UiPoint, UiRect, UiSize};
use ui_render_data::{
    BorderPrimitive, GlyphRunPrimitive, RectPrimitive, UiDrawKey, UiFrame, UiLayer, UiLayerId,
    UiPaint, UiPrimitive, UiSortKey, UiSurface, UiSurfaceId,
};
use ui_text::FontId;

use crate::{
    InteractionBoundaryAssertions, InteractionReportView, InteractionVisibleState,
    InteractionVisualControl, InteractionVisualProof,
};

const INTERACTION_PROOF_SURFACE_ID: UiSurfaceId = UiSurfaceId(1);
const INTERACTION_PROOF_RECT_DRAW_KEY: u64 = 1;
const INTERACTION_PROOF_BORDER_DRAW_KEY: u64 = 2;
const INTERACTION_PROOF_GLYPH_DRAW_KEY: u64 = 3;
const INTERACTION_PROOF_FONT_ID: FontId = FontId(1);

/// Renderer-neutral frame generated from an interaction visual proof.
///
/// The frame is proof evidence, not product UI. It turns reusable interaction
/// markers into stable rectangles, borders, and text labels so static mount,
/// gallery, or story infrastructure can inspect and render the proof.
#[derive(Debug, Clone, PartialEq)]
pub struct InteractionProofRenderFrame {
    /// Stable proof identifier used by tests, reports, and gallery/story adapters.
    pub proof_id: String,

    /// Renderer-neutral UI frame containing main, inspector, and report regions.
    pub frame: UiFrame,

    /// Compact counts used by tests and closeout evidence.
    pub summary: InteractionProofRenderSummary,
}

/// Stable summary of the rendered interaction proof frame.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InteractionProofRenderSummary {
    /// Number of controls represented in the main proof region.
    pub main_control_count: usize,

    /// Number of observed visible markers rendered across all controls.
    pub marker_count: usize,

    /// Number of inspector requirement rows rendered.
    pub inspector_requirement_count: usize,

    /// Number of report rows rendered.
    pub report_row_count: usize,

    /// Whether the frame contains all three required proof areas.
    pub has_main_inspector_and_report: bool,
}

/// Builds a renderer-neutral static frame for an interaction visual proof.
///
/// This function does not execute commands, mutate product state, open overlays,
/// or perform text editing. It only projects already-formed proof evidence into
/// render data containing a main region, inspector region, and report region.
pub fn interaction_visual_proof_to_frame(
    proof: &InteractionVisualProof,
) -> InteractionProofRenderFrame {
    let report_rows = report_rows(&proof.report_view);
    let main_height = proof.main_view.controls.len() as f32 * 54.0 + 96.0;
    let inspector_height = proof.inspector_view.declared_requirements.len() as f32 * 16.0 + 180.0;
    let report_height = report_rows.len() as f32 * 16.0 + 64.0;
    let surface_height = main_height
        .max(inspector_height)
        .max(report_height)
        .max(640.0);
    let surface_size = UiSize::new(1100.0, surface_height);
    let mut primitives = Vec::new();
    let mut order = 0_u32;

    push_rect(
        &mut primitives,
        &mut order,
        UiRect::new(0.0, 0.0, surface_size.width, surface_size.height),
        UiPaint::rgba(0.08, 0.09, 0.1, 1.0),
    );

    let main_rect = UiRect::new(16.0, 16.0, 330.0, surface_height - 32.0);
    let inspector_rect = UiRect::new(362.0, 16.0, 300.0, surface_height - 32.0);
    let report_rect = UiRect::new(678.0, 16.0, 406.0, surface_height - 32.0);
    push_section(
        &mut primitives,
        &mut order,
        main_rect,
        "main: mounted controls and visible markers",
    );
    push_section(
        &mut primitives,
        &mut order,
        inspector_rect,
        "inspector: selected descriptor and state",
    );
    push_section(
        &mut primitives,
        &mut order,
        report_rect,
        "report: replay facts, events, outcomes, boundaries",
    );

    render_main_controls(
        &mut primitives,
        &mut order,
        &proof.main_view.controls,
        main_rect,
    );
    render_inspector(proof, &mut primitives, &mut order, inspector_rect);
    render_report_rows(&mut primitives, &mut order, report_rect, &report_rows);

    let mut surface = UiSurface::new(INTERACTION_PROOF_SURFACE_ID, surface_size);
    surface.push_layer(UiLayer::with_primitives(UiLayerId(0), primitives));
    let frame = UiFrame::with_surfaces(vec![surface]);

    InteractionProofRenderFrame {
        proof_id: proof.proof_id.clone(),
        frame,
        summary: InteractionProofRenderSummary {
            main_control_count: proof.main_view.controls.len(),
            marker_count: proof
                .main_view
                .controls
                .iter()
                .map(|control| control.observed_markers.len())
                .sum(),
            inspector_requirement_count: proof.inspector_view.declared_requirements.len(),
            report_row_count: report_rows.len(),
            has_main_inspector_and_report: !proof.main_view.controls.is_empty()
                && proof.inspector_view.selected_widget.is_some()
                && !report_rows.is_empty(),
        },
    }
}

fn render_main_controls(
    primitives: &mut Vec<UiPrimitive>,
    order: &mut u32,
    controls: &[InteractionVisualControl],
    section_rect: UiRect,
) {
    for (index, control) in controls.iter().enumerate() {
        let y = section_rect.y + 36.0 + index as f32 * 54.0;
        let rect = UiRect::new(section_rect.x + 12.0, y, section_rect.width - 24.0, 44.0);
        let paint = control_paint(control);
        push_rect(primitives, order, rect, paint);
        push_border(primitives, order, rect, 1.0, control_border_paint(control));
        push_glyph(
            primitives,
            order,
            UiPoint::new(rect.x + 8.0, rect.y + 14.0),
            &format!(
                "#{:?} {} ({})",
                control.widget_id, control.label, control.control_kind_id
            ),
            rect.width - 16.0,
        );
        push_glyph(
            primitives,
            order,
            UiPoint::new(rect.x + 8.0, rect.y + 30.0),
            &format!(
                "observed: {} | current: {}",
                marker_labels(control),
                current_state_labels(control)
            ),
            rect.width - 16.0,
        );
    }
}

fn render_inspector(
    proof: &InteractionVisualProof,
    primitives: &mut Vec<UiPrimitive>,
    order: &mut u32,
    section_rect: UiRect,
) {
    let inspector = &proof.inspector_view;
    let mut y = section_rect.y + 40.0;
    let x = section_rect.x + 12.0;
    let width = section_rect.width - 24.0;

    push_glyph(
        primitives,
        order,
        UiPoint::new(x, y),
        &format!("proof: {}", proof.proof_id),
        width,
    );
    y += 18.0;
    push_glyph(
        primitives,
        order,
        UiPoint::new(x, y),
        &format!("selected: {:?}", inspector.selected_widget),
        width,
    );
    y += 18.0;
    push_glyph(
        primitives,
        order,
        UiPoint::new(x, y),
        &format!(
            "kind: {}",
            inspector.control_kind_id.as_deref().unwrap_or("<none>")
        ),
        width,
    );
    y += 18.0;
    push_glyph(
        primitives,
        order,
        UiPoint::new(x, y),
        &format!(
            "observed states: {}",
            state_labels(&inspector.observed_reusable_interaction_states)
        ),
        width,
    );
    y += 18.0;
    push_glyph(
        primitives,
        order,
        UiPoint::new(x, y),
        &format!(
            "current states: {}",
            state_labels(&inspector.current_reusable_interaction_state_set)
        ),
        width,
    );
    y += 18.0;
    push_glyph(
        primitives,
        order,
        UiPoint::new(x, y),
        &format!(
            "text-intent-probe: {} read-only: {}",
            inspector.text_intent_probe, inspector.read_only
        ),
        width,
    );
    y += 22.0;
    push_glyph(
        primitives,
        order,
        UiPoint::new(x, y),
        "declared requirements",
        width,
    );
    y += 18.0;

    for requirement in &inspector.declared_requirements {
        push_glyph(primitives, order, UiPoint::new(x, y), requirement, width);
        y += 16.0;
    }
}

fn render_report_rows(
    primitives: &mut Vec<UiPrimitive>,
    order: &mut u32,
    section_rect: UiRect,
    rows: &[String],
) {
    let x = section_rect.x + 12.0;
    let mut y = section_rect.y + 40.0;
    let width = section_rect.width - 24.0;
    for row in rows {
        push_glyph(primitives, order, UiPoint::new(x, y), row, width);
        y += 16.0;
    }
}

fn report_rows(report: &InteractionReportView) -> Vec<String> {
    let mut rows = Vec::new();
    extend_rows(&mut rows, "steps", &report.replay_steps);
    extend_rows(&mut rows, "target", &report.target_resolution);
    extend_rows(&mut rows, "focus", &report.focus_resolution);
    extend_rows(&mut rows, "transition", &report.state_transitions);
    extend_rows(&mut rows, "fact", &report.runtime_facts);
    extend_rows(&mut rows, "event", &report.runtime_events);
    extend_rows(&mut rows, "outcome", &report.semantic_outcomes);
    extend_rows(&mut rows, "suppressed", &report.suppressed_events);
    extend_rows(&mut rows, "no-target", &report.no_target_events);
    rows.push(boundary_row(report.boundary_assertions));
    rows
}

fn extend_rows(rows: &mut Vec<String>, section: &str, values: &[String]) {
    rows.push(format!("{section}: {} rows", values.len()));
    rows.extend(values.iter().map(|value| format!("  {section}: {value}")));
}

fn boundary_row(boundary: InteractionBoundaryAssertions) -> String {
    format!(
        "boundary: host={} product={} overlay={} text-edits={}",
        boundary.host_commands_executed,
        boundary.product_mutations,
        boundary.overlay_events,
        boundary.text_edit_transactions
    )
}

fn marker_labels(control: &InteractionVisualControl) -> String {
    control
        .observed_markers
        .iter()
        .map(|marker| marker.label.as_str())
        .collect::<Vec<_>>()
        .join(",")
}

fn current_state_labels(control: &InteractionVisualControl) -> String {
    state_labels(&control.current_states)
}

fn state_labels(states: &[InteractionVisibleState]) -> String {
    if states.is_empty() {
        return "<none>".to_owned();
    }
    states
        .iter()
        .map(|state| state.as_str())
        .collect::<Vec<_>>()
        .join(",")
}

fn control_paint(control: &InteractionVisualControl) -> UiPaint {
    if control.has_marker(InteractionVisibleState::Suppressed) {
        return UiPaint::rgba(0.28, 0.16, 0.16, 1.0);
    }
    if control.has_marker(InteractionVisibleState::Disabled) {
        return UiPaint::rgba(0.18, 0.18, 0.2, 1.0);
    }
    if control.has_marker(InteractionVisibleState::Pressed) {
        return UiPaint::rgba(0.12, 0.2, 0.28, 1.0);
    }
    if control.has_marker(InteractionVisibleState::Hovered) {
        return UiPaint::rgba(0.14, 0.24, 0.22, 1.0);
    }
    UiPaint::rgba(0.15, 0.16, 0.18, 1.0)
}

fn control_border_paint(control: &InteractionVisualControl) -> UiPaint {
    if control.has_marker(InteractionVisibleState::FocusVisible)
        || control.has_marker(InteractionVisibleState::Focused)
    {
        return UiPaint::rgba(0.72, 0.84, 1.0, 1.0);
    }
    if control.has_marker(InteractionVisibleState::ActivationRequested) {
        return UiPaint::rgba(0.82, 0.68, 0.3, 1.0);
    }
    UiPaint::rgba(0.4, 0.43, 0.48, 1.0)
}

fn push_section(primitives: &mut Vec<UiPrimitive>, order: &mut u32, rect: UiRect, label: &str) {
    push_rect(
        primitives,
        order,
        rect,
        UiPaint::rgba(0.11, 0.12, 0.14, 1.0),
    );
    push_border(
        primitives,
        order,
        rect,
        1.0,
        UiPaint::rgba(0.5, 0.53, 0.58, 1.0),
    );
    push_glyph(
        primitives,
        order,
        UiPoint::new(rect.x + 12.0, rect.y + 20.0),
        label,
        rect.width - 24.0,
    );
}

fn push_rect(primitives: &mut Vec<UiPrimitive>, order: &mut u32, rect: UiRect, paint: UiPaint) {
    primitives.push(
        RectPrimitive::new(
            rect,
            3.0,
            paint,
            UiDrawKey::new(INTERACTION_PROOF_RECT_DRAW_KEY, None),
            sort_key(order),
        )
        .into(),
    );
}

fn push_border(
    primitives: &mut Vec<UiPrimitive>,
    order: &mut u32,
    rect: UiRect,
    width: f32,
    paint: UiPaint,
) {
    primitives.push(
        BorderPrimitive::new(
            rect,
            3.0,
            width,
            paint,
            UiDrawKey::new(INTERACTION_PROOF_BORDER_DRAW_KEY, None),
            sort_key(order),
        )
        .into(),
    );
}

fn push_glyph(
    primitives: &mut Vec<UiPrimitive>,
    order: &mut u32,
    origin: UiPoint,
    text: &str,
    max_width: f32,
) {
    let layout = crate::text_emission::proof_label_layout(
        text,
        origin,
        max_width,
        INTERACTION_PROOF_FONT_ID,
        *order,
    );
    primitives.push(
        GlyphRunPrimitive::new(
            layout,
            Some(UiRect::new(origin.x, origin.y - 12.0, max_width, 16.0)),
            UiPaint::WHITE,
            UiDrawKey::new(INTERACTION_PROOF_GLYPH_DRAW_KEY, None),
            sort_key(order),
        )
        .into(),
    );
}

fn sort_key(order: &mut u32) -> UiSortKey {
    let key = UiSortKey::new(0, 0, *order);
    *order += 1;
    key
}
