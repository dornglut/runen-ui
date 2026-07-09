//! Accessibility graph contracts.

use serde::{Deserialize, Serialize};

use crate::source_map::UiProgramSourceMapAttachment;

use super::ids::{AccessibilityNodeId, BindingEndpointId, ControlNodeId};

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccessibilityGraph {
    pub nodes: Vec<AccessibilityNode>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccessibilityNode {
    pub node_id: AccessibilityNodeId,
    pub control_id: ControlNodeId,
    pub role: AccessibilityRole,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub label_source: Option<BindingEndpointId>,
    #[serde(default)]
    pub source_map: Option<UiProgramSourceMapAttachment>,
}

impl AccessibilityNode {
    pub fn new(
        node_id: AccessibilityNodeId,
        control_id: ControlNodeId,
        role: AccessibilityRole,
    ) -> Self {
        Self {
            node_id,
            control_id,
            role,
            label: None,
            label_source: None,
            source_map: None,
        }
    }

    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = Some(label.into());
        self
    }

    pub fn with_label_source(mut self, label_source: BindingEndpointId) -> Self {
        self.label_source = Some(label_source);
        self
    }

    pub fn with_source_map(mut self, source_map: UiProgramSourceMapAttachment) -> Self {
        self.source_map = Some(source_map);
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum AccessibilityRole {
    Label,
    Button,
    TextField,
    ColorPicker,
    List,
    Tree,
    Table,
    Prompt,
}
