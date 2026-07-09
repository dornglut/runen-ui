use super::*;
use ui_program::UiSchemaRef;
use ui_program::{
    AccessibilityNode, AccessibilityNodeId, AccessibilityRole, BindingEdge, BindingEdgeId,
    BindingEndpoint, BindingEndpointId, ControlGraphNode, ControlKernelRef, ControlKindRef,
    ControlNodeId, ControlPackageRef, ControlPropertySnapshot, ControlPropertySnapshotId,
    InspectionEntry, InspectionEntryId, InteractionHandler, InteractionHandlerId,
    InteractionTrigger, LayoutConstraintId, LayoutGraphNode, RouteCapability, RouteId,
    StateRequirement, StateRequirementId, StateRequirementLifecycle, StyleRule, StyleRuleId,
    StyleSlotId, UiProgram, UiProgramDiagnostic, UiProgramId, UiProgramSourceId,
    UiProgramSourceMapAttachment, UiProgramSourceMapEntry, UiProgramTargetId, UiProgramVersion,
    VisualOperator, VisualOperatorId,
};
use ui_schema::UiSchemaValue;

#[test]
fn artifact_contract_splits_manifest_from_typed_runtime_tables() {
    let artifact = UiRuntimeArtifact::from_program(&artifact_program());

    assert_eq!(artifact.manifest.program_id, "program.artifact");
    assert_eq!(artifact.manifest.package_ids, ["runenwerk.ui.controls"]);
    assert_eq!(
        artifact.manifest.control_kind_ids,
        ["runenwerk.ui.controls.label"]
    );
    assert_eq!(artifact.tables.controls.rows.len(), 1);
    assert_eq!(artifact.tables.properties.rows.len(), 1);
    assert_eq!(artifact.tables.layout.rows.len(), 1);
    assert_eq!(artifact.tables.style.rows.len(), 1);
    assert_eq!(artifact.tables.state.rows.len(), 1);
    assert_eq!(artifact.tables.interaction.rows.len(), 1);
    assert_eq!(artifact.tables.binding_snapshots.rows.len(), 1);
    assert_eq!(artifact.tables.collection_diffs.rows.len(), 1);
    assert_eq!(artifact.tables.visual.rows.len(), 1);
    assert_eq!(artifact.tables.text_layout_requests.rows.len(), 1);
    assert_eq!(artifact.tables.accessibility.rows.len(), 1);
    assert_eq!(artifact.tables.inspection.rows.len(), 1);
    assert!(
        artifact
            .manifest
            .cache_key
            .as_str()
            .starts_with("ui-program:program.artifact:1:")
    );
}

#[test]
fn artifact_contract_preserves_source_maps_routes_and_collection_diffs() {
    let artifact = UiRuntimeArtifact::from_program(&artifact_program());

    assert_eq!(
        artifact.manifest.route_ids[0].route_id,
        "editor.title.activate"
    );
    assert!(artifact.manifest.schema_ids.contains(&RuntimeSchemaRef {
        schema_id: "ui.label.properties".to_owned(),
        schema_version: 1,
    }));
    assert!(
        artifact
            .manifest
            .source_map
            .entries
            .iter()
            .any(|entry| entry.table == RuntimeTableKind::Control
                && entry.source_id == "definition.title")
    );
    assert_eq!(
        artifact.tables.collection_diffs.rows[0].strategy,
        CollectionDiffStrategy::ReplaceValue
    );
    assert!(matches!(
        artifact.tables.collection_diffs.rows[0].source,
        RuntimeBindingEndpoint::HostData { .. }
    ));
}

#[test]
fn artifact_projects_property_snapshots() {
    let artifact = UiRuntimeArtifact::from_program(&artifact_program());

    assert_eq!(artifact.tables.properties.rows.len(), 1);

    let row = &artifact.tables.properties.rows[0];
    assert_eq!(row.snapshot.snapshot_id.as_str(), "properties.title");
    assert_eq!(row.snapshot.owner_control.as_str(), "control.title");
    assert_eq!(row.snapshot.schema.id.as_str(), "ui.label.properties");
    assert_eq!(
        row.snapshot
            .value
            .get("label")
            .and_then(UiSchemaValue::as_str),
        Some("Title")
    );
    assert!(row.source_map_index.is_some());
}

#[test]
fn artifact_manifest_includes_property_schema() {
    let artifact = UiRuntimeArtifact::from_program(&artifact_program());

    assert!(artifact.manifest.schema_ids.contains(&RuntimeSchemaRef {
        schema_id: "ui.label.properties".to_owned(),
        schema_version: 1,
    }));
}

