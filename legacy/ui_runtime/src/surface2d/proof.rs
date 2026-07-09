use ui_controls::{ControlInspectionSection, SURFACE2D_CONTROL_KIND_ID, runenwerk_control_package};
use ui_math::UiPoint;

use super::{
    BASE_CONTROLS_SURFACE2D_PROOF_ID, Surface2DBoundaryCounters, Surface2DProofReport,
    Surface2DTransform,
};

pub fn base_controls_surface2d_report() -> Surface2DProofReport {
    let package = runenwerk_control_package();
    let catalog = ui_controls::ControlCatalogIndex::from_packages([&package]);
    let inspection = ui_controls::BaseControlsPlugin::new().inspection();
    let descriptor = package
        .surface2d_descriptors
        .iter()
        .find(|descriptor| descriptor.control_kind_id.as_str() == SURFACE2D_CONTROL_KIND_ID)
        .expect("base controls must carry Surface2D descriptor");
    let transform = Surface2DTransform::new(32.0, 48.0, 1.5);
    let invalid_transform = Surface2DTransform::new(0.0, 0.0, 0.0);
    let hover_screen = UiPoint::new(182.0, 198.0);
    let hover_world = transform
        .screen_to_world(hover_screen)
        .expect("valid transform maps screen to world");
    let summary = descriptor.summary();

    Surface2DProofReport {
        proof_id: BASE_CONTROLS_SURFACE2D_PROOF_ID.to_owned(),
        descriptor_evidence: vec![format!(
            "{} inputs:{} layers:{} budgets:{}",
            descriptor.control_kind_id.as_str(),
            descriptor.input_modes.len(),
            descriptor.layer_kinds.len(),
            descriptor.budget_evidence.len()
        )],
        transform_evidence: vec![
            format!(
                "world-to-screen:{:?}",
                transform.world_to_screen(UiPoint::new(10.0, 20.0))
            ),
            format!("screen-to-world:{:.2},{:.2}", hover_world.x, hover_world.y),
        ],
        navigation_evidence: vec![
            format!("pan:{},{}", transform.pan_x, transform.pan_y),
            format!("zoom:{}", transform.zoom),
            "fit-content".to_owned(),
        ],
        hover_evidence: vec![format!("hover:{:.1},{:.1}", hover_world.x, hover_world.y)],
        selection_evidence: vec!["selection-box".to_owned()],
        pointer_capture_evidence: vec!["pointer-capture".to_owned()],
        gesture_evidence: vec!["gesture-cancel-commit".to_owned()],
        accessibility_input_evidence: summary
            .input_modes
            .iter()
            .map(|mode| format!("input:{mode}"))
            .collect(),
        budget_evidence: summary
            .budget_evidence
            .iter()
            .map(|budget| format!("budget:{budget}"))
            .collect(),
        diagnostic_evidence: vec![format!(
            "invalid-transform:{}",
            invalid_transform
                .world_to_screen(UiPoint::new(1.0, 1.0))
                .is_none()
        )],
        catalog_projection_evidence: catalog
            .entries
            .iter()
            .filter(|entry| entry.surface2d_supported)
            .map(|entry| {
                format!(
                    "{}:{}",
                    entry.control_kind_id,
                    entry.surface2d_input_modes.len()
                )
            })
            .collect(),
        inspection_projection_evidence: inspection
            .controls
            .iter()
            .filter_map(|control| {
                control
                    .fact(ControlInspectionSection::Surface2D, "surface2d.supported")
                    .filter(|value| *value == "true")
                    .map(|value| format!("{}:{value}", control.control_kind_id))
            })
            .collect(),
        static_mount_expectations: vec![
            "surface".to_owned(),
            "background".to_owned(),
            "grid".to_owned(),
            "selection".to_owned(),
            "diagnostic".to_owned(),
            "stable-order".to_owned(),
        ],
        boundary_counters: Surface2DBoundaryCounters::default(),
    }
}
