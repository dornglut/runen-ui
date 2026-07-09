use ui_runtime::{
    base_controls_text_editing_fixture, base_controls_text_editing_positive_script,
    base_controls_text_editing_proof_frame, replay_text_editing,
};
use ui_static_mount::UiStaticMountReport;

#[test]
fn base_controls_text_editing_static_mount_accepts_runtime_proof_frame() {
    let fixture = base_controls_text_editing_fixture();
    let script = base_controls_text_editing_positive_script();
    let report = replay_text_editing(&fixture, &script);
    let proof_frame = base_controls_text_editing_proof_frame(report);

    assert!(proof_frame.summary.has_main_inspector_and_report);
    assert!(proof_frame.summary.descriptor_rows >= 1);
    assert!(proof_frame.summary.accepted_intent_rows >= 8);
    assert!(proof_frame.summary.suppressed_intent_rows >= 1);
    assert!(proof_frame.summary.value_rows >= 1);
    assert!(proof_frame.summary.caret_rows >= 1);
    assert!(proof_frame.summary.selection_rows >= 1);
    assert!(proof_frame.summary.composition_rows >= 1);
    assert!(proof_frame.summary.no_bypass_proven);

    let mount_report = UiStaticMountReport::from_frame(proof_frame.frame);

    assert!(
        mount_report.passed(),
        "static text-editing proof frame should mount: {:?}",
        mount_report.diagnostics()
    );
    let summary = &mount_report.mounted_frame().expect("mounted frame").summary;
    assert_eq!(summary.surface_count, 1);
    assert!(summary.has_rect_primitive);
    assert!(summary.has_border_primitive);
    assert!(summary.glyph_run_count > 0);
    assert!(summary.draw_order_stable);
}
