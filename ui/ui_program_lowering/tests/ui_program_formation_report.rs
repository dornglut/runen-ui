use std::{collections::BTreeMap, fs, path::PathBuf};

use ui_controls::{
    ControlKernelId, ControlKernelSet, ControlKindDescriptor, ControlKindId,
    ControlPackageDescriptor, ControlPackageId, ControlPackageRegistry,
    ControlPackageRegistrySnapshot, ControlPackageVersion, runenwerk_control_package,
};
use ui_definition::{
    AuthoredControlAccessibilityDefinition, AuthoredControlKindId, UiNodeDefinition,
};
use ui_program::UiSchemaRef;
use ui_program_lowering::form_ui_program_report_from_node_with_registry_snapshot;

#[test]
fn formation_report_from_registry_snapshot_passes_for_runenwerk_controls() {
    let node = load_node("assets/ui_gallery/button/selected.ron");

    let registry = ControlPackageRegistry::new()
        .with_package(runenwerk_control_package())
        .expect("runenwerk controls package should register");

    let report = form_ui_program_report_from_node_with_registry_snapshot(
        "ui_gallery.button.selected",
        "assets.ui_gallery.button.selected",
        &node,
        &registry.snapshot(),
    );

    assert!(report.passed(), "{:?}", report.diagnostics);
    assert!(report.diagnostics.is_empty());
    assert!(report.catalog_report.passed());
}

#[test]
fn formation_report_from_registry_snapshot_includes_catalog_derivation_diagnostics() {
    let control_kind_id = "fixture.ui.controls.no-capability";

    let snapshot = snapshot_from_package(no_capability_package(control_kind_id));

    let node = UiNodeDefinition::Control {
        id: "fixture_no_capability".into(),
        kind: AuthoredControlKindId::new(control_kind_id),
        properties: BTreeMap::new(),
        bindings: BTreeMap::new(),
        route: None,
        accessibility: Some(AuthoredControlAccessibilityDefinition {
            role: "button".to_owned(),
            label: Some("No capability".to_owned()),
        }),
        children: Vec::new(),
    };

    let report = form_ui_program_report_from_node_with_registry_snapshot(
        "fixture.ui.no_capability",
        "fixture.ui.no_capability",
        &node,
        &snapshot,
    );

    assert!(!report.passed());
    assert!(report.has_diagnostic("ui.program.catalog.control_kind_missing_activation_capability"));
    assert!(
        report
            .catalog_report
            .skipped_control_kinds
            .iter()
            .any(|kind| kind == control_kind_id)
    );
}

fn no_capability_package(control_kind_id: &str) -> ControlPackageDescriptor {
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
    package
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

fn load_node(relative_repo_path: &str) -> UiNodeDefinition {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|path| path.parent())
        .and_then(|path| path.parent())
        .expect("ui_program_lowering should live under domain/ui/ui_program_lowering")
        .to_path_buf();

    let path = repo_root.join(relative_repo_path);

    let source = fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("failed to read {:?}: {error}", path));

    ron::from_str(&source).expect("fixture should parse as UiNodeDefinition")
}
