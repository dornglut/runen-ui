use super::*;

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ControlPropertyTable {
    pub rows: Vec<ControlPropertyRow>,
}

impl ControlPropertyTable {
    pub(crate) fn from_program(program: &UiProgram, source_map: &CompiledSourceMap) -> Self {
        Self {
            rows: program
                .graphs
                .properties
                .rows
                .iter()
                .enumerate()
                .map(|(row, snapshot)| ControlPropertyRow {
                    snapshot: snapshot.clone(),
                    source_map_index: source_map
                        .index_for(RuntimeTableKind::ControlProperties, row),
                })
                .collect(),
        }
    }
}

impl Eq for ControlPropertyTable {}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ControlPropertyRow {
    pub snapshot: ControlPropertySnapshot,
    #[serde(default)]
    pub source_map_index: Option<u32>,
}

impl Eq for ControlPropertyRow {}
