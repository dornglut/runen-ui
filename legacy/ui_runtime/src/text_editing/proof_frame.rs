use ui_math::{UiPoint, UiRect, UiSize};
use ui_render_data::{
    BorderPrimitive, GlyphRunPrimitive, RectPrimitive, UiDrawKey, UiFrame, UiLayer, UiLayerId,
    UiPaint, UiPrimitive, UiSortKey, UiSurface, UiSurfaceId,
};
use ui_text::FontId;

use super::{BASE_CONTROLS_TEXT_EDITING_PROOF_ID, TextEditingReport};

#[derive(Debug, Clone, PartialEq)]
pub struct TextEditingVisualProof {
    pub proof_id: String,
    pub report: TextEditingReport,
}

pub fn text_editing_report_to_visual_proof(report: TextEditingReport) -> TextEditingVisualProof {
    TextEditingVisualProof {
        proof_id: BASE_CONTROLS_TEXT_EDITING_PROOF_ID.to_owned(),
        report,
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TextEditingProofRenderFrame {
    pub proof_id: String,
    pub frame: UiFrame,
    pub summary: TextEditingProofRenderSummary,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TextEditingProofRenderSummary {
    pub descriptor_rows: usize,
    pub value_rows: usize,
    pub accepted_intent_rows: usize,
    pub suppressed_intent_rows: usize,
    pub lifecycle_rows: usize,
    pub caret_rows: usize,
    pub selection_rows: usize,
    pub composition_rows: usize,
    pub has_main_inspector_and_report: bool,
    pub no_bypass_proven: bool,
}

pub fn text_editing_visual_proof_to_frame(
    proof: &TextEditingVisualProof,
) -> TextEditingProofRenderFrame {
    let report = &proof.report;
    let size = UiSize::new(940.0, 660.0);
    let mut primitives = Vec::new();
    let mut order = 0_u32;

    panel(
        &mut primitives,
        &mut order,
        UiRect::new(16.0, 16.0, 288.0, 620.0),
        "main: editable target",
    );
    panel(
        &mut primitives,
        &mut order,
        UiRect::new(324.0, 16.0, 288.0, 620.0),
        "inspector: text editing",
    );
    panel(
        &mut primitives,
        &mut order,
        UiRect::new(632.0, 16.0, 288.0, 620.0),
        "report: evidence",
    );

    let value = representative_value(report);
    let final_value = report.value_evidence.last();

    label(
        &mut primitives,
        &mut order,
        32.0,
        58.0,
        &format!("target: {}", descriptor_target(report)),
    );
    label(
        &mut primitives,
        &mut order,
        32.0,
        82.0,
        &format!(
            "value: {}",
            value.map_or("<none>", |row| row.rendered_value.as_str())
        ),
    );
    label(
        &mut primitives,
        &mut order,
        32.0,
        106.0,
        &format!(
            "committed: {}",
            final_value.map_or("<empty>", |row| empty_marker(&row.committed_text))
        ),
    );
    label(
        &mut primitives,
        &mut order,
        32.0,
        130.0,
        &format!(
            "caret: {}",
            final_value.map_or("<none>", |row| row.caret.as_str())
        ),
    );
    label(
        &mut primitives,
        &mut order,
        32.0,
        154.0,
        &format!(
            "selection: {}",
            selected_range(report).unwrap_or_else(|| {
                final_value
                    .map(|row| row.selection.clone())
                    .unwrap_or_else(|| "<none>".to_owned())
            })
        ),
    );
    label(
        &mut primitives,
        &mut order,
        32.0,
        178.0,
        &format!(
            "composition: {}",
            value
                .and_then(|row| row.composition_text.as_deref())
                .unwrap_or("<none>")
        ),
    );

    label(
        &mut primitives,
        &mut order,
        340.0,
        58.0,
        &format!("modes: {}", mode_list(report)),
    );
    label(
        &mut primitives,
        &mut order,
        340.0,
        82.0,
        &format!("accepted: {}", accepted_intents(report)),
    );
    label(
        &mut primitives,
        &mut order,
        340.0,
        106.0,
        &format!("suppressed: {}", suppression_summary(report)),
    );
    label(
        &mut primitives,
        &mut order,
        340.0,
        130.0,
        &format!("caret rows: {}", report.caret_evidence.len()),
    );
    label(
        &mut primitives,
        &mut order,
        340.0,
        154.0,
        &format!("selection rows: {}", report.selection_evidence.len()),
    );
    label(
        &mut primitives,
        &mut order,
        340.0,
        178.0,
        &format!("composition rows: {}", report.composition_evidence.len()),
    );

    label(
        &mut primitives,
        &mut order,
        648.0,
        58.0,
        &format!("steps: {}", report.input_steps.len()),
    );
    label(
        &mut primitives,
        &mut order,
        648.0,
        82.0,
        &format!("lifecycle: {}", lifecycle_summary(report)),
    );
    label(
        &mut primitives,
        &mut order,
        648.0,
        106.0,
        &format!("value rows: {}", report.value_evidence.len()),
    );
    label(
        &mut primitives,
        &mut order,
        648.0,
        130.0,
        &format!(
            "no_bypass: {}",
            report.boundary_assertions.no_bypass_evidence()
        ),
    );
    label(
        &mut primitives,
        &mut order,
        648.0,
        154.0,
        &format!(
            "host/product/authored: {}/{}/{}",
            report.boundary_assertions.host_commands_executed,
            report.boundary_assertions.product_mutations,
            report.boundary_assertions.authored_ui_edits,
        ),
    );
    label(
        &mut primitives,
        &mut order,
        648.0,
        178.0,
        &format!(
            "undo/plugin: {}/{}",
            report.boundary_assertions.product_undo_redo_operations,
            report.boundary_assertions.plugin_framework_operations,
        ),
    );

    let mut surface = UiSurface::new(UiSurfaceId(14), size);
    surface.push_layer(UiLayer::with_primitives(UiLayerId(0), primitives));
    TextEditingProofRenderFrame {
        proof_id: proof.proof_id.clone(),
        frame: UiFrame::with_surfaces(vec![surface]),
        summary: TextEditingProofRenderSummary {
            descriptor_rows: report.descriptor_evidence.len(),
            value_rows: report.value_evidence.len(),
            accepted_intent_rows: report.accepted_edit_intents.len(),
            suppressed_intent_rows: report.suppressed_edit_intents.len(),
            lifecycle_rows: report.lifecycle_transitions.len(),
            caret_rows: report.caret_evidence.len(),
            selection_rows: report.selection_evidence.len(),
            composition_rows: report.composition_evidence.len(),
            has_main_inspector_and_report: !report.descriptor_evidence.is_empty()
                && !report.input_steps.is_empty()
                && !report.accepted_edit_intents.is_empty(),
            no_bypass_proven: report.boundary_assertions.no_bypass_evidence(),
        },
    }
}

pub fn base_controls_text_editing_proof_frame(
    report: TextEditingReport,
) -> TextEditingProofRenderFrame {
    let proof = text_editing_report_to_visual_proof(report);
    text_editing_visual_proof_to_frame(&proof)
}

fn mode_list(report: &TextEditingReport) -> String {
    let mut modes = report
        .descriptor_evidence
        .iter()
        .map(|descriptor| descriptor.mode.clone())
        .collect::<Vec<_>>();
    modes.sort();
    modes.dedup();
    modes.join(",")
}

fn descriptor_target(report: &TextEditingReport) -> String {
    report
        .descriptor_evidence
        .first()
        .map(|descriptor| descriptor.control_kind_id.clone())
        .unwrap_or_else(|| "<none>".to_owned())
}

fn representative_value(report: &TextEditingReport) -> Option<&super::TextEditingValueEvidence> {
    report
        .value_evidence
        .iter()
        .find(|row| row.composition_text.is_some())
        .or_else(|| report.value_evidence.last())
}

fn selected_range(report: &TextEditingReport) -> Option<String> {
    report
        .selection_evidence
        .iter()
        .rev()
        .find(|selection| !selection.collapsed)
        .map(|selection| format!("{}..{}", selection.anchor, selection.extent))
}

fn accepted_intents(report: &TextEditingReport) -> String {
    let mut intents = report
        .accepted_edit_intents
        .iter()
        .map(|intent| intent.intent.clone())
        .collect::<Vec<_>>();
    intents.sort();
    intents.dedup();
    if intents.is_empty() {
        return "<none>".to_owned();
    }
    intents.join(", ")
}

fn suppression_summary(report: &TextEditingReport) -> String {
    report
        .suppressed_edit_intents
        .iter()
        .find(|suppression| suppression.intent == "paste")
        .or_else(|| report.suppressed_edit_intents.first())
        .map(|suppression| format!("{} / {}", suppression.intent, suppression.reason))
        .unwrap_or_else(|| "<none>".to_owned())
}

fn lifecycle_summary(report: &TextEditingReport) -> String {
    report
        .lifecycle_transitions
        .last()
        .map(|transition| format!("{} -> {}", transition.from.as_str(), transition.to.as_str()))
        .unwrap_or_else(|| "<none>".to_owned())
}

fn empty_marker(value: &str) -> &str {
    if value.is_empty() { "<empty>" } else { value }
}

fn panel(primitives: &mut Vec<UiPrimitive>, order: &mut u32, area: UiRect, title: &str) {
    primitives.push(
        RectPrimitive::new(
            area,
            3.0,
            UiPaint::rgba(0.10, 0.11, 0.13, 1.0),
            UiDrawKey::new(1401, None),
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
            UiDrawKey::new(1402, None),
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
        250.0,
        FontId(14),
        *order,
    );
    primitives.push(
        GlyphRunPrimitive::new(
            layout,
            Some(UiRect::new(x, y - 12.0, 250.0, 16.0)),
            UiPaint::WHITE,
            UiDrawKey::new(1403, None),
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
