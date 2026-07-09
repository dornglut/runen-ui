use ui_controls::{
    BaseControlsPlugin, ControlCatalogIndex, ControlInspectionSection, ControlKindId,
    ControlPackageDescriptor, ControlPackageValidationReason, ControlSurface2DBudgetEvidenceKind,
    ControlSurface2DDescriptor, ControlSurface2DInputMode, SURFACE2D_CONTROL_KIND_ID,
    runenwerk_control_package,
};

#[test]
fn surface2d_descriptor_is_package_backed() {
    let package = runenwerk_control_package();
    let descriptor = package
        .surface2d_descriptor(&ControlKindId::new(SURFACE2D_CONTROL_KIND_ID))
        .expect("Surface2D descriptor");

    assert!(descriptor.proof_required);
    assert!(!descriptor.renderer_backend_required);
    assert!(!descriptor.executes_host_commands);
    assert!(!descriptor.mutates_product_state);
    assert!(!descriptor.graph_or_timeline_semantics);
    assert!(descriptor.accessibility.is_complete());
    assert!(descriptor.interaction.is_complete());
    assert!(descriptor.input_modes.len() >= 8);
    assert!(descriptor.layer_kinds.len() >= 4);
    assert!(descriptor.budget_evidence.len() >= 9);
}

#[test]
fn catalog_projects_surface2d_support_without_product_mutation() {
    let package = runenwerk_control_package();
    let catalog = ControlCatalogIndex::from_packages([&package]);
    let surface = catalog
        .entry(SURFACE2D_CONTROL_KIND_ID)
        .expect("Surface2D catalog entry");

    assert!(surface.surface2d_supported);
    assert!(
        surface
            .surface2d_input_modes
            .contains(&"keyboard-pan".to_owned())
    );
    assert!(
        surface
            .surface2d_input_modes
            .contains(&"pointer-capture".to_owned())
    );
    assert!(surface.surface2d_layers.contains(&"grid".to_owned()));
    assert!(
        surface
            .surface2d_layers
            .contains(&"diagnostic-overlay".to_owned())
    );
    assert!(
        surface
            .surface2d_budget_evidence
            .contains(&"primitive-count".to_owned())
    );
    assert!(surface.surface2d_accessibility_complete);
    assert!(surface.surface2d_interaction_complete);
    assert!(!surface.surface2d_graph_or_timeline_semantics);
    assert!(!surface.renderer_backend_required);
    assert!(!surface.executes_host_commands);
    assert!(!surface.mutates_product_state);
}

#[test]
fn inspection_projects_surface2d_as_separate_section() {
    let inspection = BaseControlsPlugin::new().inspection();
    let surface = inspection
        .controls
        .iter()
        .find(|control| control.control_kind_id == SURFACE2D_CONTROL_KIND_ID)
        .expect("Surface2D inspection");

    assert_eq!(
        surface.fact(ControlInspectionSection::Surface2D, "surface2d.supported"),
        Some("true")
    );
    assert_eq!(
        surface.fact(
            ControlInspectionSection::Surface2D,
            "surface2d.renderer_backend_required"
        ),
        Some("false")
    );
    assert_eq!(
        surface.fact(ControlInspectionSection::TextDisplay, "surface2d.supported"),
        None
    );
}

#[test]
fn surface2d_descriptor_summary_stays_renderer_and_product_neutral() {
    let summary =
        ControlSurface2DDescriptor::new(ControlKindId::new(SURFACE2D_CONTROL_KIND_ID)).summary();

    assert!(summary.surface2d_supported);
    assert!(summary.accessibility_complete);
    assert!(summary.interaction_complete);
    assert!(!summary.renderer_backend_required);
    assert!(!summary.executes_host_commands);
    assert!(!summary.mutates_product_state);
    assert!(!summary.graph_or_timeline_semantics);
}

#[test]
fn surface2d_validation_rejects_unresolved_control_kind() {
    let mut package = runenwerk_control_package();
    package
        .surface2d_descriptors
        .push(ControlSurface2DDescriptor::new(ControlKindId::new(
            "runenwerk.ui.controls.missing-surface2d",
        )));

    assert_surface2d_reason(
        package,
        ControlPackageValidationReason::UnresolvedSurface2DDescriptor,
    );
}

#[test]
fn surface2d_validation_rejects_renderer_backend_requirement() {
    let mut package = runenwerk_control_package();
    package.surface2d_descriptors[0].renderer_backend_required = true;

    assert_surface2d_reason(
        package,
        ControlPackageValidationReason::InvalidSurface2DDescriptor,
    );
}

#[test]
fn surface2d_validation_rejects_product_mutation() {
    let mut package = runenwerk_control_package();
    package.surface2d_descriptors[0].mutates_product_state = true;

    assert_surface2d_reason(
        package,
        ControlPackageValidationReason::InvalidSurface2DDescriptor,
    );
}

#[test]
fn surface2d_validation_rejects_missing_required_input_mode() {
    let mut package = runenwerk_control_package();
    package.surface2d_descriptors[0]
        .input_modes
        .retain(|mode| *mode != ControlSurface2DInputMode::PointerCapture);

    assert_surface2d_reason(
        package,
        ControlPackageValidationReason::UnsupportedSurface2DDescriptor,
    );
}

#[test]
fn surface2d_validation_rejects_missing_budget_evidence() {
    let mut package = runenwerk_control_package();
    package.surface2d_descriptors[0]
        .budget_evidence
        .retain(|kind| *kind != ControlSurface2DBudgetEvidenceKind::PrimitiveCount);

    assert_surface2d_reason(
        package,
        ControlPackageValidationReason::UnsupportedSurface2DDescriptor,
    );
}

fn assert_surface2d_reason(
    package: ControlPackageDescriptor,
    reason: ControlPackageValidationReason,
) {
    let report = package.validate_contract();
    assert!(!report.is_valid(), "package unexpectedly valid");
    assert!(
        report.has_reason(reason),
        "expected reason {:?}, got {:?}",
        reason,
        report.diagnostics
    );
}
