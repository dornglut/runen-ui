use ui_runtime::{base_controls_generic_text_proof_frame, base_controls_generic_text_report};

#[test]
fn generic_text_report_contains_vertical_evidence_and_boundaries() {
    let report = base_controls_generic_text_report();
    assert!(!report.descriptor_evidence.is_empty());
    assert!(!report.source_block_evidence.is_empty());
    assert!(!report.layout_request_evidence.is_empty());
    assert!(!report.layout_result_evidence.is_empty());
    assert!(!report.line_metric_evidence.is_empty());
    assert!(!report.glyph_run_evidence.is_empty());
    assert!(!report.overflow_evidence.is_empty());
    assert!(!report.catalog_projection_evidence.is_empty());
    assert!(!report.inspection_projection_evidence.is_empty());
    assert!(report.boundary_assertions.no_bypass_evidence());
    assert_eq!(report.boundary_assertions.host_commands_executed, 0);
    assert_eq!(report.boundary_assertions.product_mutations, 0);
    assert_eq!(report.boundary_assertions.authored_ui_edits, 0);
    assert_eq!(report.boundary_assertions.product_undo_redo_operations, 0);
    assert_eq!(report.boundary_assertions.plugin_framework_operations, 0);
    assert_eq!(report.boundary_assertions.renderer_backend_operations, 0);
}

#[test]
fn generic_text_proof_frame_exposes_source_layout_evidence_panels() {
    let proof_frame = base_controls_generic_text_proof_frame();
    assert!(proof_frame.summary.has_source_layout_and_evidence_panels);
    assert!(proof_frame.summary.source_blocks >= 10);
    assert!(proof_frame.summary.inline_spans >= 3);
    assert!(proof_frame.summary.wrapped_lines > 0);
    assert!(proof_frame.summary.aligned_lines >= 2);
    assert!(proof_frame.summary.truncated_lines > 0);
    assert!(proof_frame.summary.fallback_rows >= 1);
    assert!(proof_frame.summary.catalog_rows >= 1);
    assert!(proof_frame.summary.inspection_rows >= 1);
    assert!(proof_frame.summary.no_bypass_proven);
}
