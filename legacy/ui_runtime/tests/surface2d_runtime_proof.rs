use ui_math::UiPoint;
use ui_runtime::{
    Surface2DTransform, base_controls_surface2d_proof_frame, base_controls_surface2d_report,
};

#[test]
fn surface2d_runtime_report_contains_complete_proof_rows() {
    let report = base_controls_surface2d_report();

    assert_eq!(report.proof_id, "base-controls.surface2d.proof");
    assert_eq!(report.descriptor_evidence.len(), 1);
    assert!(report.transform_evidence.len() >= 2);
    assert!(report.navigation_evidence.len() >= 3);
    assert_eq!(report.hover_evidence.len(), 1);
    assert_eq!(report.selection_evidence.len(), 1);
    assert_eq!(report.pointer_capture_evidence.len(), 1);
    assert_eq!(report.gesture_evidence.len(), 1);
    assert!(report.accessibility_input_evidence.len() >= 8);
    assert!(report.budget_evidence.len() >= 9);
    assert!(
        report
            .diagnostic_evidence
            .iter()
            .any(|row| row == "invalid-transform:true")
    );
    assert_eq!(report.catalog_projection_evidence.len(), 1);
    assert_eq!(report.inspection_projection_evidence.len(), 1);
    assert!(report.boundary_counters.clean());
}

#[test]
fn surface2d_transform_maps_both_directions_and_rejects_invalid_zoom() {
    let transform = Surface2DTransform::new(10.0, 20.0, 2.0);
    let screen = transform
        .world_to_screen(UiPoint::new(5.0, 7.0))
        .expect("valid world to screen");
    assert_eq!(screen, UiPoint::new(20.0, 34.0));
    assert_eq!(
        transform.screen_to_world(screen),
        Some(UiPoint::new(5.0, 7.0))
    );

    let invalid = Surface2DTransform::new(0.0, 0.0, 0.0);
    assert!(invalid.world_to_screen(UiPoint::new(1.0, 1.0)).is_none());
    assert!(invalid.screen_to_world(UiPoint::new(1.0, 1.0)).is_none());
}

#[test]
fn surface2d_proof_frame_contains_renderer_neutral_surface_evidence() {
    let proof_frame = base_controls_surface2d_proof_frame();

    assert_eq!(proof_frame.proof_id, "base-controls.surface2d.proof");
    assert!(proof_frame.summary.has_background);
    assert!(proof_frame.summary.has_grid);
    assert!(proof_frame.summary.has_selection_outline);
    assert!(proof_frame.summary.has_diagnostic_overlay);
    assert!(proof_frame.summary.primitive_count >= 15);
    assert_eq!(proof_frame.summary.catalog_rows, 1);
    assert_eq!(proof_frame.summary.inspection_rows, 1);
    assert!(proof_frame.summary.boundary_clean);
    assert_eq!(proof_frame.frame.surfaces.len(), 1);
}