#[test]
fn artifact_cache_key_changes_when_property_value_changes() {
    let mut first = artifact_program();
    let mut second = artifact_program();

    first.graphs.properties.rows[0].value =
        UiSchemaValue::object([("label", UiSchemaValue::string("Title"))]);
    second.graphs.properties.rows[0].value =
        UiSchemaValue::object([("label", UiSchemaValue::string("Renamed"))]);

    let first_artifact = UiRuntimeArtifact::from_program(&first);
    let second_artifact = UiRuntimeArtifact::from_program(&second);

    assert_ne!(
        first_artifact.manifest.cache_key,
        second_artifact.manifest.cache_key
    );
}

#[test]
fn source_map_includes_property_table_entry() {
    let artifact = UiRuntimeArtifact::from_program(&artifact_program());

    assert!(artifact.manifest.source_map.entries.iter().any(|entry| {
        entry.table == RuntimeTableKind::ControlProperties
            && entry.source_id == "definition.title"
            && entry.target_id == "program.properties.title"
    }));
}

fn artifact_program() -> UiProgram {
    let control_id = ControlNodeId::new("control.title");
    let state_id = StateRequirementId::new("state.title.text");
    let property_schema = UiSchemaRef::new("ui.label.properties", 1);
    let state_schema = UiSchemaRef::new("ui.label.state", 1);
    let event_schema = UiSchemaRef::new("ui.label.event", 1);
    let source_map = source_map();

    let mut program = UiProgram::new(
        UiProgramId::new("program.artifact"),
        UiProgramVersion::new(1),
    )
    .with_source_map_entry(source_map.entry.clone())
    .with_diagnostic(
        UiProgramDiagnostic::new("ui.program.ready", "ready")
            .with_source_map(source_map.entry.clone()),
    );
    program.graphs.control.add_node(
        ControlGraphNode::new(
            control_id.clone(),
            ControlPackageRef::new("runenwerk.ui.controls"),
            ControlKindRef::new("runenwerk.ui.controls.label"),
        )
        .with_capability(RouteCapability::new("editor.title.write"))
        .with_source_map(source_map.clone()),
    );
    program.graphs.properties.add_snapshot(
        ControlPropertySnapshot::new(
            ControlPropertySnapshotId::new("properties.title"),
            control_id.clone(),
            property_schema.clone(),
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
        LayoutGraphNode::new(LayoutConstraintId::new("layout.title"), control_id.clone())
            .with_layout_kernel(ControlKernelRef::new("runenwerk.ui.controls.label.layout"))
            .with_source_map(source_map.clone()),
    );
    program.graphs.style.rules.push(
        StyleRule::new(
            StyleRuleId::new("style.title"),
            control_id.clone(),
            StyleSlotId::new("style_slot.title"),
            property_schema.clone(),
        )
        .with_source_map(source_map.clone()),
    );
    program.graphs.state.requirements.push(
        StateRequirement::new(
            state_id.clone(),
            control_id.clone(),
            StateRequirementLifecycle::Committed,
            state_schema,
        )
        .with_source_map(source_map.clone()),
    );
    program.graphs.interaction.handlers.push(
        InteractionHandler::new(
            InteractionHandlerId::new("interaction.title.activate"),
            control_id.clone(),
            InteractionTrigger::Press,
            RouteId::new("editor.title.activate"),
            event_schema,
        )
        .with_capability(RouteCapability::new("editor.title.write"))
        .with_source_map(source_map.clone()),
    );
    program.graphs.binding.bindings.push(
        BindingEdge::new(
            BindingEdgeId::new("binding.title"),
            BindingEndpoint::HostData {
                endpoint_id: BindingEndpointId::new("host.title"),
            },
            BindingEndpoint::UiState {
                requirement_id: state_id,
                endpoint_id: BindingEndpointId::new("state.title.text"),
            },
            property_schema.clone(),
        )
        .with_source_map(source_map.clone()),
    );
    program.graphs.visual.operators.push(
        VisualOperator::new(
            VisualOperatorId::new("visual.title"),
            control_id.clone(),
            ControlKernelRef::new("runenwerk.ui.controls.label.visual"),
        )
        .with_source_map(source_map.clone()),
    );
    program.graphs.accessibility.nodes.push(
        AccessibilityNode::new(
            AccessibilityNodeId::new("accessibility.title"),
            control_id.clone(),
            AccessibilityRole::Label,
        )
        .with_label_source(BindingEndpointId::new("state.title.text"))
        .with_source_map(source_map.clone()),
    );
    program.graphs.inspection.entries.push(
        InspectionEntry::new(
            InspectionEntryId::new("inspection.title"),
            control_id,
            "Title",
            property_schema,
        )
        .with_binding(BindingEndpointId::new("state.title.text"))
        .with_source_map(source_map),
    );
    program
}

fn source_map() -> UiProgramSourceMapAttachment {
    UiProgramSourceMapAttachment::new(UiProgramSourceMapEntry::new(
        UiProgramSourceId::new("definition.title"),
        UiProgramTargetId::new("program.title"),
    ))
}
