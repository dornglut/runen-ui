use super::*;
use std::{fs, path::PathBuf};
use ui_controls::{ControlPackageRegistry, runenwerk_control_package};
use ui_definition::UiNodeDefinition;
use ui_program::UiSchemaRef;
use ui_program::{
    AccessibilityNode, AccessibilityNodeId, AccessibilityRole, BindingEdge, BindingEdgeId,
    BindingEndpoint, BindingEndpointId, ControlGraphNode, ControlKernelRef, ControlKindRef,
    ControlNodeId, ControlPackageRef, ControlPropertySnapshot, ControlPropertySnapshotId,
    InspectionEntry, InspectionEntryId, InteractionHandler, InteractionHandlerId,
    InteractionTrigger, LayoutConstraintId, LayoutGraphNode, RouteCapability, RouteId,
    StateRequirement, StateRequirementId, StateRequirementLifecycle, StyleRule, StyleRuleId,
    StyleSlotId, UiProgram, UiProgramId, UiProgramSourceId, UiProgramSourceMapAttachment,
    UiProgramSourceMapEntry, UiProgramSourceSpan, UiProgramTargetId, UiProgramVersion,
    UiSchemaValue, VisualOperator, VisualOperatorId,
};
use ui_program_lowering::form_ui_program_report_from_node_with_registry_snapshot;

#[test]
fn compiler_contract_resolves_packages_capabilities_cache_keys_and_source_maps() {
    let program = compiler_program(RouteCapability::new("editor.inspector.read"));
    let report = UiCompiler.compile_report(&program);

    assert!(report.passed());
    assert_eq!(report.package_resolution.packages.len(), 1);
    assert_eq!(
        report.package_resolution.packages[0].kernel_ids,
        [
            "runenwerk.ui.controls.label.layout",
            "runenwerk.ui.controls.label.visual"
        ]
    );
    assert!(
        report
            .capability_checks
            .iter()
            .any(|check| check.status == CapabilityCheckStatus::SatisfiedByControl)
    );
    assert!(
        report
            .cache_key()
            .as_str()
            .starts_with("ui-program:editor.inspector:2:")
    );
    assert!(
        report
            .compiled_source_map()
            .entries
            .iter()
            .any(|entry| entry.target_id == "program.control.title")
    );
    assert_eq!(report.artifact.manifest.diagnostics, []);
    assert_eq!(report.artifact.tables.controls.rows.len(), 1);
    assert_eq!(report.artifact.tables.properties.rows.len(), 1);
    assert_eq!(report.artifact.tables.binding_snapshots.rows.len(), 1);
}

#[test]
fn compiler_contract_reports_missing_capability_declarations() {
    let program = compiler_program(RouteCapability::new("editor.inspector.write"));
    let report = UiCompiler.compile_report(&program);

    assert!(!report.passed());
    assert!(
        report
            .artifact
            .manifest
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code
                == "ui.compiler.capability.missing_control_declaration")
    );
}

#[test]
fn compiler_reports_missing_property_snapshot_for_control() {
    let mut program = compiler_program(RouteCapability::new("editor.inspector.read"));
    program.graphs.properties.rows.clear();

    let report = UiCompiler.compile_report(&program);

    assert!(!report.passed());
    assert!(!report.graph_integrity.passed());
    assert!(
        report
            .artifact
            .manifest
            .diagnostics
            .iter()
            .any(|diagnostic| {
                diagnostic.code == "ui.compiler.graph.missing_property_snapshot"
                    && diagnostic.message.contains("control.title")
            })
    );
}

#[test]
fn compiler_reports_duplicate_property_snapshots_for_control() {
    let mut program = compiler_program(RouteCapability::new("editor.inspector.read"));
    let mut duplicate = program.graphs.properties.rows[0].clone();
    duplicate.snapshot_id = ControlPropertySnapshotId::new("properties.title.duplicate");
    program.graphs.properties.rows.push(duplicate);

    let report = UiCompiler.compile_report(&program);

    assert!(!report.passed());
    assert!(!report.graph_integrity.passed());
    assert!(
        report
            .artifact
            .manifest
            .diagnostics
            .iter()
            .any(|diagnostic| {
                diagnostic.code == "ui.compiler.graph.duplicate_property_snapshots"
                    && diagnostic.message.contains("control.title")
                    && diagnostic.message.contains("properties.title")
                    && diagnostic.message.contains("properties.title.duplicate")
            })
    );
}

#[test]
fn compiler_reports_property_snapshot_with_missing_owner_control() {
    let mut program = compiler_program(RouteCapability::new("editor.inspector.read"));
    program.graphs.properties.rows[0].owner_control = ControlNodeId::new("control.missing");

    let report = UiCompiler.compile_report(&program);

    assert!(!report.passed());
    assert!(!report.graph_integrity.passed());
    assert!(
        report
            .artifact
            .manifest
            .diagnostics
            .iter()
            .any(|diagnostic| {
                diagnostic.code == "ui.compiler.graph.missing_property_snapshot"
                    && diagnostic.message.contains("control.title")
            })
    );
    assert!(
        report
            .artifact
            .manifest
            .diagnostics
            .iter()
            .any(|diagnostic| {
                diagnostic.code == "ui.compiler.graph.property_snapshot_missing_owner_control"
                    && diagnostic.message.contains("properties.title")
                    && diagnostic.message.contains("control.missing")
            })
    );
}

