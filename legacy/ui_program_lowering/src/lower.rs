//! File: domain/ui/ui_program_lowering/src/lower.rs
//! Crate: ui_program_lowering
//!
//! Semantic lowering from authored UI nodes into typed UiProgram graph families.

use ui_definition::{
    AuthoredUiNodePath, UiNodeDefinition,
    authored_control_schema::authored_control_properties_to_schema_value,
};
use ui_program::{
    AccessibilityNode, AccessibilityNodeId, AccessibilityRole, BindingEdge, BindingEdgeId,
    BindingEndpoint, BindingEndpointId, ControlGraphNode, ControlKindRef, ControlNodeId,
    ControlPackageRef, ControlPropertySnapshot, ControlPropertySnapshotId, InspectionEntry,
    InspectionEntryId, InteractionHandler, InteractionHandlerId, InteractionTrigger,
    LayoutConstraintId, LayoutGraphNode, RouteId, StateRequirement, StateRequirementId,
    StateRequirementLifecycle, StyleRule, StyleRuleId, StyleSlotId, UiProgram, UiProgramDiagnostic,
    VisualOperator, VisualOperatorId,
};
use ui_schema::UiSchemaValidationDiagnostic;

use crate::catalog::UiProgramFormationControlCatalog;
use crate::source_map::source_map_for_path;

pub(crate) fn lower_control_nodes(
    node: &UiNodeDefinition,
    path: &AuthoredUiNodePath,
    catalog: &UiProgramFormationControlCatalog,
    program: &mut UiProgram,
) {
    if let UiNodeDefinition::Control {
        id,
        kind,
        properties,
        bindings,
        route,
        accessibility,
        children,
        ..
    } = node
    {
        let control_id = ControlNodeId::new(format!("control.{}", id.as_str()));
        let Some(contract) = catalog.control_kind(kind.as_str()) else {
            program.diagnostics.push(
                UiProgramDiagnostic::new(
                    "ui.program.control.unknown_kind",
                    format!(
                        "control {} declares unknown control kind {}",
                        id.as_str(),
                        kind.as_str()
                    ),
                )
                .with_source_map(
                    source_map_for_path(path, format!("program.control.{}", id.as_str())).entry,
                ),
            );

            return;
        };

        let property_value = authored_control_properties_to_schema_value(properties);
        let property_validation = contract.property_schema.validate(&property_value);
        if !property_validation.is_valid() {
            let source_map =
                source_map_for_path(path, format!("program.control.{}", id.as_str())).entry;
            for schema_diagnostic in property_validation.diagnostics {
                program.diagnostics.push(property_validation_diagnostic(
                    id.as_str(),
                    kind.as_str(),
                    &schema_diagnostic,
                    source_map.clone(),
                ));
            }
            return;
        }

        let mut control = ControlGraphNode::new(
            control_id.clone(),
            ControlPackageRef::new(contract.package_id.clone()),
            ControlKindRef::new(kind.as_str()),
        )
        .with_source_map(source_map_for_path(
            path,
            format!("program.control.{}", id.as_str()),
        ));

        let property_snapshot = ControlPropertySnapshot::new(
            ControlPropertySnapshotId::new(format!("properties.{}", id.as_str())),
            control_id.clone(),
            contract.property_schema.schema_ref.clone(),
            property_value,
        )
        .with_source_map(source_map_for_path(
            path,
            format!("program.properties.{}", id.as_str()),
        ));

        let mut accessibility_rows = Vec::new();
        let mut local_diagnostics = Vec::new();

        if let Some(accessibility) = accessibility.as_ref() {
            let accessibility_source_map =
                source_map_for_path(path, format!("program.accessibility.{}", id.as_str()));

            if let Some(role) = accessibility_role_from_authored(accessibility.role.as_str()) {
                let mut accessibility_node = AccessibilityNode::new(
                    AccessibilityNodeId::new(format!("accessibility.{}", id.as_str())),
                    control_id.clone(),
                    role,
                )
                .with_source_map(accessibility_source_map);

                if let Some(label) = accessibility.label.as_ref() {
                    accessibility_node = accessibility_node.with_label(label.clone());
                }

                accessibility_rows.push(accessibility_node);
            } else {
                local_diagnostics.push(
                    UiProgramDiagnostic::new(
                        "ui.program.accessibility.unknown_role",
                        format!(
                            "control {} declares unknown accessibility role {}",
                            id.as_str(),
                            accessibility.role
                        ),
                    )
                    .with_source_map(accessibility_source_map.entry),
                );
            }
        }

        if !local_diagnostics.is_empty() {
            program.diagnostics.extend(local_diagnostics);
            return;
        }

        let inspection_entry = InspectionEntry::new(
            InspectionEntryId::new(format!("inspection.{}", id.as_str())),
            control_id.clone(),
            contract.display_name.clone(),
            contract.property_schema.schema_ref.clone(),
        )
        .with_source_map(source_map_for_path(
            path,
            format!("program.inspection.{}", id.as_str()),
        ));

        if route.is_some() {
            control = control.with_capability(contract.activation_capability.clone());
        }

        let layout_node = LayoutGraphNode::new(
            LayoutConstraintId::new(format!("layout.{}", id.as_str())),
            control_id.clone(),
        )
        .with_layout_kernel(contract.layout_kernel.clone())
        .with_source_map(source_map_for_path(
            path,
            format!("program.layout.{}", id.as_str()),
        ));

        let style_rule = StyleRule::new(
            StyleRuleId::new(format!("style.{}", id.as_str())),
            control_id.clone(),
            StyleSlotId::new(format!("style_slot.{}", id.as_str())),
            contract.property_schema.schema_ref.clone(),
        )
        .with_source_map(source_map_for_path(
            path,
            format!("program.style.{}", id.as_str()),
        ));

        let visual_operator = VisualOperator::new(
            VisualOperatorId::new(format!("visual.{}", id.as_str())),
            control_id.clone(),
            contract.visual_kernel.clone(),
        )
        .with_source_map(source_map_for_path(
            path,
            format!("program.visual.{}", id.as_str()),
        ));

        let mut interaction_rows = Vec::new();
        if let Some(route) = route.as_ref() {
            interaction_rows.push(
                InteractionHandler::new(
                    InteractionHandlerId::new(format!("interaction.{}.activate", id.as_str())),
                    control_id.clone(),
                    InteractionTrigger::Press,
                    RouteId::new(route.as_str()),
                    contract.event_payload_schema.clone(),
                )
                .with_capability(contract.activation_capability.clone())
                .with_source_map(source_map_for_path(
                    path,
                    format!("program.interaction.{}.activate", id.as_str()),
                )),
            );
        }

        let mut state_rows = Vec::new();
        let mut binding_rows = Vec::new();
        for (binding_name, binding_ref) in bindings {
            let state_id =
                StateRequirementId::new(format!("state.{}.{}", id.as_str(), binding_name));
            let state_endpoint_id =
                BindingEndpointId::new(format!("state.{}.{}", id.as_str(), binding_name));
            let binding_id =
                BindingEdgeId::new(format!("binding.{}.{}", id.as_str(), binding_name));

            state_rows.push(
                StateRequirement::new(
                    state_id.clone(),
                    control_id.clone(),
                    StateRequirementLifecycle::HostFed,
                    contract.state_schema.clone(),
                )
                .with_source_map(source_map_for_path(
                    path,
                    format!("program.state.{}.{}", id.as_str(), binding_name),
                )),
            );

            binding_rows.push(
                BindingEdge::new(
                    binding_id,
                    BindingEndpoint::HostData {
                        endpoint_id: BindingEndpointId::new(binding_ref.as_str()),
                    },
                    BindingEndpoint::UiState {
                        requirement_id: state_id,
                        endpoint_id: state_endpoint_id,
                    },
                    contract.state_schema.clone(),
                )
                .with_source_map(source_map_for_path(
                    path,
                    format!("program.binding.{}.{}", id.as_str(), binding_name),
                )),
            );
        }

        program.graphs.control.add_node(control);
        program.graphs.properties.add_snapshot(property_snapshot);
        program.graphs.layout.constraints.push(layout_node);
        program.graphs.style.rules.push(style_rule);
        program.graphs.state.requirements.extend(state_rows);
        program.graphs.interaction.handlers.extend(interaction_rows);
        program.graphs.binding.bindings.extend(binding_rows);
        program.graphs.visual.operators.push(visual_operator);
        program
            .graphs
            .accessibility
            .nodes
            .extend(accessibility_rows);
        program.graphs.inspection.entries.push(inspection_entry);

        for child in children {
            lower_control_nodes(child, &path.child(child.id()), catalog, program);
        }
        return;
    }

    for child in node.children() {
        lower_control_nodes(child, &path.child(child.id()), catalog, program);
    }
}

