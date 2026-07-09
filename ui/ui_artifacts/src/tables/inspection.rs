use super::*;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct InspectionTable {
    pub rows: Vec<InspectionRow>,
}

impl InspectionTable {
    pub(crate) fn from_program(program: &UiProgram, source_map: &CompiledSourceMap) -> Self {
        Self {
            rows: program
                .graphs
                .inspection
                .entries
                .iter()
                .enumerate()
                .map(|(row, entry)| InspectionRow {
                    entry: entry.clone(),
                    source_map_index: source_map.index_for(RuntimeTableKind::Inspection, row),
                })
                .collect(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct InspectionRow {
    pub entry: InspectionEntry,
    #[serde(default)]
    pub source_map_index: Option<u32>,
}
