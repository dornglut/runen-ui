//! Layout graph contracts.

use serde::{Deserialize, Serialize};

use crate::source_map::UiProgramSourceMapAttachment;

use super::ids::{ControlKernelRef, ControlNodeId, LayoutConstraintId};

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LayoutGraph {
    pub constraints: Vec<LayoutGraphNode>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct LayoutGraphNode {
    pub constraint_id: LayoutConstraintId,
    pub target_control: ControlNodeId,
    #[serde(default)]
    pub measurement_dependencies: Vec<ControlNodeId>,
    #[serde(default)]
    pub layout_kernel: Option<ControlKernelRef>,
    #[serde(default)]
    pub source_map: Option<UiProgramSourceMapAttachment>,
}

impl LayoutGraphNode {
    pub fn new(constraint_id: LayoutConstraintId, target_control: ControlNodeId) -> Self {
        Self {
            constraint_id,
            target_control,
            measurement_dependencies: Vec::new(),
            layout_kernel: None,
            source_map: None,
        }
    }

    pub fn with_measurement_dependency(mut self, control_id: ControlNodeId) -> Self {
        self.measurement_dependencies.push(control_id);
        self
    }

    pub fn with_layout_kernel(mut self, kernel: ControlKernelRef) -> Self {
        self.layout_kernel = Some(kernel);
        self
    }

    pub fn with_source_map(mut self, source_map: UiProgramSourceMapAttachment) -> Self {
        self.source_map = Some(source_map);
        self
    }
}
