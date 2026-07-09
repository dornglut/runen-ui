use ui_runtime::base_controls_generic_text_proof_frame;
use ui_static_mount::UiStaticMountReport;

#[test]
fn base_controls_generic_text_static_mount_accepts_runtime_proof_frame() {
    let proof_frame = base_controls_generic_text_proof_frame();

    assert!(proof_frame.summary.has_source_layout_and_evidence_panels);
    assert!(proof_frame.summary.source_blocks >= 10);
    assert!(proof_frame.summary.source_runs >= 10);
    assert!(proof_frame.summary.inline_spans >= 3);
    assert!(proof_frame.summary.line_count >= 10);
    assert!(proof_frame.summary.glyph_run_count >= 10);
    assert!(proof_frame.summary.glyph_count > 0);
    assert!(proof_frame.summary.wrapped_lines > 0);
    assert!(proof_frame.summary.aligned_lines >= 2);
    assert!(proof_frame.summary.truncated_lines > 0);
    assert!(proof_frame.summary.fallback_rows >= 1);
    assert!(proof_frame.summary.catalog_rows >= 1);
    assert!(proof_frame.summary.inspection_rows >= 1);
    assert!(proof_frame.summary.no_bypass_proven);

    let mount_report = UiStaticMountReport::from_frame(proof_frame.frame);

    assert!(
        mount_report.passed(),
        "static generic-text proof frame should mount: {:?}",
        mount_report.diagnostics()
    );
    let summary = &mount_report.mounted_frame().expect("mounted frame").summary;
    assert_eq!(summary.surface_count, 1);
    assert!(summary.has_rect_primitive);
    assert!(summary.has_border_primitive);
    assert!(summary.glyph_run_count > 0);
    assert!(summary.draw_order_stable);
}
