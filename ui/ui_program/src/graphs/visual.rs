//! Visual graph contracts.

use serde::{Deserialize, Serialize};

use crate::source_map::UiProgramSourceMapAttachment;

use super::ids::{ControlKernelRef, ControlNodeId, VisualOperatorId};

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisualGraph {
    pub operators: Vec<VisualOperator>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisualOperator {
    pub operator_id: VisualOperatorId,
    pub control_id: ControlNodeId,
    pub visual_kernel: ControlKernelRef,
    #[serde(default)]
    pub input_dependencies: Vec<ControlNodeId>,
    #[serde(default)]
    pub source_map: Option<UiProgramSourceMapAttachment>,
}

impl VisualOperator {
    pub fn new(
        operator_id: VisualOperatorId,
        control_id: ControlNodeId,
        visual_kernel: ControlKernelRef,
    ) -> Self {
        Self {
            operator_id,
            control_id,
            visual_kernel,
            input_dependencies: Vec::new(),
            source_map: None,
        }
    }

    pub fn with_input_dependency(mut self, control_id: ControlNodeId) -> Self {
        self.input_dependencies.push(control_id);
        self
    }

    pub fn with_source_map(mut self, source_map: UiProgramSourceMapAttachment) -> Self {
        self.source_map = Some(source_map);
        self
    }
}
