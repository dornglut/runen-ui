use super::*;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccessibilityTable {
    pub rows: Vec<AccessibilityRow>,
}

impl AccessibilityTable {
    pub(crate) fn from_program(program: &UiProgram, source_map: &CompiledSourceMap) -> Self {
        Self {
            rows: program
                .graphs
                .accessibility
                .nodes
                .iter()
                .enumerate()
                .map(|(row, node)| AccessibilityRow {
                    node: node.clone(),
                    source_map_index: source_map.index_for(RuntimeTableKind::Accessibility, row),
                })
                .collect(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccessibilityRow {
    pub node: AccessibilityNode,
    #[serde(default)]
    pub source_map_index: Option<u32>,
}
