use super::*;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BindingSnapshotTable {
    pub rows: Vec<BindingSnapshotRow>,
}

impl BindingSnapshotTable {
    pub(crate) fn from_program(program: &UiProgram, source_map: &CompiledSourceMap) -> Self {
        Self {
            rows: program
                .graphs
                .binding
                .bindings
                .iter()
                .enumerate()
                .map(|(row, binding)| BindingSnapshotRow {
                    binding: binding.clone(),
                    source_map_index: source_map.index_for(RuntimeTableKind::BindingSnapshot, row),
                })
                .collect(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BindingSnapshotRow {
    pub binding: BindingEdge,
    #[serde(default)]
    pub source_map_index: Option<u32>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CollectionDiffPlan {
    pub rows: Vec<CollectionDiffPlanEntry>,
}

impl CollectionDiffPlan {
    pub(crate) fn from_program(program: &UiProgram) -> Self {
        Self {
            rows: program
                .graphs
                .binding
                .bindings
                .iter()
                .map(|binding| CollectionDiffPlanEntry {
                    edge_id: binding.edge_id.as_str().to_owned(),
                    source: RuntimeBindingEndpoint::from_endpoint(&binding.source),
                    target: RuntimeBindingEndpoint::from_endpoint(&binding.target),
                    value_schema: RuntimeSchemaRef::from_schema_ref(&binding.value_schema),
                    strategy: CollectionDiffStrategy::ReplaceValue,
                })
                .collect(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CollectionDiffPlanEntry {
    pub edge_id: String,
    pub source: RuntimeBindingEndpoint,
    pub target: RuntimeBindingEndpoint,
    pub value_schema: RuntimeSchemaRef,
    pub strategy: CollectionDiffStrategy,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CollectionDiffStrategy {
    #[default]
    ReplaceValue,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum RuntimeBindingEndpoint {
    ControlProperty {
        control_id: String,
        endpoint_id: String,
    },
    UiState {
        requirement_id: String,
        endpoint_id: String,
    },
    HostData {
        endpoint_id: String,
    },
}

impl RuntimeBindingEndpoint {
    fn from_endpoint(endpoint: &BindingEndpoint) -> Self {
        match endpoint {
            BindingEndpoint::ControlProperty {
                control_id,
                endpoint_id,
            } => Self::ControlProperty {
                control_id: control_id.as_str().to_owned(),
                endpoint_id: endpoint_id.as_str().to_owned(),
            },
            BindingEndpoint::UiState {
                requirement_id,
                endpoint_id,
            } => Self::UiState {
                requirement_id: requirement_id.as_str().to_owned(),
                endpoint_id: endpoint_id.as_str().to_owned(),
            },
            BindingEndpoint::HostData { endpoint_id } => Self::HostData {
                endpoint_id: endpoint_id.as_str().to_owned(),
            },
        }
    }
}
