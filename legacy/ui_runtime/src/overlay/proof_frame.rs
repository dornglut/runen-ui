use ui_math::{UiPoint, UiRect, UiSize};
use ui_render_data::{
    BorderPrimitive, GlyphRunPrimitive, RectPrimitive, UiDrawKey, UiFrame, UiLayer, UiLayerId,
    UiPaint, UiPrimitive, UiSortKey, UiSurface, UiSurfaceId,
};
use ui_text::FontId;

use super::{BASE_CONTROLS_OVERLAY_LAYERING_PROOF_ID, OverlayLayeringReport};

#[derive(Debug, Clone, PartialEq)]
pub struct OverlayLayeringVisualProof {
    pub proof_id: String,
    pub report: OverlayLayeringReport,
}

pub fn overlay_layering_report_to_visual_proof(
    report: OverlayLayeringReport,
) -> OverlayLayeringVisualProof {
    OverlayLayeringVisualProof {
        proof_id: BASE_CONTROLS_OVERLAY_LAYERING_PROOF_ID.to_owned(),
        report,
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct OverlayLayeringProofRenderFrame {
    pub proof_id: String,
    pub frame: UiFrame,
    pub summary: OverlayLayeringProofRenderSummary,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OverlayLayeringProofRenderSummary {
    pub anchor_rows: usize,
    pub stack_rows: usize,
    pub placement_rows: usize,
    pub report_rows: usize,
    pub has_main_inspector_and_report: bool,
}

pub fn overlay_layering_visual_proof_to_frame(
    proof: &OverlayLayeringVisualProof,
) -> OverlayLayeringProofRenderFrame {
    let report = &proof.report;
    let size = UiSize::new(920.0, 640.0);
    let mut primitives = Vec::new();
    let mut order = 0_u32;

    panel(
        &mut primitives,
        &mut order,
        UiRect::new(16.0, 16.0, 280.0, 600.0),
        "main: anchors",
    );
    panel(
        &mut primitives,
        &mut order,
        UiRect::new(316.0, 16.0, 280.0, 600.0),
        "inspector: overlays",
    );
    panel(
        &mut primitives,
        &mut order,
        UiRect::new(616.0, 16.0, 280.0, 600.0),
        "report: evidence",
    );
    label(
        &mut primitives,
        &mut order,
        32.0,
        58.0,
        &format!("anchors={}", report.declarations.len()),
    );
    label(
        &mut primitives,
        &mut order,
        332.0,
        58.0,
        &format!("open={}", report.open_intents.len()),
    );
    label(
        &mut primitives,
        &mut order,
        632.0,
        58.0,
        &format!("stack={}", report.stack_entries.len()),
    );
    label(
        &mut primitives,
        &mut order,
        632.0,
        76.0,
        &format!("placement={}", report.placement_resolutions.len()),
    );
    label(
        &mut primitives,
        &mut order,
        632.0,
        94.0,
        &format!("dismiss={}", report.dismissal_evidence.len()),
    );
    label(
        &mut primitives,
        &mut order,
        632.0,
        112.0,
        &format!("suppress={}", report.suppression_evidence.len()),
    );

    let mut surface = UiSurface::new(UiSurfaceId(13), size);
    surface.push_layer(UiLayer::with_primitives(UiLayerId(0), primitives));
    OverlayLayeringProofRenderFrame {
        proof_id: proof.proof_id.clone(),
        frame: UiFrame::with_surfaces(vec![surface]),
        summary: OverlayLayeringProofRenderSummary {
            anchor_rows: report.declarations.len(),
            stack_rows: report.stack_entries.len(),
            placement_rows: report.placement_resolutions.len(),
            report_rows: report.input_steps.len()
                + report.open_intents.len()
                + report.dismissal_evidence.len()
                + report.suppression_evidence.len(),
            has_main_inspector_and_report: !report.declarations.is_empty()
                && !report.open_intents.is_empty()
                && !report.input_steps.is_empty(),
        },
    }
}

pub fn base_controls_overlay_layering_proof_frame(
    report: OverlayLayeringReport,
) -> OverlayLayeringProofRenderFrame {
    let proof = overlay_layering_report_to_visual_proof(report);
    overlay_layering_visual_proof_to_frame(&proof)
}

fn panel(primitives: &mut Vec<UiPrimitive>, order: &mut u32, area: UiRect, title: &str) {
    primitives.push(
        RectPrimitive::new(
            area,
            3.0,
            UiPaint::rgba(0.11, 0.12, 0.14, 1.0),
            UiDrawKey::new(1301, None),
            sort_key(order),
        )
        .into(),
    );
    primitives.push(
        BorderPrimitive::new(
            area,
            3.0,
            1.0,
            UiPaint::WHITE,
            UiDrawKey::new(1302, None),
            sort_key(order),
        )
        .into(),
    );
    label(primitives, order, area.x + 12.0, area.y + 20.0, title);
}

fn label(primitives: &mut Vec<UiPrimitive>, order: &mut u32, x: f32, y: f32, text: &str) {
    let layout = crate::text_emission::proof_label_layout(
        text,
        UiPoint::new(x, y),
        240.0,
        FontId(13),
        *order,
    );
    primitives.push(
        GlyphRunPrimitive::new(
            layout,
            Some(UiRect::new(x, y - 12.0, 240.0, 16.0)),
            UiPaint::WHITE,
            UiDrawKey::new(1303, None),
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
