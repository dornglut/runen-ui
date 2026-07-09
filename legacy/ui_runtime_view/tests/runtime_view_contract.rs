use std::{fs, path::PathBuf};

use ui_artifacts::{BindingSnapshotRow, UiRuntimeArtifact, UiRuntimeArtifactDiagnostic};
use ui_compiler::UiCompiler;
use ui_controls::{ControlPackageRegistry, runenwerk_control_package};
use ui_definition::UiNodeDefinition;
use ui_program::{
    BindingEdge, BindingEdgeId, BindingEndpoint, BindingEndpointId, ControlNodeId,
    ControlPropertySnapshotId, StateRequirementId, UiSchemaRef, UiSchemaValue,
};
use ui_program_lowering::form_ui_program_report_from_node_with_registry_snapshot;
use ui_runtime_view::{
    ButtonRuntimeHostData, ButtonRuntimeViewReport,
    DIAGNOSTIC_BINDING_MISSING_CONTROL_PROPERTY_OWNER,
    DIAGNOSTIC_BINDING_MISSING_STATE_REQUIREMENT, DIAGNOSTIC_CONTROL_DUPLICATE_PROPERTY_SNAPSHOTS,
    DIAGNOSTIC_CONTROL_MISSING_PROPERTY_SNAPSHOT, DIAGNOSTIC_PROPERTY_MISSING_OWNER_CONTROL,
    UiRuntimeView,
};

#[test]
fn runtime_view_projects_compiled_button_artifact() {
    let artifact = compiled_selected_button_artifact();
    let view = UiRuntimeView::from_artifact(&artifact);

    assert!(view.passed(), "{:?}", view.diagnostics);
    assert_eq!(view.controls.len(), 1);

    let control = &view.controls[0];
    assert_eq!(control.control_id().as_str(), "control.button_selected");
    assert!(control.control.source_map_index.is_some());

    let property = control
        .property()
        .expect("compiled button should have property row");
    assert_eq!(
        property
            .snapshot
            .value
            .get("label")
            .and_then(UiSchemaValue::as_str),
        Some("Selected")
    );
    assert_eq!(
        property
            .snapshot
            .value
            .get("variant")
            .and_then(UiSchemaValue::as_str),
        Some("secondary")
    );
    assert_eq!(
        property.snapshot.schema.id.as_str(),
        "runenwerk.ui.controls.button.properties"
    );
    assert_eq!(property.snapshot.schema.version.value(), 1);
    assert!(property.source_map_index.is_some());

    assert_eq!(control.layout.len(), 1);
    assert_eq!(control.style.len(), 1);
    assert_eq!(control.state.len(), 1);
    assert_eq!(control.interaction.len(), 1);
    assert_eq!(control.binding_snapshots.len(), 1);
    assert_eq!(control.visual.len(), 1);
    assert_eq!(control.accessibility.len(), 1);
    assert_eq!(control.inspection.len(), 1);
}

#[test]
fn runtime_view_reports_missing_property_snapshot() {
    let mut artifact = compiled_selected_button_artifact();
    artifact.tables.properties.rows.clear();

    let view = UiRuntimeView::from_artifact(&artifact);

    assert!(!view.passed());
    assert_has_diagnostic(&view, DIAGNOSTIC_CONTROL_MISSING_PROPERTY_SNAPSHOT);
}

#[test]
fn runtime_view_reports_duplicate_property_snapshots() {
    let mut artifact = compiled_selected_button_artifact();
    let mut duplicate = artifact.tables.properties.rows[0].clone();
    duplicate.snapshot.snapshot_id = ControlPropertySnapshotId::new("properties.button_duplicate");
    artifact.tables.properties.rows.push(duplicate);

    let view = UiRuntimeView::from_artifact(&artifact);

    assert!(!view.passed());
    assert_has_diagnostic(&view, DIAGNOSTIC_CONTROL_DUPLICATE_PROPERTY_SNAPSHOTS);
}

#[test]
fn runtime_view_reports_orphan_property_snapshot() {
    let mut artifact = compiled_selected_button_artifact();
    let mut orphan = artifact.tables.properties.rows[0].clone();
    orphan.snapshot.snapshot_id = ControlPropertySnapshotId::new("properties.orphan");
    orphan.snapshot.owner_control = ControlNodeId::new("control.missing");
    artifact.tables.properties.rows.push(orphan);

    let view = UiRuntimeView::from_artifact(&artifact);

    assert!(!view.passed());
    assert_has_diagnostic(&view, DIAGNOSTIC_PROPERTY_MISSING_OWNER_CONTROL);
}

