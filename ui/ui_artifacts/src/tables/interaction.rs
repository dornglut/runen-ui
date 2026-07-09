use super::*;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct InteractionDispatchTable {
    pub rows: Vec<InteractionDispatchRow>,
}

impl InteractionDispatchTable {
    pub(crate) fn from_program(program: &UiProgram, source_map: &CompiledSourceMap) -> Self {
        Self {
            rows: program
                .graphs
                .interaction
                .handlers
                .iter()
                .enumerate()
                .map(|(row, handler)| InteractionDispatchRow {
                    handler: handler.clone(),
                    source_map_index: source_map.index_for(RuntimeTableKind::Interaction, row),
                })
                .collect(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct InteractionDispatchRow {
    pub handler: InteractionHandler,
    #[serde(default)]
    pub source_map_index: Option<u32>,
}
