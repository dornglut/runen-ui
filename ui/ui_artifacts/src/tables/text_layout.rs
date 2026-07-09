use super::*;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextLayoutRequestTable {
    pub rows: Vec<TextLayoutRequest>,
}

impl TextLayoutRequestTable {
    pub(crate) fn from_program(program: &UiProgram, source_map: &CompiledSourceMap) -> Self {
        let mut rows = BTreeMap::<String, TextLayoutRequest>::new();
        for (row, control) in program.graphs.control.nodes.iter().enumerate() {
            let control_kind = control.control_kind.as_str();
            if control_kind.ends_with(".label") || control_kind.contains(".text") {
                rows.insert(
                    control.node_id.as_str().to_owned(),
                    TextLayoutRequest {
                        request_id: format!(
                            "text_layout.{}",
                            control.node_id.as_str().replace('.', "_")
                        ),
                        control_id: control.node_id.clone(),
                        text_source: None,
                        source_map_index: source_map.index_for(RuntimeTableKind::Control, row),
                    },
                );
            }
        }
        for (row, accessibility) in program.graphs.accessibility.nodes.iter().enumerate() {
            if let Some(label_source) = accessibility.label_source.as_ref() {
                let source_map_index = source_map.index_for(RuntimeTableKind::Accessibility, row);
                rows.entry(accessibility.control_id.as_str().to_owned())
                    .and_modify(|request| {
                        request.text_source = Some(label_source.clone());
                        if request.source_map_index.is_none() {
                            request.source_map_index = source_map_index;
                        }
                    })
                    .or_insert_with(|| TextLayoutRequest {
                        request_id: format!(
                            "text_layout.{}",
                            accessibility.control_id.as_str().replace('.', "_")
                        ),
                        control_id: accessibility.control_id.clone(),
                        text_source: Some(label_source.clone()),
                        source_map_index,
                    });
            }
        }
        Self {
            rows: rows.into_values().collect(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct TextLayoutRequest {
    pub request_id: String,
    pub control_id: ControlNodeId,
    #[serde(default)]
    pub text_source: Option<BindingEndpointId>,
    #[serde(default)]
    pub source_map_index: Option<u32>,
}
