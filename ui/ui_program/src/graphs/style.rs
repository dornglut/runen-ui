//! Style graph contracts.

use serde::{Deserialize, Serialize};
use ui_schema::UiSchemaRef;

use crate::source_map::UiProgramSourceMapAttachment;

use super::ids::{ControlNodeId, StyleRuleId, StyleSlotId};

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StyleGraph {
    pub rules: Vec<StyleRule>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StyleRule {
    pub rule_id: StyleRuleId,
    pub target_control: ControlNodeId,
    pub style_slot: StyleSlotId,
    pub property_schema: UiSchemaRef,
    #[serde(default)]
    pub source_map: Option<UiProgramSourceMapAttachment>,
}

impl StyleRule {
    pub fn new(
        rule_id: StyleRuleId,
        target_control: ControlNodeId,
        style_slot: StyleSlotId,
        property_schema: UiSchemaRef,
    ) -> Self {
        Self {
            rule_id,
            target_control,
            style_slot,
            property_schema,
            source_map: None,
        }
    }

    pub fn with_source_map(mut self, source_map: UiProgramSourceMapAttachment) -> Self {
        self.source_map = Some(source_map);
        self
    }
}
