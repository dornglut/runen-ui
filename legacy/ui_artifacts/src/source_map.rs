use super::*;

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompiledSourceMap {
    pub entries: Vec<CompiledSourceMapEntry>,
}

impl CompiledSourceMap {
    pub fn from_program(program: &UiProgram) -> Self {
        let mut entries = Vec::new();

        for (row, entry) in program.source_map.iter().enumerate() {
            entries.push(CompiledSourceMapEntry::from_entry(
                entry,
                RuntimeTableKind::Program,
                row as u32,
                None,
                None,
            ));
        }
        for (row, node) in program.graphs.control.nodes.iter().enumerate() {
            push_attachment_entry(
                &mut entries,
                RuntimeTableKind::Control,
                row,
                node.source_map.as_ref(),
            );
        }
        for (row, snapshot) in program.graphs.properties.rows.iter().enumerate() {
            push_attachment_entry(
                &mut entries,
                RuntimeTableKind::ControlProperties,
                row,
                snapshot.source_map.as_ref(),
            );
        }
        for (row, node) in program.graphs.layout.constraints.iter().enumerate() {
            push_attachment_entry(
                &mut entries,
                RuntimeTableKind::Layout,
                row,
                node.source_map.as_ref(),
            );
        }
        for (row, requirement) in program.graphs.state.requirements.iter().enumerate() {
            push_attachment_entry(
                &mut entries,
                RuntimeTableKind::State,
                row,
                requirement.source_map.as_ref(),
            );
        }
        for (row, rule) in program.graphs.style.rules.iter().enumerate() {
            push_attachment_entry(
                &mut entries,
                RuntimeTableKind::Style,
                row,
                rule.source_map.as_ref(),
            );
        }
        for (row, handler) in program.graphs.interaction.handlers.iter().enumerate() {
            push_attachment_entry(
                &mut entries,
                RuntimeTableKind::Interaction,
                row,
                handler.source_map.as_ref(),
            );
        }
        for (row, binding) in program.graphs.binding.bindings.iter().enumerate() {
            push_attachment_entry(
                &mut entries,
                RuntimeTableKind::BindingSnapshot,
                row,
                binding.source_map.as_ref(),
            );
        }
        for (row, operator) in program.graphs.visual.operators.iter().enumerate() {
            push_attachment_entry(
                &mut entries,
                RuntimeTableKind::Visual,
                row,
                operator.source_map.as_ref(),
            );
        }
        for (row, node) in program.graphs.accessibility.nodes.iter().enumerate() {
            push_attachment_entry(
                &mut entries,
                RuntimeTableKind::Accessibility,
                row,
                node.source_map.as_ref(),
            );
        }
        for (row, entry) in program.graphs.inspection.entries.iter().enumerate() {
            push_attachment_entry(
                &mut entries,
                RuntimeTableKind::Inspection,
                row,
                entry.source_map.as_ref(),
            );
        }

        Self { entries }
    }

    pub fn index_for(&self, table: RuntimeTableKind, row: usize) -> Option<u32> {
        self.entries
            .iter()
            .position(|entry| entry.table == table && entry.row == row as u32)
            .map(|index| index as u32)
    }

    pub fn index_for_entry(&self, source_map: &UiProgramSourceMapEntry) -> Option<u32> {
        self.entries
            .iter()
            .position(|entry| {
                entry.source_id == source_map.source_id && entry.target_id == source_map.target_id
            })
            .map(|index| index as u32)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompiledSourceMapEntry {
    pub source_id: String,
    pub target_id: String,
    pub table: RuntimeTableKind,
    pub row: u32,
    #[serde(default)]
    pub source_span: Option<UiProgramSourceSpan>,
    #[serde(default)]
    pub generated_by: Option<String>,
}

impl CompiledSourceMapEntry {
    pub(crate) fn from_entry(
        entry: &UiProgramSourceMapEntry,
        table: RuntimeTableKind,
        row: u32,
        source_span: Option<UiProgramSourceSpan>,
        generated_by: Option<String>,
    ) -> Self {
        Self {
            source_id: entry.source_id.to_owned(),
            target_id: entry.target_id.to_owned(),
            table,
            row,
            source_span,
            generated_by,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RuntimeTableKind {
    Program,
    Control,
    ControlProperties,
    Layout,
    Style,
    State,
    Interaction,
    BindingSnapshot,
    CollectionDiff,
    Visual,
    TextLayoutRequest,
    Accessibility,
    Inspection,
}
