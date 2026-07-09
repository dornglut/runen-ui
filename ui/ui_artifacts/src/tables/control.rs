use super::*;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlTable {
    pub rows: Vec<ControlTableRow>,
}

impl ControlTable {
    pub(crate) fn from_program(program: &UiProgram, source_map: &CompiledSourceMap) -> Self {
        Self {
            rows: program
                .graphs
                .control
                .nodes
                .iter()
                .enumerate()
                .map(|(row, node)| ControlTableRow {
                    node: node.clone(),
                    source_map_index: source_map.index_for(RuntimeTableKind::Control, row),
                })
                .collect(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlTableRow {
    pub node: ControlGraphNode,
    #[serde(default)]
    pub source_map_index: Option<u32>,
}
