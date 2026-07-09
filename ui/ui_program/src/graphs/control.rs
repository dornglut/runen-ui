//! Control graph contracts.

use serde::{Deserialize, Serialize};

use crate::events::RouteCapability;
use crate::source_map::UiProgramSourceMapAttachment;

use super::ids::{ControlKindRef, ControlNodeId, ControlPackageRef, StateRequirementId};

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlGraph {
    pub nodes: Vec<ControlGraphNode>,
}

impl ControlGraph {
    pub fn add_node(&mut self, node: ControlGraphNode) {
        self.nodes.push(node);
    }

    pub fn node(&self, node_id: &ControlNodeId) -> Option<&ControlGraphNode> {
        self.nodes.iter().find(|node| &node.node_id == node_id)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlGraphNode {
    pub node_id: ControlNodeId,
    pub package_id: ControlPackageRef,
    pub control_kind: ControlKindRef,
    #[serde(default)]
    pub parent: Option<ControlNodeId>,
    #[serde(default)]
    pub children: Vec<ControlNodeId>,
    #[serde(default)]
    pub local_state_requirements: Vec<StateRequirementId>,
    #[serde(default)]
    pub required_capabilities: Vec<RouteCapability>,
    #[serde(default)]
    pub source_map: Option<UiProgramSourceMapAttachment>,
}

impl ControlGraphNode {
    pub fn new(
        node_id: ControlNodeId,
        package_id: ControlPackageRef,
        control_kind: ControlKindRef,
    ) -> Self {
        Self {
            node_id,
            package_id,
            control_kind,
            parent: None,
            children: Vec::new(),
            local_state_requirements: Vec::new(),
            required_capabilities: Vec::new(),
            source_map: None,
        }
    }

    pub fn with_parent(mut self, parent: ControlNodeId) -> Self {
        self.parent = Some(parent);
        self
    }

    pub fn with_child(mut self, child: ControlNodeId) -> Self {
        self.children.push(child);
        self
    }

    pub fn with_state_requirement(mut self, requirement: StateRequirementId) -> Self {
        self.local_state_requirements.push(requirement);
        self
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
