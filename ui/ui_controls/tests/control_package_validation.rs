use ui_controls::{
    ControlPackageDescriptor, ControlPackageValidationReason, runenwerk_control_package,
};

#[test]
fn control_package_complete_contract_validates() {
    let package = runenwerk_control_package();
    let report = package.validate_contract();
    assert!(report.is_valid(), "{:?}", report.diagnostics);
    assert_eq!(package.control_kinds.len(), 9);
    assert_eq!(package.property_schemas.len(), 9);
    assert_eq!(package.state_schemas.len(), 9);
    assert_eq!(package.event_payload_schemas.len(), 9);
    assert_eq!(package.kernels.len(), 45);
    assert_eq!(package.fixtures.len(), 9);
    assert_eq!(package.diagnostics.len(), 9);
    assert_eq!(package.migrations.len(), 9);
    assert_eq!(package.stories.len(), 9);
    assert_eq!(package.interaction_descriptors.len(), 9);
    assert_eq!(package.overlay_descriptors.len(), 9);
    assert_eq!(package.editable_text_descriptors.len(), 1);
    assert_eq!(package.generic_text_descriptors.len(), 7);
    assert_eq!(package.surface2d_descriptors.len(), 1);
}

#[test]
fn control_package_rejects_missing_property_schema() {
    let mut package = runenwerk_control_package();
    package.property_schemas.clear();
    assert_has_reason(package, ControlPackageValidationReason::MissingSchema);
}

#[test]
fn control_package_rejects_missing_state_schema() {
    let mut package = runenwerk_control_package();
    package.state_schemas.clear();
    assert_has_reason(package, ControlPackageValidationReason::MissingSchema);
}

#[test]
fn control_package_rejects_missing_event_payload_schema() {
    let mut package = runenwerk_control_package();
    package.event_payload_schemas.clear();
    assert_has_reason(package, ControlPackageValidationReason::MissingSchema);
}

#[test]
fn control_package_rejects_missing_layout_kernel() {
    let mut package = runenwerk_control_package();
    let missing = package.control_kinds[0].kernels.layout.clone();
    package.kernels.retain(|kernel| kernel.kernel_id != missing);
    assert_has_reason(package, ControlPackageValidationReason::MissingKernel);
}

#[test]
fn control_package_rejects_missing_interaction_kernel() {
    let mut package = runenwerk_control_package();
    let missing = package.control_kinds[0].kernels.interaction.clone();
    package.kernels.retain(|kernel| kernel.kernel_id != missing);
    assert_has_reason(package, ControlPackageValidationReason::MissingKernel);
}

#[test]
fn control_package_rejects_missing_visual_kernel() {
    let mut package = runenwerk_control_package();
    let missing = package.control_kinds[0].kernels.visual.clone();
    package.kernels.retain(|kernel| kernel.kernel_id != missing);
    assert_has_reason(package, ControlPackageValidationReason::MissingKernel);
}

#[test]
fn control_package_rejects_missing_accessibility_kernel() {
    let mut package = runenwerk_control_package();
    let missing = package.control_kinds[0].kernels.accessibility.clone();
    package.kernels.retain(|kernel| kernel.kernel_id != missing);
    assert_has_reason(package, ControlPackageValidationReason::MissingKernel);
}

#[test]
fn control_package_rejects_missing_inspection_kernel() {
    let mut package = runenwerk_control_package();
    let missing = package.control_kinds[0].kernels.inspection.clone();
    package.kernels.retain(|kernel| kernel.kernel_id != missing);
    assert_has_reason(package, ControlPackageValidationReason::MissingKernel);
}

#[test]
fn control_package_rejects_duplicate_schema_ref() {
    let mut package = runenwerk_control_package();
    package
        .property_schemas
        .push(package.property_schemas[0].clone());
    assert_has_reason(package, ControlPackageValidationReason::DuplicateSchemaRef);
}

#[test]
fn control_package_rejects_duplicate_kernel_id() {
    let mut package = runenwerk_control_package();
    package.kernels.push(package.kernels[0].clone());
    assert_has_reason(package, ControlPackageValidationReason::DuplicateKernelId);
}

#[test]
fn control_package_rejects_duplicate_fixture_id() {
    let mut package = runenwerk_control_package();
    package.fixtures.push(package.fixtures[0].clone());
    assert_has_reason(package, ControlPackageValidationReason::DuplicateFixtureId);
}

#[test]
fn control_package_rejects_duplicate_diagnostic_id() {
    let mut package = runenwerk_control_package();
    package.diagnostics.push(package.diagnostics[0].clone());
    assert_has_reason(
        package,
        ControlPackageValidationReason::DuplicateDiagnosticId,
    );
}

#[test]
fn control_package_rejects_duplicate_migration_id() {
    let mut package = runenwerk_control_package();
    package.migrations.push(package.migrations[0].clone());
    assert_has_reason(
        package,
        ControlPackageValidationReason::DuplicateMigrationId,
    );
}

#[test]
fn control_package_rejects_duplicate_story_id() {
    let mut package = runenwerk_control_package();
    package.stories.push(package.stories[0].clone());
    assert_has_reason(package, ControlPackageValidationReason::DuplicateStoryId);
}

#[test]
fn control_package_rejects_duplicate_interaction_descriptor() {
    let mut package = runenwerk_control_package();
    package
        .interaction_descriptors
        .push(package.interaction_descriptors[0].clone());
    assert_has_reason(
        package,
        ControlPackageValidationReason::DuplicateInteractionDescriptor,
    );
}

#[test]
fn control_package_rejects_duplicate_generic_text_descriptor() {
    let mut package = runenwerk_control_package();
    package
        .generic_text_descriptors
        .push(package.generic_text_descriptors[0].clone());
    assert_has_reason(
        package,
        ControlPackageValidationReason::DuplicateGenericTextDescriptor,
    );
}

#[test]
fn control_package_rejects_duplicate_surface2d_descriptor() {
    let mut package = runenwerk_control_package();
    package
        .surface2d_descriptors
        .push(package.surface2d_descriptors[0].clone());
    assert_has_reason(
        package,
        ControlPackageValidationReason::DuplicateSurface2DDescriptor,
    );
}

#[test]
fn runenwerk_control_package_validates() {
    assert!(runenwerk_control_package().validate_contract().is_valid());
}

fn assert_has_reason(package: ControlPackageDescriptor, reason: ControlPackageValidationReason) {
    let report = package.validate_contract();
    assert!(!report.is_valid(), "package unexpectedly valid");
    assert!(
        report.has_reason(reason),
        "expected reason {:?}, got {:?}",
        reason,
        report.diagnostics
    );
}
