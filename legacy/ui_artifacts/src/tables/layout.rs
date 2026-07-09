use super::*;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LayoutPlanTable {
    pub rows: Vec<LayoutPlanRow>,
}

impl LayoutPlanTable {
    pub(crate) fn from_program(program: &UiProgram, source_map: &CompiledSourceMap) -> Self {
        Self {
            rows: program
                .graphs
                .layout
                .constraints
                .iter()
                .enumerate()
                .map(|(row, constraint)| LayoutPlanRow {
                    constraint: constraint.clone(),
                    source_map_index: source_map.index_for(RuntimeTableKind::Layout, row),
                })
                .collect(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LayoutPlanRow {
    pub constraint: LayoutGraphNode,
    #[serde(default)]
    pub source_map_index: Option<u32>,
}
