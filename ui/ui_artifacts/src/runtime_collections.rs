use super::*;

pub(crate) fn sorted_control_kind_ids(program: &UiProgram) -> Vec<String> {
    program
        .graphs
        .control
        .nodes
        .iter()
        .map(|node| node.control_kind.as_str().to_owned())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

pub(crate) fn sorted_kernel_ids(program: &UiProgram) -> Vec<String> {
    let mut kernel_ids = BTreeSet::new();
    for constraint in &program.graphs.layout.constraints {
        if let Some(kernel) = constraint.layout_kernel.as_ref() {
            kernel_ids.insert(kernel.as_str().to_owned());
        }
    }
    for operator in &program.graphs.visual.operators {
        kernel_ids.insert(operator.visual_kernel.as_str().to_owned());
    }
    kernel_ids.into_iter().collect()
}

pub(crate) fn sorted_schema_ids(program: &UiProgram) -> Vec<RuntimeSchemaRef> {
    let mut schemas = BTreeSet::new();
    for requirement in &program.graphs.state.requirements {
        schemas.insert(RuntimeSchemaRef::from_schema_ref(&requirement.schema));
    }
    for snapshot in &program.graphs.properties.rows {
        schemas.insert(RuntimeSchemaRef::from_schema_ref(&snapshot.schema));
    }
    for rule in &program.graphs.style.rules {
        schemas.insert(RuntimeSchemaRef::from_schema_ref(&rule.property_schema));
    }
    for handler in &program.graphs.interaction.handlers {
        schemas.insert(RuntimeSchemaRef::from_schema_ref(&handler.payload_schema));
    }
    for binding in &program.graphs.binding.bindings {
        schemas.insert(RuntimeSchemaRef::from_schema_ref(&binding.value_schema));
    }
    for inspection in &program.graphs.inspection.entries {
        schemas.insert(RuntimeSchemaRef::from_schema_ref(&inspection.value_schema));
    }
    schemas.into_iter().collect()
}

pub(crate) fn sorted_route_ids(program: &UiProgram) -> Vec<RuntimeRouteRef> {
    program
        .graphs
        .interaction
        .handlers
        .iter()
        .map(|handler| RuntimeRouteRef {
            route_id: handler.route.as_str().to_owned(),
            payload_schema: RuntimeSchemaRef::from_schema_ref(&handler.payload_schema),
        })
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}
