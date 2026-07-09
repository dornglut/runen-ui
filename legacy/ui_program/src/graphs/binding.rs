//! Binding graph contracts.

use serde::{Deserialize, Serialize};
use ui_schema::UiSchemaRef;

use crate::events::RouteCapability;
use crate::source_map::UiProgramSourceMapAttachment;

use super::ids::{BindingEdgeId, BindingEndpointId, ControlNodeId, StateRequirementId};

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct BindingGraph {
    pub bindings: Vec<BindingEdge>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BindingEdge {
    pub edge_id: BindingEdgeId,
    pub source: BindingEndpoint,
    pub target: BindingEndpoint,
    pub value_schema: UiSchemaRef,
    #[serde(default)]
    pub required_capabilities: Vec<RouteCapability>,
    #[serde(default)]
    pub source_map: Option<UiProgramSourceMapAttachment>,
}

impl BindingEdge {
    pub fn new(
        edge_id: BindingEdgeId,
        source: BindingEndpoint,
        target: BindingEndpoint,
        value_schema: UiSchemaRef,
    ) -> Self {
        Self {
            edge_id,
            source,
            target,
            value_schema,
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

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum BindingEndpoint {
    ControlProperty {
        control_id: ControlNodeId,
        endpoint_id: BindingEndpointId,
    },
    UiState {
        requirement_id: StateRequirementId,
        endpoint_id: BindingEndpointId,
    },
    HostData {
        endpoint_id: BindingEndpointId,
    },
}
