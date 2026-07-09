use super::*;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StateTable {
    pub rows: Vec<StateTableRow>,
}

impl StateTable {
    pub(crate) fn from_program(program: &UiProgram, source_map: &CompiledSourceMap) -> Self {
        Self {
            rows: program
                .graphs
                .state
                .requirements
                .iter()
                .enumerate()
                .map(|(row, requirement)| StateTableRow {
                    requirement: requirement.clone(),
                    source_map_index: source_map.index_for(RuntimeTableKind::State, row),
                })
                .collect(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StateTableRow {
    pub requirement: StateRequirement,
    #[serde(default)]
    pub source_map_index: Option<u32>,
}
