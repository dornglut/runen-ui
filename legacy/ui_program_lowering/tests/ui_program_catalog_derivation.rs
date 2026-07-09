use ui_controls::{
    BUTTON_CONTROL_KIND_ID, ControlKernelId, ControlKernelKind, ControlKernelSet,
    ControlKindDescriptor, ControlKindId, ControlPackageDescriptor, ControlPackageId,
    ControlPackageRegistry, ControlPackageRegistrySnapshot, ControlPackageVersion,
    runenwerk_control_package,
};
use ui_program::UiSchemaRef;
use ui_program_lowering::UiProgramFormationControlCatalog;

#[test]
fn runenwerk_control_package_derives_catalog_without_diagnostics() {
    let registry = ControlPackageRegistry::new()
        .with_package(runenwerk_control_package())
        .expect("runenwerk controls package should register");

    let report = UiProgramFormationControlCatalog::derive_from_control_package_registry_snapshot(
        &registry.snapshot(),
    );

    assert!(report.passed(), "{:?}", report.diagnostics);
    assert!(report.diagnostics.is_empty());
    assert!(report.skipped_control_kinds.is_empty());
}

#[test]
fn button_catalog_contract_exposes_resolved_property_schema() {
    let registry = ControlPackageRegistry::new()
        .with_package(runenwerk_control_package())
        .expect("runenwerk controls package should register");

    let report = UiProgramFormationControlCatalog::derive_from_control_package_registry_snapshot(
        &registry.snapshot(),
    );

    assert!(report.passed(), "{:?}", report.diagnostics);

    let contract = report
        .catalog
        .control_kind(BUTTON_CONTROL_KIND_ID)
        .expect("button contract should be present");

    assert_eq!(
        contract.property_schema.schema_ref.id.as_str(),
        "runenwerk.ui.controls.button.properties"
    );
    assert_eq!(contract.property_schema.schema_ref.version.value(), 1);
    assert!(contract.property_schema.fields.contains_key("label"));
}

#[test]
fn catalog_derivation_reports_control_kind_without_activation_capability() {
    let control_kind_id = "fixture.ui.controls.no-capability";

    let kind = ControlKindDescriptor::new(
        ControlKindId::new(control_kind_id),
        "NoCapability",
        UiSchemaRef::new("fixture.ui.controls.no_capability.properties", 1),
        UiSchemaRef::new("fixture.ui.controls.no_capability.state", 1),
        UiSchemaRef::new("fixture.ui.controls.no_capability.event", 1),
        ControlKernelSet::new(
            ControlKernelId::new("fixture.ui.controls.no_capability.layout"),
            ControlKernelId::new("fixture.ui.controls.no_capability.interaction"),
            ControlKernelId::new("fixture.ui.controls.no_capability.visual"),
            ControlKernelId::new("fixture.ui.controls.no_capability.accessibility"),
            ControlKernelId::new("fixture.ui.controls.no_capability.inspection"),
        ),
    );

    let mut package = ControlPackageDescriptor::new(
        ControlPackageId::new("fixture.ui.controls"),
        ControlPackageVersion::new(1),
    );
    package.control_kinds.push(kind);

    let report = UiProgramFormationControlCatalog::derive_from_control_package_registry_snapshot(
        &snapshot_from_package(package),
    );

    assert!(!report.passed());
    assert!(report.has_diagnostic("ui.program.catalog.control_kind_missing_activation_capability"));
    assert_eq!(report.skipped_control_kinds, [control_kind_id.to_owned()]);
}

#[test]
fn catalog_derivation_rejects_missing_referenced_property_schema() {
    let mut package = runenwerk_control_package();
    let kind = button_kind(&package).clone();
    package
        .property_schemas
        .retain(|schema| schema.schema_ref() != &kind.property_schema);

    assert_catalog_rejects_button_kind(package, "ui.program.catalog.missing_property_schema");
}

#[test]
fn catalog_derivation_rejects_missing_referenced_state_schema() {
    let mut package = runenwerk_control_package();
    let kind = button_kind(&package).clone();
    package
        .state_schemas
        .retain(|schema| schema.schema_ref() != &kind.state_schema);

    assert_catalog_rejects_button_kind(package, "ui.program.catalog.missing_state_schema");
}

#[test]
fn catalog_derivation_rejects_missing_referenced_event_payload_schema() {
    let mut package = runenwerk_control_package();
    let kind = button_kind(&package).clone();
    package
        .event_payload_schemas
        .retain(|schema| schema.schema_ref() != &kind.event_payload_schema);

    assert_catalog_rejects_button_kind(package, "ui.program.catalog.missing_event_payload_schema");
}

