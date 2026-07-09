//! File: domain/ui/ui_program/src/lib.rs
//! Crate: ui_program

pub mod commands;
pub mod diagnostics;
pub mod events;
pub mod graphs;
pub mod ids;
pub mod program;
pub mod source_map;
pub mod version;

pub use commands::*;
pub use diagnostics::*;
pub use events::*;
pub use graphs::*;
pub use ids::*;
pub use program::*;
pub use source_map::*;
pub use ui_schema::{UiSchemaRef, UiSchemaValue};
pub use version::*;

#[cfg(test)]
mod tests {
    use super::*;
    use ui_schema::{UiSchema, UiSchemaRef, UiSchemaShape, UiSchemaValue};

    #[test]
    fn route_contract_uses_stable_ids_and_schema_payloads() {
        let route = RouteId::new("editor.color.apply");
        let payload_schema = UiSchema::object("ui.color.rgba", 1)
            .with_required_field("r", UiSchemaShape::Number)
            .with_required_field("g", UiSchemaShape::Number)
            .with_required_field("b", UiSchemaShape::Number)
            .with_required_field("a", UiSchemaShape::Number);
        let source_map = UiProgramSourceMapEntry::new(
            UiProgramSourceId::new("definition.button.apply"),
            UiProgramTargetId::new("program.event.apply"),
        );
        let packet = UiEventPacket::new(
            route.clone(),
            RouteSchemaVersion::new(1),
            UiSchemaRef::new("ui.color.rgba", 1),
            UiSchemaValue::object([
                ("r", UiSchemaValue::number(0.1)),
                ("g", UiSchemaValue::number(0.2)),
                ("b", UiSchemaValue::number(0.3)),
                ("a", UiSchemaValue::number(1.0)),
            ]),
        )
        .with_capability(RouteCapability::new("editor.color.write"))
        .with_source_control(UiEventSourceControlId::new("control.apply_button"))
        .with_phase(UiEventPhase::Commit)
        .with_source_map_entry(source_map.clone())
        .with_payload_validation(&payload_schema);

        assert_eq!(packet.route, route);
        assert_eq!(packet.schema_version.value(), 1);
        assert_eq!(packet.phase, UiEventPhase::Commit);
        assert_eq!(
            packet
                .source_control
                .as_ref()
                .map(UiEventSourceControlId::as_str),
            Some("control.apply_button")
        );
        assert_eq!(packet.capabilities[0].as_str(), "editor.color.write");
        assert!(packet.requires_capability(&RouteCapability::new("editor.color.write")));
        assert_eq!(
            packet.payload_schema(),
            &UiSchemaRef::new("ui.color.rgba", 1)
        );
        assert!(packet.payload.diagnostics.is_empty());
        assert_eq!(packet.source_map, [source_map]);
        assert_eq!(
            packet.payload.value.get("a"),
            Some(&UiSchemaValue::number(1.0))
        );
    }

    #[test]
    fn route_contract_reports_invalid_payload_schema() {
        let payload_schema =
            UiSchema::object("ui.color.rgba", 1).with_required_field("r", UiSchemaShape::Number);
        let packet = UiEventPacket::new(
            RouteId::new("editor.color.apply"),
            RouteSchemaVersion::new(1),
            UiSchemaRef::new("ui.color.rgba", 1),
            UiSchemaValue::object([("r", UiSchemaValue::string("not-a-number"))]),
        )
        .with_payload_validation(&payload_schema)
        .with_diagnostic(UiProgramDiagnostic::new(
            "ui.event.payload.invalid",
            "payload failed schema validation",
        ));

        assert_eq!(
            packet.payload.diagnostics[0].diagnostic_id.as_str(),
            "ui.schema.field_kind_mismatch"
        );
        assert_eq!(packet.diagnostics[0].code, "ui.event.payload.invalid");
    }

