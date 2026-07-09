//! State graph contracts.

use serde::{Deserialize, Serialize};
use ui_schema::UiSchemaRef;

use crate::source_map::UiProgramSourceMapAttachment;

use super::ids::{ControlNodeId, StateRequirementId};

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StateGraph {
    pub requirements: Vec<StateRequirement>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StateRequirement {
    pub requirement_id: StateRequirementId,
    pub owner_control: ControlNodeId,
    pub lifecycle: StateRequirementLifecycle,
    pub schema: UiSchemaRef,
    pub participates_in_evaluation: bool,
    pub persistence: StatePersistence,
    #[serde(default)]
    pub invalidates: Vec<StateRequirementId>,
    #[serde(default)]
    pub source_map: Option<UiProgramSourceMapAttachment>,
}

impl StateRequirement {
    pub fn new(
        requirement_id: StateRequirementId,
        owner_control: ControlNodeId,
        lifecycle: StateRequirementLifecycle,
        schema: UiSchemaRef,
    ) -> Self {
        Self {
            requirement_id,
            owner_control,
            lifecycle,
            schema,
            participates_in_evaluation: true,
            persistence: StatePersistence::Ephemeral,
            invalidates: Vec::new(),
            source_map: None,
        }
    }

    pub fn with_persistence(mut self, persistence: StatePersistence) -> Self {
        self.persistence = persistence;
        self
    }

    pub fn with_invalidation(mut self, requirement_id: StateRequirementId) -> Self {
        self.invalidates.push(requirement_id);
        self
    }

    pub fn with_source_map(mut self, source_map: UiProgramSourceMapAttachment) -> Self {
        self.source_map = Some(source_map);
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum StateRequirementLifecycle {
    Transient,
    Preview,
    Committed,
    Focus,
    Hover,
    PressedCaptured,
    Drag,
    Animation,
    HostFed,
    PackageOwned,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum StatePersistence {
    #[default]
    Ephemeral,
    Retained,
    HostBacked,
}
