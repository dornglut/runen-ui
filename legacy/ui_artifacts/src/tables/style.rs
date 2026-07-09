use super::*;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StyleResolutionTable {
    pub rows: Vec<StyleResolutionRow>,
}

impl StyleResolutionTable {
    pub(crate) fn from_program(program: &UiProgram, source_map: &CompiledSourceMap) -> Self {
        Self {
            rows: program
                .graphs
                .style
                .rules
                .iter()
                .enumerate()
                .map(|(row, rule)| StyleResolutionRow {
                    rule: rule.clone(),
                    source_map_index: source_map.index_for(RuntimeTableKind::Style, row),
                })
                .collect(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StyleResolutionRow {
    pub rule: StyleRule,
    #[serde(default)]
    pub source_map_index: Option<u32>,
}
