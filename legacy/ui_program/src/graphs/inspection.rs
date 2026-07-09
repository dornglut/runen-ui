//! Inspection graph contracts.

use serde::{Deserialize, Serialize};
use ui_schema::UiSchemaRef;

use crate::source_map::UiProgramSourceMapAttachment;

use super::ids::{BindingEndpointId, ControlNodeId, InspectionEntryId};

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct InspectionGraph {
    pub entries: Vec<InspectionEntry>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct InspectionEntry {
    pub entry_id: InspectionEntryId,
    pub control_id: ControlNodeId,
    pub display_name: String,
    pub value_schema: UiSchemaRef,
    #[serde(default)]
    pub binding: Option<BindingEndpointId>,
    #[serde(default)]
    pub source_map: Option<UiProgramSourceMapAttachment>,
}

impl InspectionEntry {
    pub fn new(
        entry_id: InspectionEntryId,
        control_id: ControlNodeId,
        display_name: impl Into<String>,
        value_schema: UiSchemaRef,
    ) -> Self {
        Self {
            entry_id,
            control_id,
            display_name: display_name.into(),
            value_schema,
            binding: None,
            source_map: None,
        }
    }

    pub fn with_binding(mut self, binding: BindingEndpointId) -> Self {
        self.binding = Some(binding);
        self
    }

    pub fn with_source_map(mut self, source_map: UiProgramSourceMapAttachment) -> Self {
        self.source_map = Some(source_map);
        self
    }
}
