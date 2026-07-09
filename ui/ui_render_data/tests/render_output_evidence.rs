use ui_math::{UiRect, UiSize};
use ui_render_data::{
    BorderPrimitive, RectPrimitive, UiDrawKey, UiExpectedPrimitiveCount, UiFrame,
    UiFrameOutputSummary, UiLayer, UiLayerId, UiPaint, UiPrimitive, UiPrimitiveFamily,
    UiRenderOutputDiagnosticKind, UiRenderOutputEvidence, UiRenderOutputProvenance, UiSortKey,
    UiSurface, UiSurfaceId,
};

#[test]
fn frame_output_summary_counts_primitive_families_without_backend_state() {
    let frame = evidence_frame();
    let summary = UiFrameOutputSummary::from_frame(&frame);

    assert_eq!(summary.surface_count, 1);
    assert_eq!(summary.layer_count, 1);
    assert_eq!(summary.primitive_count, 3);
    assert_eq!(summary.count_for_family(UiPrimitiveFamily::Rect), 2);
    assert_eq!(summary.count_for_family(UiPrimitiveFamily::Border), 1);
    assert_eq!(summary.count_for_family(UiPrimitiveFamily::GlyphRun), 0);
    assert_eq!(summary.surfaces[0].surface_id, 7);
}

#[test]
fn render_output_evidence_reports_missing_expected_primitives() {
    let frame = evidence_frame();
    let evidence = UiRenderOutputEvidence::from_frame(
        "runenwerk.ui.render.evidence.button.basic",
        UiRenderOutputProvenance::new("ui_runtime.build_ui_frame", "button.basic"),
        &frame,
        [
            UiExpectedPrimitiveCount::at_least(UiPrimitiveFamily::Rect, 1),
            UiExpectedPrimitiveCount::exactly(UiPrimitiveFamily::GlyphRun, 1),
        ],
    );

    assert!(!evidence.is_valid());
    assert_eq!(
        evidence
            .frame_summary
            .count_for_family(UiPrimitiveFamily::Rect),
        2
    );
    assert!(evidence.diagnostics.iter().any(|diagnostic| {
        diagnostic.kind == UiRenderOutputDiagnosticKind::MissingPrimitiveCount
            && diagnostic.message.contains("glyph-run")
    }));
}

#[test]
fn render_output_evidence_passes_when_expected_counts_match() {
    let frame = evidence_frame();
    let evidence = UiRenderOutputEvidence::from_frame(
        "runenwerk.ui.render.evidence.panel.basic",
        UiRenderOutputProvenance::new("ui_runtime.build_ui_frame", "panel.basic"),
        &frame,
        [
            UiExpectedPrimitiveCount::exactly(UiPrimitiveFamily::Rect, 2),
            UiExpectedPrimitiveCount::exactly(UiPrimitiveFamily::Border, 1),
        ],
    );

    assert!(evidence.is_valid(), "{:?}", evidence.diagnostics);
    assert_eq!(evidence.expected_primitive_counts[0].label(), "rect:2");
    assert_eq!(evidence.expected_primitive_counts[1].label(), "border:1");
}

fn evidence_frame() -> UiFrame {
    let draw_key = UiDrawKey::new(1, None);
    let paint = UiPaint::rgba(0.2, 0.3, 0.4, 1.0);
    let layer = UiLayer::with_primitives(
        UiLayerId(0),
        vec![
            UiPrimitive::Rect(RectPrimitive::new(
                UiRect::new(0.0, 0.0, 100.0, 40.0),
                4.0,
                paint,
                draw_key,
                UiSortKey::new(0, 0, 0),
            )),
            UiPrimitive::Border(BorderPrimitive::new(
                UiRect::new(0.0, 0.0, 100.0, 40.0),
                4.0,
                1.0,
                paint,
                draw_key,
                UiSortKey::new(0, 0, 1),
            )),
            UiPrimitive::Rect(RectPrimitive::new(
                UiRect::new(8.0, 8.0, 48.0, 16.0),
                0.0,
                paint,
                draw_key,
                UiSortKey::new(0, 0, 2),
            )),
        ],
    );

    UiFrame::with_surfaces(vec![UiSurface::with_layers(
        UiSurfaceId(7),
        UiSize::new(128.0, 64.0),
        vec![layer],
    )])
}