#[test]
fn compiler_reports_property_snapshot_schema_mismatch() {
    let mut program = compiler_program(RouteCapability::new("editor.inspector.read"));
    program.graphs.properties.rows[0].schema = UiSchemaRef::new("ui.label.wrong_properties", 1);

    let report = UiCompiler.compile_report(&program);

    assert!(!report.passed());
    assert!(!report.graph_integrity.passed());
    assert!(
        report
            .artifact
            .manifest
            .diagnostics
            .iter()
            .any(|diagnostic| {
                diagnostic.code == "ui.compiler.graph.property_snapshot_schema_mismatch"
                    && diagnostic.message.contains("properties.title")
                    && diagnostic.message.contains("control.title")
                    && diagnostic.message.contains("ui.label.wrong_properties@1")
                    && diagnostic.message.contains("ui.label.properties@1")
            })
    );
}

fn compiler_program(handler_capability: RouteCapability) -> UiProgram {
    let mut program = UiProgram::new(
        UiProgramId::new("editor.inspector"),
        UiProgramVersion::new(2),
    )
    .with_source_map_entry(UiProgramSourceMapEntry::new(
        UiProgramSourceId::new("definition.inspector"),
        UiProgramTargetId::new("program.inspector"),
    ));
    let source_map = || {
        UiProgramSourceMapAttachment::new(UiProgramSourceMapEntry::new(
            UiProgramSourceId::new("definition.title"),
            UiProgramTargetId::new("program.control.title"),
        ))
        .with_source_span(UiProgramSourceSpan::new(1, 12))
    };
    program.graphs.control.add_node(
        ControlGraphNode::new(
            ControlNodeId::new("control.title"),
            ControlPackageRef::new("runenwerk.ui.controls"),
            ControlKindRef::new("runenwerk.ui.controls.label"),
        )
        .with_capability(RouteCapability::new("editor.inspector.read"))
        .with_source_map(source_map()),
    );
    program.graphs.properties.add_snapshot(
        ControlPropertySnapshot::new(
            ControlPropertySnapshotId::new("properties.title"),
            ControlNodeId::new("control.title"),
            UiSchemaRef::new("ui.label.properties", 1),
            UiSchemaValue::object([("label", UiSchemaValue::string("Title"))]),
        )
        .with_source_map(UiProgramSourceMapAttachment::new(
            UiProgramSourceMapEntry::new(
                UiProgramSourceId::new("definition.title"),
                UiProgramTargetId::new("program.properties.title"),
            ),
        )),
    );
    program.graphs.layout.constraints.push(
        LayoutGraphNode::new(
            LayoutConstraintId::new("layout.title"),
            ControlNodeId::new("control.title"),
        )
        .with_layout_kernel(ControlKernelRef::new("runenwerk.ui.controls.label.layout"))
        .with_source_map(source_map()),
    );
    program.graphs.style.rules.push(
        StyleRule::new(
            StyleRuleId::new("style.title"),
            ControlNodeId::new("control.title"),
            StyleSlotId::new("style_slot.title"),
            UiSchemaRef::new("ui.label.properties", 1),
        )
        .with_source_map(source_map()),
    );
    program.graphs.state.requirements.push(
        StateRequirement::new(
            StateRequirementId::new("state.title"),
            ControlNodeId::new("control.title"),
            StateRequirementLifecycle::HostFed,
            UiSchemaRef::new("ui.label.state", 1),
        )
        .with_source_map(source_map()),
    );
    program.graphs.interaction.handlers.push(
        InteractionHandler::new(
            InteractionHandlerId::new("interaction.title.preview"),
            ControlNodeId::new("control.title"),
            InteractionTrigger::ValuePreview,
            RouteId::new("editor.inspector.preview"),
            UiSchemaRef::new("ui.label.properties", 1),
        )
        .with_capability(handler_capability)
        .with_source_map(source_map()),
    );
    program.graphs.binding.bindings.push(
        BindingEdge::new(
            BindingEdgeId::new("binding.title"),
            BindingEndpoint::HostData {
                endpoint_id: BindingEndpointId::new("host.selection.title"),
            },
            BindingEndpoint::ControlProperty {
                control_id: ControlNodeId::new("control.title"),
                endpoint_id: BindingEndpointId::new("control.title.text"),
            },
            UiSchemaRef::new("ui.label.properties", 1),
        )
        .with_capability(RouteCapability::new("editor.inspector.read")),
    );
    program.graphs.visual.operators.push(
        VisualOperator::new(
            VisualOperatorId::new("visual.title"),
            ControlNodeId::new("control.title"),
            ControlKernelRef::new("runenwerk.ui.controls.label.visual"),
        )
        .with_source_map(source_map()),
    );
    program
        .graphs
        .accessibility
        .nodes
        .push(AccessibilityNode::new(
            AccessibilityNodeId::new("accessibility.title"),
            ControlNodeId::new("control.title"),
            AccessibilityRole::Label,
        ));
    program.graphs.inspection.entries.push(InspectionEntry::new(
        InspectionEntryId::new("inspection.title"),
        ControlNodeId::new("control.title"),
        "Title",
        UiSchemaRef::new("ui.label.properties", 1),
    ));
    program
}