#[test]
fn catalog_derivation_rejects_missing_referenced_layout_kernel() {
    let mut package = runenwerk_control_package();
    let kind = button_kind(&package).clone();
    package
        .kernels
        .retain(|kernel| kernel.kernel_id != kind.kernels.layout);

    assert_catalog_rejects_button_kind(package, "ui.program.catalog.missing_layout_kernel");
}

#[test]
fn catalog_derivation_rejects_missing_referenced_interaction_kernel() {
    let mut package = runenwerk_control_package();
    let kind = button_kind(&package).clone();
    package
        .kernels
        .retain(|kernel| kernel.kernel_id != kind.kernels.interaction);

    assert_catalog_rejects_button_kind(package, "ui.program.catalog.missing_interaction_kernel");
}

#[test]
fn catalog_derivation_rejects_missing_referenced_visual_kernel() {
    let mut package = runenwerk_control_package();
    let kind = button_kind(&package).clone();
    package
        .kernels
        .retain(|kernel| kernel.kernel_id != kind.kernels.visual);

    assert_catalog_rejects_button_kind(package, "ui.program.catalog.missing_visual_kernel");
}

#[test]
fn catalog_derivation_rejects_missing_referenced_accessibility_kernel() {
    let mut package = runenwerk_control_package();
    let kind = button_kind(&package).clone();
    package
        .kernels
        .retain(|kernel| kernel.kernel_id != kind.kernels.accessibility);

    assert_catalog_rejects_button_kind(package, "ui.program.catalog.missing_accessibility_kernel");
}

#[test]
fn catalog_derivation_rejects_missing_referenced_inspection_kernel() {
    let mut package = runenwerk_control_package();
    let kind = button_kind(&package).clone();
    package
        .kernels
        .retain(|kernel| kernel.kernel_id != kind.kernels.inspection);

    assert_catalog_rejects_button_kind(package, "ui.program.catalog.missing_inspection_kernel");
}

#[test]
fn catalog_derivation_rejects_referenced_kernel_kind_mismatch() {
    let mut package = runenwerk_control_package();
    let kind = button_kind(&package).clone();
    let layout_kernel = package
        .kernels
        .iter_mut()
        .find(|kernel| kernel.kernel_id == kind.kernels.layout)
        .expect("button layout kernel should exist");

    layout_kernel.kind = ControlKernelKind::Visual;

    assert_catalog_rejects_button_kind(package, "ui.program.catalog.kernel_kind_mismatch");
}

fn assert_catalog_rejects_button_kind(package: ControlPackageDescriptor, diagnostic_code: &str) {
    let report = UiProgramFormationControlCatalog::derive_from_control_package_registry_snapshot(
        &snapshot_from_package(package),
    );

    assert!(!report.passed());
    assert!(report.has_diagnostic(diagnostic_code));
    assert!(
        report
            .skipped_control_kinds
            .iter()
            .any(|kind| kind == BUTTON_CONTROL_KIND_ID)
    );
    assert!(!report.catalog.contains_control_kind(BUTTON_CONTROL_KIND_ID));
    assert!(report.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == diagnostic_code
            && diagnostic.message.contains("runenwerk.ui.controls")
            && diagnostic.message.contains(BUTTON_CONTROL_KIND_ID)
    }));
}

fn button_kind(package: &ControlPackageDescriptor) -> &ControlKindDescriptor {
    package
        .control_kind(&ControlKindId::new(BUTTON_CONTROL_KIND_ID))
        .expect("runenwerk package should contain button control kind")
}

fn snapshot_from_package(package: ControlPackageDescriptor) -> ControlPackageRegistrySnapshot {
    ControlPackageRegistrySnapshot {
        packages: vec![package.clone()],
        control_kinds: package.control_kinds.clone(),
        schemas: package
            .property_schemas
            .iter()
            .chain(package.state_schemas.iter())
            .chain(package.event_payload_schemas.iter())
            .cloned()
            .collect(),
        kernels: package.kernels.clone(),
        fixtures: package.fixtures.clone(),
        diagnostics: package.diagnostics.clone(),
        migrations: package.migrations.clone(),
        stories: package.stories.clone(),
        route_requirements: package.route_requirements.clone(),
        target_profiles: package.target_profiles.clone(),
    }
}
