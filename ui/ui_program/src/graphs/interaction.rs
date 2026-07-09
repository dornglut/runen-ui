//! Interaction graph contracts.

use serde::{Deserialize, Serialize};
use ui_schema::UiSchemaRef;

use crate::events::{RouteCapability, RouteId};
use crate::source_map::UiProgramSourceMapAttachment;

use super::ids::{ControlNodeId, InteractionHandlerId};

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct InteractionGraph {
    pub handlers: Vec<InteractionHandler>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct InteractionHandler {
    pub handler_id: InteractionHandlerId,
    pub control_id: ControlNodeId,
    pub trigger: InteractionTrigger,
    pub route: RouteId,
    pub payload_schema: UiSchemaRef,
    #[serde(default)]
    pub required_capabilities: Vec<RouteCapability>,
    #[serde(default)]
    pub source_map: Option<UiProgramSourceMapAttachment>,
}

impl InteractionHandler {
    pub fn new(
        handler_id: InteractionHandlerId,
        control_id: ControlNodeId,
        trigger: InteractionTrigger,
        route: RouteId,
        payload_schema: UiSchemaRef,
    ) -> Self {
        Self {
            handler_id,
            control_id,
            trigger,
            route,
            payload_schema,
            required_capabilities: Vec::new(),
            source_map: None,
        }
    }

    pub fn with_capability(mut self, capability: RouteCapability) -> Self {
        self.required_capabilities.push(capability);
        self
    }

    pub fn with_source_map(mut self, source_map: UiProgramSourceMapAttachment) -> Self {
        self.source_map = Some(source_map);
        self
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum InteractionTrigger {
    Press,
    Release,
    PointerMove,
    TextCommit,
    SelectionChange,
    ValuePreview,
    ValueCommit,
    Cancel,
}