// File: domain/ui/ui_compiler/src/tests.rs
// Test: compiler_lowers_authored_button_program_to_runtime_artifact

#[test]
fn compiler_lowers_authored_button_program_to_runtime_artifact() {
    let node = load_authored_node("assets/ui_gallery/button/selected.ron");

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
    assert!(formation_report.diagnostics.is_empty());

    let report = UiCompiler.compile_report(&formation_report.program);

    assert!(
        report.passed(),
        "{:?}",
        report.artifact.manifest.diagnostics
    );
    assert_eq!(report.artifact.manifest.diagnostics, []);

    assert_eq!(report.artifact.tables.controls.rows.len(), 1);
    assert_eq!(report.artifact.tables.properties.rows.len(), 1);
    assert_eq!(report.artifact.tables.layout.rows.len(), 1);
    assert_eq!(report.artifact.tables.style.rows.len(), 1);
    assert_eq!(report.artifact.tables.state.rows.len(), 1);
    assert_eq!(report.artifact.tables.interaction.rows.len(), 1);
    assert_eq!(report.artifact.tables.binding_snapshots.rows.len(), 1);
    assert_eq!(report.artifact.tables.collection_diffs.rows.len(), 1);
    assert_eq!(report.artifact.tables.visual.rows.len(), 1);
    assert_eq!(report.artifact.tables.accessibility.rows.len(), 1);
    assert_eq!(report.artifact.tables.inspection.rows.len(), 1);

    assert!(
        report
            .artifact
            .manifest
            .package_ids
            .iter()
            .any(|package_id| package_id == "runenwerk.ui.controls")
    );

    let property_row = &report.artifact.tables.properties.rows[0];
    assert_eq!(
        property_row.snapshot.owner_control.as_str(),
        "control.button_selected"
    );
    assert_eq!(
        property_row.snapshot.schema.id.as_str(),
        "runenwerk.ui.controls.button.properties"
    );
    assert_eq!(
        property_row
            .snapshot
            .value
            .get("label")
            .and_then(UiSchemaValue::as_str),
        Some("Selected")
    );
    assert_eq!(
        property_row
            .snapshot
            .value
            .get("variant")
            .and_then(UiSchemaValue::as_str),
        Some("secondary")
    );
    assert!(property_row.source_map_index.is_some());

    assert!(
        report
            .artifact
            .manifest
            .control_kind_ids
            .iter()
            .any(|kind_id| kind_id == "runenwerk.ui.controls.button")
    );

    assert!(
        report
            .artifact
            .manifest
            .schema_ids
            .iter()
            .any(
                |schema| schema.schema_id == "runenwerk.ui.controls.button.properties"
                    && schema.schema_version == 1
            )
    );

    assert!(
        report
            .artifact
            .manifest
            .schema_ids
            .iter()
            .any(
                |schema| schema.schema_id == "runenwerk.ui.controls.button.state"
                    && schema.schema_version == 1
            )
    );

    assert!(
        report
            .artifact
            .manifest
            .schema_ids
            .iter()
            .any(
                |schema| schema.schema_id == "runenwerk.ui.controls.button.event"
                    && schema.schema_version == 1
            )
    );

    assert!(report.artifact.manifest.route_ids.iter().any(|route| {
        route.route_id == "ui_gallery.button.selected.activate"
            && route.payload_schema.schema_id == "runenwerk.ui.controls.button.event"
            && route.payload_schema.schema_version == 1
    }));

    assert!(
        report
            .artifact
            .manifest
            .kernel_ids
            .iter()
            .any(|kernel_id| kernel_id == "runenwerk.ui.controls.button.layout")
    );

    assert!(
        report
            .artifact
            .manifest
            .kernel_ids
            .iter()
            .any(|kernel_id| kernel_id == "runenwerk.ui.controls.button.visual")
    );

    assert!(
        report
            .artifact
            .manifest
            .capability_ids
            .iter()
            .any(|capability_id| capability_id == "runenwerk.ui.controls.activate")
    );
    assert!(
        report
            .artifact
            .manifest
            .cache_key
            .as_str()
            .starts_with("ui-program:ui_gallery.button.selected:1:")
    );
}

fn load_authored_node(path: &str) -> UiNodeDefinition {
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|path| path.parent())
        .and_then(|path| path.parent())
        .expect("ui_compiler should live under domain/ui/ui_compiler")
        .to_path_buf();

    let fixture_path = repo_root.join(path);
    let source = fs::read_to_string(&fixture_path).unwrap_or_else(|error| {
        panic!(
            "fixture {} should be readable: {error}",
            fixture_path.display()
        )
    });

    ron::from_str(&source).expect("fixture should parse as UiNodeDefinition")
}