#[test]
fn runtime_view_report_fails_when_artifact_manifest_has_error_diagnostic() {
    let mut artifact = compiled_selected_button_artifact();
    artifact
        .manifest
        .push_diagnostic(UiRuntimeArtifactDiagnostic::error(
            "fixture.artifact.error",
            "fixture artifact error",
        ));

    let report = UiRuntimeView::from_artifact_report(&artifact);
    let view = UiRuntimeView::from_artifact(&artifact);

    assert!(!report.passed());
    assert!(!report.artifact_passed());
    assert!(report.view_passed(), "{:?}", report.view.diagnostics);
    assert!(!view.passed());
    assert!(!view.artifact_passed());
    assert!(view.view_passed(), "{:?}", view.diagnostics);
    assert_eq!(report.artifact_diagnostics.len(), 1);
    assert_eq!(view.artifact_diagnostics.len(), 1);
}

#[test]
fn runtime_view_joins_bindings_through_state_owner() {
    let artifact = compiled_selected_button_artifact();
    let view = UiRuntimeView::from_artifact(&artifact);
    let control = view
        .control(&ControlNodeId::new("control.button_selected"))
        .expect("selected button control should be projected");

    assert_eq!(control.binding_snapshots.len(), 1);
    assert_eq!(
        control.binding_snapshots[0].binding.edge_id.as_str(),
        "binding.button_selected.selected"
    );
}

#[test]
fn button_runtime_view_report_projects_route_state_accessibility_and_style_axes() {
    let artifact = compiled_selected_button_artifact();
    let runtime_report = UiRuntimeView::from_artifact_report(&artifact);
    let button_report = ButtonRuntimeViewReport::from_runtime_view_report_with_host_data(
        &runtime_report,
        &ButtonRuntimeHostData::new().with_bool("ui_gallery.button.selected.active", true),
    );

    assert!(
        runtime_report.passed(),
        "{:?}",
        runtime_report.view.diagnostics
    );
    assert!(button_report.passed(), "{:?}", button_report.diagnostics);
    assert_eq!(button_report.buttons.len(), 1);

    let button = &button_report.buttons[0];
    assert_eq!(button.control_id, "control.button_selected");
    assert_eq!(button.label, "Selected");
    assert_eq!(
        button.route.as_deref(),
        Some("ui_gallery.button.selected.activate")
    );
    assert_eq!(
        button.capability.as_deref(),
        Some("runenwerk.ui.controls.activate")
    );
    assert!(button.selected);
    assert!(!button.disabled);
    assert_eq!(
        button.selected_host_endpoint.as_deref(),
        Some("ui_gallery.button.selected.active")
    );
    assert_eq!(
        button.accessibility_label.as_deref(),
        Some("Selected demo button")
    );
    assert_eq!(button.style_axes.variant, "secondary");
    assert_eq!(button.style_axes.tone, "neutral");
    assert_eq!(button.style_axes.density, "normal");
    assert_eq!(button.style_axes.size, "md");
    assert!(button.source_map_indexes.control.is_some());
    assert!(button.source_map_indexes.property.is_some());
    assert!(button.source_map_indexes.state.is_some());
    assert!(button.source_map_indexes.interaction.is_some());
    assert!(button.source_map_indexes.accessibility.is_some());
}

#[test]
fn runtime_view_reports_binding_missing_state_requirement() {
    let mut artifact = compiled_selected_button_artifact();
    artifact.tables.binding_snapshots.rows[0].binding.target = BindingEndpoint::UiState {
        requirement_id: StateRequirementId::new("state.button_selected.missing"),
        endpoint_id: BindingEndpointId::new("state.button_selected.missing"),
    };

    let view = UiRuntimeView::from_artifact(&artifact);

    assert!(!view.passed());
    assert_has_diagnostic(&view, DIAGNOSTIC_BINDING_MISSING_STATE_REQUIREMENT);
}

#[test]
fn runtime_view_reports_binding_missing_control_property_owner() {
    let mut artifact = compiled_selected_button_artifact();
    artifact
        .tables
        .binding_snapshots
        .rows
        .push(control_property_binding_row(
            "binding.button_missing.label",
            "host.button_missing.label",
            "control.missing",
        ));

    let view = UiRuntimeView::from_artifact(&artifact);

    assert!(!view.passed());
    assert_has_diagnostic(&view, DIAGNOSTIC_BINDING_MISSING_CONTROL_PROPERTY_OWNER);
}

