use ui_controls::{
    ControlBudgetEvidenceId, ControlKernelKind, ControlMountEligibility, ControlPackageRegistry,
    ControlPackageRegistryError, ControlPackageValidationReason, ControlRenderEvidenceId,
    ControlStoryId, runenwerk_control_package,
};

#[test]
fn control_package_rejects_missing_story_for_mount_eligibility() {
    let mut package = runenwerk_control_package();
    package.control_kinds[0].mount_eligibility = ControlMountEligibility::requires_evidence(
        [],
        [ControlRenderEvidenceId::new(
            "runenwerk.ui.controls.test.render",
        )],
        [ControlBudgetEvidenceId::new(
            "runenwerk.ui.controls.test.budget",
        )],
    );

    assert_reason(
        package,
        ControlPackageValidationReason::MissingMountEvidence,
    );
}

#[test]
fn control_package_rejects_missing_render_evidence_for_mount_eligibility() {
    let mut package = runenwerk_control_package();
    package.control_kinds[0].mount_eligibility = ControlMountEligibility::requires_evidence(
        [ControlStoryId::new("runenwerk.ui.controls.test.story")],
        [],
        [ControlBudgetEvidenceId::new(
            "runenwerk.ui.controls.test.budget",
        )],
    );

    assert_reason(
        package,
        ControlPackageValidationReason::RenderEvidenceMissing,
    );
}

#[test]
fn control_package_rejects_missing_budget_evidence_for_mount_eligibility() {
    let mut package = runenwerk_control_package();
    package.control_kinds[0].mount_eligibility = ControlMountEligibility::requires_evidence(
        [ControlStoryId::new("runenwerk.ui.controls.test.story")],
        [ControlRenderEvidenceId::new(
            "runenwerk.ui.controls.test.render",
        )],
        [],
    );

    assert_reason(
        package,
        ControlPackageValidationReason::BudgetEvidenceMissing,
    );
}

#[test]
fn control_registry_rejects_invalid_package() {
    let mut package = runenwerk_control_package();
    package.property_schemas.clear();

    let error = ControlPackageRegistry::new()
        .with_package(package)
        .expect_err("invalid package should not register");

    assert!(matches!(
        error,
        ControlPackageRegistryError::InvalidPackage { .. }
    ));
}

#[test]
fn control_registry_rejects_duplicate_package() {
    let package = runenwerk_control_package();
    let error = ControlPackageRegistry::new()
        .with_package(package.clone())
        .expect("first package should register")
        .with_package(package)
        .expect_err("duplicate package should not register");

    assert!(matches!(
        error,
        ControlPackageRegistryError::DuplicatePackage { .. }
    ));
}

#[test]
fn control_registry_rejects_duplicate_control_kind_across_packages() {
    let first = runenwerk_control_package();
    let mut second = runenwerk_control_package();
    second.package_id = ui_controls::ControlPackageId::new("runenwerk.ui.controls.copy");

    let error = ControlPackageRegistry::new()
        .with_package(first)
        .expect("first package should register")
        .with_package(second)
        .expect_err("duplicate control kind should not register");

    assert!(matches!(
        error,
        ControlPackageRegistryError::DuplicateControlKind { .. }
    ));
}

#[test]
fn control_registry_snapshot_is_deterministic() {
    let registry = ControlPackageRegistry::new()
        .with_package(runenwerk_control_package())
        .expect("runenwerk package should register");

    let first = registry.snapshot();
    let second = registry.snapshot();

    assert_eq!(first.package_ids(), second.package_ids());
    assert_eq!(first.kernel_ids(), second.kernel_ids());
    assert_eq!(first.control_kinds.len(), 9);
    assert_eq!(first.kernels.len(), 45);
}

#[test]
fn control_kernel_role_validation_reports_wrong_role() {
    let package = runenwerk_control_package();
    let kind = &package.control_kinds[0];
    let mut kernels = package.kernels.clone();
    let layout = kind.kernels.layout.clone();
    let kernel = kernels
        .iter_mut()
        .find(|kernel| kernel.kernel_id == layout)
        .expect("layout kernel should exist");
    kernel.kind = ControlKernelKind::Visual;

    let errors = kind.kernels.validate_required_roles(&kernels);

    assert_eq!(errors.len(), 1);
}

fn assert_reason(
    package: ui_controls::ControlPackageDescriptor,
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