    #[test]
    fn architecture_contract_exposes_named_graph_families() {
        let mut program = UiProgram::new(
            UiProgramId::new("editor.inspector"),
            UiProgramVersion::new(1),
        )
        .with_source(UiProgramSource::authored(
            UiProgramSourceId::new("definition.inspector"),
            "inspector definition",
        ))
        .with_source_map_entry(UiProgramSourceMapEntry::new(
            UiProgramSourceId::new("definition.inspector"),
            UiProgramTargetId::new("program.inspector"),
        ))
        .with_diagnostic(
            UiProgramDiagnostic::new("ui.program.ready", "program contract assembled")
                .with_severity(UiProgramDiagnosticSeverity::Info),
        );

        assert_eq!(program.id.as_str(), "editor.inspector");
        assert_eq!(
            program.sources[0].source_id.as_str(),
            "definition.inspector"
        );
        assert_eq!(program.source_map[0].target_id, "program.inspector");
        assert_eq!(
            program.diagnostics[0].severity,
            UiProgramDiagnosticSeverity::Info
        );

        let control_id = ControlNodeId::new("control.apply_button");
        let requirement_id = StateRequirementId::new("state.apply_button.preview");
        let property_schema = UiSchemaRef::new("ui.button.properties", 1);
        let state_schema = UiSchemaRef::new("ui.button.state", 1);
        let event_schema = UiSchemaRef::new("ui.button.event.activate", 1);
        let source_map = UiProgramSourceMapAttachment::new(UiProgramSourceMapEntry::new(
            UiProgramSourceId::new("definition.apply_button"),
            UiProgramTargetId::new("program.control.apply_button"),
        ));

        program.graphs.control.add_node(
            ControlGraphNode::new(
                control_id.clone(),
                ControlPackageRef::new("runenwerk.ui.controls"),
                ControlKindRef::new("runenwerk.ui.controls.button"),
            )
            .with_state_requirement(requirement_id.clone())
            .with_capability(RouteCapability::new("editor.inspector.write"))
            .with_source_map(source_map.clone()),
        );
        program.graphs.properties.add_snapshot(
            ControlPropertySnapshot::new(
                ControlPropertySnapshotId::new("properties.apply_button"),
                control_id.clone(),
                property_schema.clone(),
                UiSchemaValue::object([("label", UiSchemaValue::string("Apply"))]),
            )
            .with_source_map(source_map.clone()),
        );
        program.graphs.layout.constraints.push(
            LayoutGraphNode::new(
                LayoutConstraintId::new("layout.apply_button.fill_width"),
                control_id.clone(),
            )
            .with_layout_kernel(ControlKernelRef::new("runenwerk.ui.controls.button.layout"))
            .with_source_map(source_map.clone()),
        );
        program.graphs.state.requirements.push(
            StateRequirement::new(
                requirement_id.clone(),
                control_id.clone(),
                StateRequirementLifecycle::Preview,
                state_schema.clone(),
            )
            .with_persistence(StatePersistence::Retained)
            .with_source_map(source_map.clone()),
        );
        program.graphs.style.rules.push(StyleRule::new(
            StyleRuleId::new("style.apply_button.primary"),
            control_id.clone(),
            StyleSlotId::new("style_slot.button.fill"),
            property_schema.clone(),
        ));
        program
            .graphs
            .interaction
            .handlers
            .push(InteractionHandler::new(
                InteractionHandlerId::new("interaction.apply_button.press"),
                control_id.clone(),
                InteractionTrigger::Press,
                RouteId::new("editor.inspector.apply"),
                event_schema.clone(),
            ));
        program.graphs.binding.bindings.push(BindingEdge::new(
            BindingEdgeId::new("binding.apply_button.label"),
            BindingEndpoint::HostData {
                endpoint_id: BindingEndpointId::new("host.inspector.selection_name"),
            },
            BindingEndpoint::ControlProperty {
                control_id: control_id.clone(),
                endpoint_id: BindingEndpointId::new("control.apply_button.label"),
            },
            property_schema.clone(),
        ));
        program.graphs.visual.operators.push(VisualOperator::new(
            VisualOperatorId::new("visual.apply_button.button"),
            control_id.clone(),
            ControlKernelRef::new("runenwerk.ui.controls.button.visual"),
        ));
        program
            .graphs
            .accessibility
            .nodes
            .push(AccessibilityNode::new(
                AccessibilityNodeId::new("accessibility.apply_button"),
                control_id.clone(),
                AccessibilityRole::Button,
            ));
        program.graphs.inspection.entries.push(
            InspectionEntry::new(
                InspectionEntryId::new("inspection.apply_button.label"),
                control_id.clone(),
                "Apply button label",
                property_schema,
            )
            .with_binding(BindingEndpointId::new("control.apply_button.label")),
        );

        assert_eq!(
            program
                .graphs
                .control
                .node(&control_id)
                .unwrap()
                .control_kind
                .as_str(),
            "runenwerk.ui.controls.button"
        );
        assert_eq!(
            program
                .graphs
                .properties
                .snapshot_for_control(&control_id)
                .and_then(|snapshot| snapshot.get("label"))
                .and_then(UiSchemaValue::as_str),
            Some("Apply")
        );
        assert_eq!(
            program.graphs.state.requirements[0].lifecycle,
            StateRequirementLifecycle::Preview
        );
        assert_eq!(
            program.graphs.interaction.handlers[0].route.as_str(),
            "editor.inspector.apply"
        );
        assert_eq!(
            program.graphs.visual.operators[0].visual_kernel.as_str(),
            "runenwerk.ui.controls.button.visual"
        );
        assert_eq!(
            program.graphs.accessibility.nodes[0].role,
            AccessibilityRole::Button
        );
        assert_eq!(
            program.graphs.inspection.entries[0].display_name,
            "Apply button label"
        );
    }

    #[test]
    fn default_graphs_include_empty_property_graph() {
        let graphs = UiProgramGraphs::default();

        assert!(graphs.properties.rows.is_empty());
    }

    #[test]
    fn control_property_snapshot_preserves_owner_schema_value_and_source_map() {
        let control_id = ControlNodeId::new("control.apply_button");
        let schema = UiSchemaRef::new("ui.button.properties", 1);
        let source_map = UiProgramSourceMapAttachment::new(UiProgramSourceMapEntry::new(
            UiProgramSourceId::new("definition.apply_button"),
            UiProgramTargetId::new("program.properties.apply_button"),
        ));

        let snapshot = ControlPropertySnapshot::new(
            ControlPropertySnapshotId::new("properties.apply_button"),
            control_id.clone(),
            schema.clone(),
            UiSchemaValue::object([("label", UiSchemaValue::string("Apply"))]),
        )
        .with_source_map(source_map.clone());

        assert_eq!(snapshot.owner_control, control_id);
        assert_eq!(snapshot.schema, schema);
        assert_eq!(
            snapshot.get("label").and_then(UiSchemaValue::as_str),
            Some("Apply")
        );
        assert_eq!(snapshot.source_map, Some(source_map));
    }
}
