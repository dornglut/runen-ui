use super::*;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisualOperatorTable {
    pub rows: Vec<VisualOperatorRow>,
}

impl VisualOperatorTable {
    pub(crate) fn from_program(program: &UiProgram, source_map: &CompiledSourceMap) -> Self {
        Self {
            rows: program
                .graphs
                .visual
                .operators
                .iter()
                .enumerate()
                .map(|(row, operator)| VisualOperatorRow {
                    operator: operator.clone(),
                    source_map_index: source_map.index_for(RuntimeTableKind::Visual, row),
                })
                .collect(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisualOperatorRow {
    pub operator: VisualOperator,
    #[serde(default)]
    pub source_map_index: Option<u32>,
}
