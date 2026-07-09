use ui_controls::{ControlKindId, ControlPackageValidationReason, runenwerk_control_package};

#[test]
fn base_controls_package_exposes_overlay_descriptors_for_all_controls() {
    let package = runenwerk_control_package();
    assert_eq!(package.control_kinds.len(), 9);
    assert_eq!(package.overlay_descriptors.len(), 9);
    assert!(package.validate_contract().is_valid());
    for kind in &package.control_kinds {
        let descriptor = package
            .overlay_descriptor(&kind.control_kind_id)
            .expect("every base control kind exposes package-backed overlay support");
        assert!(!descriptor.requirements.is_empty());
        assert_eq!(descriptor.control_kind_id, kind.control_kind_id);
    }
}

#[test]
fn base_controls_package_rejects_duplicate_overlay_descriptor() {
    let mut package = runenwerk_control_package();
    package
        .overlay_descriptors
        .push(package.overlay_descriptors[0].clone());
    assert_has_reason(
        package,
        ControlPackageValidationReason::DuplicateOverlayDescriptor,
    );
}

#[test]
fn base_controls_package_rejects_unresolved_overlay_descriptor() {
    let mut package = runenwerk_control_package();
    package.overlay_descriptors[0].control_kind_id = ControlKindId::new("runenwerk.ui.missing");
    assert_has_reason(
        package,
        ControlPackageValidationReason::UnresolvedOverlayDescriptor,
    );
}

#[test]
fn base_controls_package_rejects_empty_overlay_requirements() {
    let mut package = runenwerk_control_package();
    package.overlay_descriptors[0].requirements.clear();
    assert_has_reason(
        package,
        ControlPackageValidationReason::InvalidOverlayDescriptor,
    );
}

fn assert_has_reason(
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
