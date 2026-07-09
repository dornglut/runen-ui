use ui_runtime::{
    base_controls_overlay_layering_fixture, base_controls_overlay_layering_positive_script,
    base_controls_overlay_layering_proof_frame, replay_overlay_layering,
};
use ui_static_mount::UiStaticMountReport;

#[test]
fn base_controls_overlay_layering_static_mount_accepts_runtime_overlay_proof_frame() {
    let fixture = base_controls_overlay_layering_fixture();
    let script = base_controls_overlay_layering_positive_script();
    let report = replay_overlay_layering(&fixture, &script);
    let proof_frame = base_controls_overlay_layering_proof_frame(report);

    assert!(proof_frame.summary.has_main_inspector_and_report);
    assert!(proof_frame.summary.anchor_rows >= 8);
    assert!(proof_frame.summary.stack_rows >= 4);
    assert!(proof_frame.summary.placement_rows >= 4);

    let mount_report = UiStaticMountReport::from_frame(proof_frame.frame);

    assert!(
        mount_report.passed(),
        "static overlay proof frame should mount: {:?}",
        mount_report.diagnostics()
    );
    let summary = &mount_report.mounted_frame().expect("mounted frame").summary;
    assert_eq!(summary.surface_count, 1);
    assert!(summary.has_rect_primitive);
    assert!(summary.has_border_primitive);
    assert!(summary.glyph_run_count > 0);
    assert!(summary.draw_order_stable);
}
