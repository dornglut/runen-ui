use super::*;

pub(crate) fn runtime_capabilities(program: &UiProgram) -> Vec<RuntimeCapabilityRecord> {
    let mut capabilities = BTreeMap::<String, RuntimeCapabilityRecord>::new();
    for node in &program.graphs.control.nodes {
        for capability in &node.required_capabilities {
            let record = capability_record(&mut capabilities, capability.as_str());
            push_unique(&mut record.declared_by_controls, node.node_id.as_str());
        }
    }
    for handler in &program.graphs.interaction.handlers {
        for capability in &handler.required_capabilities {
            let record = capability_record(&mut capabilities, capability.as_str());
            push_unique(
                &mut record.required_by_interactions,
                handler.handler_id.as_str(),
            );
        }
    }
    for binding in &program.graphs.binding.bindings {
        for capability in &binding.required_capabilities {
            let record = capability_record(&mut capabilities, capability.as_str());
            push_unique(&mut record.required_by_bindings, binding.edge_id.as_str());
        }
    }
    capabilities.into_values().collect()
}

fn capability_record<'a>(
    capabilities: &'a mut BTreeMap<String, RuntimeCapabilityRecord>,
    capability_id: &str,
) -> &'a mut RuntimeCapabilityRecord {
    capabilities
        .entry(capability_id.to_owned())
        .or_insert_with(|| RuntimeCapabilityRecord {
            capability_id: capability_id.to_owned(),
            ..RuntimeCapabilityRecord::default()
        })
}

fn push_unique(values: &mut Vec<String>, value: &str) {
    if !values.iter().any(|existing| existing == value) {
        values.push(value.to_owned());
    }
}
