use super::*;
use ui_artifacts::UiRuntimeArtifact;
use ui_binding::{BindingAuthorization, HostDataSnapshot};
use ui_program::{
    AccessibilityNode, AccessibilityNodeId, AccessibilityRole, BindingEdge, BindingEdgeId,
    BindingEndpoint, BindingEndpointId, ControlGraphNode, ControlKernelRef, ControlKindRef,
    ControlNodeId, ControlPackageRef, InspectionEntry, InspectionEntryId, RouteCapability,
    StateRequirement, StateRequirementId, StateRequirementLifecycle, UiProgram, UiProgramId,
    UiProgramVersion, VisualOperator, VisualOperatorId,
};
use ui_schema::{UiSchemaRef, UiSchemaValue};
use ui_state::{UiStateBucket, UiStateModel};

#[test]
fn evaluator_contract_projects_typed_artifact_tables_and_updates_boundary_state() {
    let artifact = UiRuntimeArtifact::from_program(&evaluation_program());
    let mut state = UiStateModel::default();
    let output = UiEvaluator.evaluate_with_context(
        &artifact,
        &mut state,
        UiEvaluationContext::default()
            .with_binding_authorization(BindingAuthorization::read_write("editor.score.read"))
            .with_host_data(HostDataSnapshot::new(
                "binding.score.host",
                UiSchemaValue::integer(42),
                11,
            )),
    );

    assert_eq!(output.controls.rows.len(), 1);
    assert_eq!(output.state.rows[0].bucket, UiStateBucket::HostFed);
    assert_eq!(output.state.rows[0].revision, 1);
    assert_eq!(
        state.value("state.score.value"),
        Some(&UiSchemaValue::integer(42))
    );
    assert_eq!(
        output.binding.dirty_report.dirty_bindings[0].as_str(),
        "binding.score.value"
    );
    assert_eq!(
        output.visual.operators[0].operator.operator_id.as_str(),
        "visual.score.label"
    );
    assert_eq!(
        output.visual.text_layout_requests[0].control_id.as_str(),
        "control.score.label"
    );
    assert_eq!(
        output.accessibility.rows[0].node.node_id.as_str(),
        "accessibility.score.label"
    );
    assert_eq!(
        output.inspection.rows[0].entry.entry_id.as_str(),
        "inspection.score.value"
    );
    assert!(output.diagnostics.is_empty());
}

fn evaluation_program() -> UiProgram {
    let control_id = ControlNodeId::new("control.score.label");
    let state_id = StateRequirementId::new("state.score.value");
    let mut program = UiProgram::new(UiProgramId::new("program.score"), UiProgramVersion::new(1));
    program.graphs.control.add_node(ControlGraphNode::new(
        control_id.clone(),
        ControlPackageRef::new("editor.score"),
        ControlKindRef::new("editor.score.label"),
    ));
    program
        .graphs
        .state
        .requirements
        .push(StateRequirement::new(
            state_id.clone(),
            control_id.clone(),
            StateRequirementLifecycle::HostFed,
            UiSchemaRef::new("ui.score.value", 1),
        ));
    program.graphs.binding.bindings.push(
        BindingEdge::new(
            BindingEdgeId::new("binding.score.value"),
            BindingEndpoint::HostData {
                endpoint_id: BindingEndpointId::new("binding.score.host"),
            },
            BindingEndpoint::UiState {
                requirement_id: state_id.clone(),
                endpoint_id: BindingEndpointId::new("binding.score.state"),
            },
            UiSchemaRef::new("ui.score.value", 1),
        )
        .with_capability(RouteCapability::new("editor.score.read")),
    );
    program.graphs.visual.operators.push(VisualOperator::new(
        VisualOperatorId::new("visual.score.label"),
        control_id.clone(),
        ControlKernelRef::new("editor.score.label.visual"),
    ));
    program
        .graphs
        .accessibility
        .nodes
        .push(AccessibilityNode::new(
            AccessibilityNodeId::new("accessibility.score.label"),
            control_id.clone(),
            AccessibilityRole::Label,
        ));
    program.graphs.inspection.entries.push(InspectionEntry::new(
        InspectionEntryId::new("inspection.score.value"),
        control_id,
        "Score",
        UiSchemaRef::new("ui.score.value", 1),
    ));
    program
}