#[test]
fn runtime_view_joins_bindings_through_control_property_owner() {
    let mut artifact = compiled_selected_button_artifact();
    artifact
        .tables
        .binding_snapshots
        .rows
        .push(control_property_binding_row(
            "binding.button_selected.label",
            "host.button_selected.label",
            "control.button_selected",
        ));

    let view = UiRuntimeView::from_artifact(&artifact);
    let control = view
        .control(&ControlNodeId::new("control.button_selected"))
        .expect("selected button control should be projected");

    assert!(
        control
            .binding_snapshots
            .iter()
            .any(|row| { row.binding.edge_id.as_str() == "binding.button_selected.label" })
    );
}

#[test]
fn runtime_view_does_not_join_unrelated_bindings() {
    let mut artifact = compiled_selected_button_artifact();
    add_second_control(&mut artifact);
    artifact
        .tables
        .binding_snapshots
        .rows
        .push(control_property_binding_row(
            "binding.button_other.label",
            "host.button_other.label",
            "control.button_other",
        ));

    let view = UiRuntimeView::from_artifact(&artifact);

    assert!(view.passed(), "{:?}", view.diagnostics);

    let selected = view
        .control(&ControlNodeId::new("control.button_selected"))
        .expect("selected button control should be projected");
    let other = view
        .control(&ControlNodeId::new("control.button_other"))
        .expect("second button control should be projected");

    assert_eq!(selected.binding_snapshots.len(), 1);
    assert_eq!(
        selected.binding_snapshots[0].binding.edge_id.as_str(),
        "binding.button_selected.selected"
    );
    assert_eq!(other.binding_snapshots.len(), 1);
    assert_eq!(
        other.binding_snapshots[0].binding.edge_id.as_str(),
        "binding.button_other.label"
    );
}

fn compiled_selected_button_artifact() -> UiRuntimeArtifact {
    let node = load_node("assets/ui_gallery/button/selected.ron");
    let registry = ControlPackageRegistry::new()
        .with_package(runenwerk_control_package())
        .expect("runenwerk controls package should register");

    let formation_report = form_ui_program_report_from_node_with_registry_snapshot(
        "ui_gallery.button.selected",
        "assets.ui_gallery.button.selected",
        &node,
        &registry.snapshot(),
    );

    assert!(
        formation_report.passed(),
        "{:?}",
        formation_report.diagnostics
    );

    let report = UiCompiler.compile_report(&formation_report.program);
    assert!(
        report.passed(),
        "{:?}",
        report.artifact.manifest.diagnostics
    );

    report.artifact
}

fn add_second_control(artifact: &mut UiRuntimeArtifact) {
    let mut control = artifact.tables.controls.rows[0].clone();
    control.node.node_id = ControlNodeId::new("control.button_other");
    control.node.local_state_requirements.clear();
    artifact.tables.controls.rows.push(control);

    let mut property = artifact.tables.properties.rows[0].clone();
    property.snapshot.snapshot_id = ControlPropertySnapshotId::new("properties.button_other");
    property.snapshot.owner_control = ControlNodeId::new("control.button_other");
    artifact.tables.properties.rows.push(property);
}

fn control_property_binding_row(
    edge_id: &str,
    host_endpoint_id: &str,
    control_id: &str,
) -> BindingSnapshotRow {
    BindingSnapshotRow {
        binding: BindingEdge::new(
            BindingEdgeId::new(edge_id),
            BindingEndpoint::HostData {
                endpoint_id: BindingEndpointId::new(host_endpoint_id),
            },
            BindingEndpoint::ControlProperty {
                control_id: ControlNodeId::new(control_id),
                endpoint_id: BindingEndpointId::new(format!("{control_id}.label")),
            },
            UiSchemaRef::new("runenwerk.ui.controls.button.properties", 1),
        ),
        source_map_index: Some(1),
    }
}

fn assert_has_diagnostic(view: &UiRuntimeView, code: &str) {
    assert!(
        view.diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == code),
        "{:?}",
        view.diagnostics
    );
}

fn load_node(relative_repo_path: &str) -> UiNodeDefinition {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|path| path.parent())
        .and_then(|path| path.parent())
        .expect("ui_runtime_view should live under domain/ui/ui_runtime_view")
        .to_path_buf();

    let path = repo_root.join(relative_repo_path);
    let source = fs::read_to_string(&path)
        .unwrap_or_else(|error| panic!("failed to read {:?}: {error}", path));

    ron::from_str(&source).expect("fixture should parse as UiNodeDefinition")
}
