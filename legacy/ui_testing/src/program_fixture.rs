//! Concrete UiProgram fixture builders used by headless tests.

use ui_program::{
    AccessibilityNode, AccessibilityNodeId, AccessibilityRole, BindingEdge, BindingEdgeId,
    BindingEndpoint, BindingEndpointId, ControlGraphNode, ControlKernelRef, ControlKindRef,
    ControlNodeId, ControlPackageRef, ControlPropertySnapshot, ControlPropertySnapshotId,
    InspectionEntry, InspectionEntryId, InteractionHandler, InteractionHandlerId,
    InteractionTrigger, LayoutConstraintId, LayoutGraphNode, RouteCapability, RouteId,
    StatePersistence, StateRequirement, StateRequirementId, StateRequirementLifecycle, StyleRule,
    StyleRuleId, StyleSlotId, UiProgram, UiProgramId, UiProgramSource, UiProgramSourceId,
    UiProgramSourceMapAttachment, UiProgramSourceMapEntry, UiProgramSourceSpan, UiProgramTargetId,
    UiProgramVersion, VisualOperator, VisualOperatorId,
};
use ui_schema::{UiSchemaRef, UiSchemaValue};

pub(crate) fn headless_program(preview_capability: RouteCapability) -> UiProgram {
    let control_id = ControlNodeId::new("control.fixture.title");
    let state_id = StateRequirementId::new("state.fixture.title");
    let state_endpoint = BindingEndpointId::new("binding.fixture.title.state");
    let host_endpoint = BindingEndpointId::new("host.fixture.title");
    let title_schema = UiSchemaRef::new("ui.fixture.title", 1);
    let event_schema = UiSchemaRef::new("ui.fixture.preview", 1);
    let source_map = || {
        UiProgramSourceMapAttachment::new(UiProgramSourceMapEntry::new(
            UiProgramSourceId::new("definition.fixture.title"),
            UiProgramTargetId::new("program.fixture.control.title"),
        ))
        .with_source_span(UiProgramSourceSpan::new(0, 16))
    };

    let mut program = UiProgram::new(
        UiProgramId::new("fixture.headless"),
        UiProgramVersion::new(1),
    )
    .with_source(UiProgramSource::authored(
        UiProgramSourceId::new("definition.fixture"),
        "headless architecture fixture",
    ))
    .with_source_map_entry(UiProgramSourceMapEntry::new(
        UiProgramSourceId::new("definition.fixture"),
        UiProgramTargetId::new("program.fixture"),
    ));

    program.graphs.control.add_node(
        ControlGraphNode::new(
            control_id.clone(),
            ControlPackageRef::new("runenwerk.ui.controls"),
            ControlKindRef::new("runenwerk.ui.controls.label"),
        )
        .with_state_requirement(state_id.clone())
        .with_capability(preview_capability.clone())
        .with_source_map(source_map()),
    );
    program.graphs.properties.add_snapshot(
        ControlPropertySnapshot::new(
            ControlPropertySnapshotId::new("properties.fixture.title"),
            control_id.clone(),
            title_schema.clone(),
            UiSchemaValue::object([("label", UiSchemaValue::string("Inspector"))]),
        )
        .with_source_map(source_map()),
    );
    program.graphs.layout.constraints.push(
        LayoutGraphNode::new(
            LayoutConstraintId::new("layout.fixture.title"),
            control_id.clone(),
        )
        .with_layout_kernel(ControlKernelRef::new("runenwerk.ui.controls.label.layout"))
        .with_source_map(source_map()),
    );
    program.graphs.state.requirements.push(
        StateRequirement::new(
            state_id.clone(),
            control_id.clone(),
            StateRequirementLifecycle::HostFed,
            title_schema.clone(),
        )
        .with_persistence(StatePersistence::HostBacked)
        .with_source_map(source_map()),
    );
    program.graphs.style.rules.push(
        StyleRule::new(
            StyleRuleId::new("style.fixture.title"),
            control_id.clone(),
            StyleSlotId::new("style_slot.fixture.title"),
            title_schema.clone(),
        )
        .with_source_map(source_map()),
    );
    program.graphs.interaction.handlers.push(
        InteractionHandler::new(
            InteractionHandlerId::new("interaction.fixture.preview"),
            control_id.clone(),
            InteractionTrigger::ValuePreview,
            RouteId::new("headless.fixture.preview"),
            event_schema,
        )
        .with_capability(preview_capability)
        .with_source_map(source_map()),
    );
    program.graphs.binding.bindings.push(
        BindingEdge::new(
            BindingEdgeId::new("binding.fixture.title"),
            BindingEndpoint::HostData {
                endpoint_id: host_endpoint,
            },
            BindingEndpoint::UiState {
                requirement_id: state_id,
                endpoint_id: state_endpoint.clone(),
            },
            title_schema.clone(),
        )
        .with_source_map(source_map()),
    );
    program.graphs.visual.operators.push(
        VisualOperator::new(
            VisualOperatorId::new("visual.fixture.title"),
            control_id.clone(),
            ControlKernelRef::new("runenwerk.ui.controls.label.visual"),
        )
        .with_source_map(source_map()),
    );
    program.graphs.accessibility.nodes.push(
        AccessibilityNode::new(
            AccessibilityNodeId::new("accessibility.fixture.title"),
            control_id.clone(),
            AccessibilityRole::Label,
        )
        .with_label_source(state_endpoint.clone())
        .with_source_map(source_map()),
    );
    program.graphs.inspection.entries.push(
        InspectionEntry::new(
            InspectionEntryId::new("inspection.fixture.title"),
            control_id,
            "Fixture title",
            title_schema,
        )
        .with_binding(state_endpoint)
        .with_source_map(source_map()),
    );

    program
}