fn property_validation_diagnostic(
    control_id: &str,
    control_kind: &str,
    schema_diagnostic: &UiSchemaValidationDiagnostic,
    source_map: ui_program::UiProgramSourceMapEntry,
) -> UiProgramDiagnostic {
    let code = match schema_diagnostic.diagnostic_id.as_str() {
        "ui.schema.required_field_missing" => {
            "ui.program.control.properties.required_field_missing"
        }
        "ui.schema.unknown_field" => "ui.program.control.properties.unknown_field",
        "ui.schema.string_value_not_allowed" => {
            "ui.program.control.properties.string_value_not_allowed"
        }
        "ui.schema.field_kind_mismatch" | "ui.schema.root_kind_mismatch" => {
            "ui.program.control.properties.value_kind_mismatch"
        }
        _ => "ui.program.control.properties.validation_failed",
    };
    let field_path = if schema_diagnostic.field_path.is_empty() {
        "<root>".to_owned()
    } else {
        schema_diagnostic.field_path.join(".")
    };

    UiProgramDiagnostic::new(
        code,
        format!(
            "control {control_id} kind {control_kind} properties failed schema {}@{} at {field_path}: {} ({})",
            schema_diagnostic.schema_ref.id.as_str(),
            schema_diagnostic.schema_ref.version.value(),
            schema_diagnostic.message,
            schema_diagnostic.diagnostic_id.as_str()
        ),
    )
    .with_source_map(source_map)
}

fn accessibility_role_from_authored(role: &str) -> Option<AccessibilityRole> {
    match role {
        "button" => Some(AccessibilityRole::Button),
        "label" => Some(AccessibilityRole::Label),
        "text_field" | "textfield" | "text-field" => Some(AccessibilityRole::TextField),
        "color_picker" | "color-picker" => Some(AccessibilityRole::ColorPicker),
        "list" => Some(AccessibilityRole::List),
        "tree" => Some(AccessibilityRole::Tree),
        "table" => Some(AccessibilityRole::Table),
        "prompt" => Some(AccessibilityRole::Prompt),
        _ => None,
    }
}
