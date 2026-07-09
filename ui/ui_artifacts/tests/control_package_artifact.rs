use ui_artifacts::UiControlPackageArtifact;
use ui_controls::{ControlPackageRegistry, runenwerk_control_package};

#[test]
fn control_package_artifact_exports_registry_snapshot() {
    let registry = ControlPackageRegistry::new()
        .with_package(runenwerk_control_package())
        .expect("runenwerk package should register");
    let artifact = UiControlPackageArtifact::from_registry_snapshot(&registry.snapshot());

    assert!(artifact.validate_contract().is_valid());
    assert_eq!(artifact.manifest.package_ids, ["runenwerk.ui.controls"]);
    assert_eq!(artifact.manifest.control_kind_ids.len(), 9);
    assert_eq!(artifact.manifest.kernel_count, 45);
    assert_eq!(artifact.tables.packages.len(), 1);
    assert_eq!(artifact.tables.control_kinds.len(), 9);
}

#[test]
fn control_package_artifact_is_deterministic() {
    let registry = ControlPackageRegistry::new()
        .with_package(runenwerk_control_package())
        .expect("runenwerk package should register");

    let first = UiControlPackageArtifact::from_registry_snapshot(&registry.snapshot());
    let second = UiControlPackageArtifact::from_registry_snapshot(&registry.snapshot());

    assert_eq!(first.manifest, second.manifest);
    assert_eq!(first.tables, second.tables);
}
