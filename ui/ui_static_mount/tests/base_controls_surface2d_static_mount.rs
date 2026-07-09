use ui_runtime::base_controls_surface2d_proof_frame;
use ui_static_mount::UiStaticMountReport;

#[test]
fn base_controls_surface2d_static_mount_accepts_runtime_proof_frame() {
    let proof_frame = base_controls_surface2d_proof_frame();

    assert!(proof_frame.summary.has_background);
    assert!(proof_frame.summary.has_grid);
    assert!(proof_frame.summary.has_selection_outline);
    assert!(proof_frame.summary.has_diagnostic_overlay);
    assert!(proof_frame.summary.primitive_count >= 15);
    assert!(proof_frame.summary.boundary_clean);

    let mount_report = UiStaticMountReport::from_frame(proof_frame.frame);

    assert!(mount_report.passed(), "{:?}", mount_report.diagnostics());
    let summary = &mount_report.mounted_frame().expect("mounted frame").summary;
    assert_eq!(summary.surface_count, 1);
    assert!(summary.has_rect_primitive);
    assert!(summary.has_border_primitive);
    assert!(summary.primitive_count >= 15);
    assert!(summary.draw_order_stable);
}
