use serde::{Deserialize, Serialize};

use crate::diagnostic::UiStoryDiagnostic;
use crate::identity::{UiStoryWorkflowNodeId, UiStoryWorkflowProfileId};

use super::{
    UiStoryWorkflowEdge, UiStoryWorkflowNode, UiStoryWorkflowTopologyError, validate_workflow_graph,
};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiStoryWorkflowGraph {
    pub profile_id: UiStoryWorkflowProfileId,
    pub nodes: Vec<UiStoryWorkflowNode>,
    pub edges: Vec<UiStoryWorkflowEdge>,
    pub terminal_node: UiStoryWorkflowNodeId,
}

impl UiStoryWorkflowGraph {
    pub fn new(
        profile_id: UiStoryWorkflowProfileId,
        nodes: impl IntoIterator<Item = UiStoryWorkflowNode>,
        edges: impl IntoIterator<Item = UiStoryWorkflowEdge>,
        terminal_node: UiStoryWorkflowNodeId,
    ) -> Self {
        Self {
            profile_id,
            nodes: nodes.into_iter().collect(),
            edges: edges.into_iter().collect(),
            terminal_node,
        }
    }

    pub fn profile_id(&self) -> &UiStoryWorkflowProfileId {
        &self.profile_id
    }

    pub fn nodes(&self) -> &[UiStoryWorkflowNode] {
        &self.nodes
    }

    pub fn edges(&self) -> &[UiStoryWorkflowEdge] {
        &self.edges
    }

    pub fn terminal_node(&self) -> &UiStoryWorkflowNodeId {
        &self.terminal_node
    }

    pub fn node(&self, node_id: &UiStoryWorkflowNodeId) -> Option<&UiStoryWorkflowNode> {
        self.nodes.iter().find(|node| &node.node_id == node_id)
    }

    pub fn validate(&self) -> Vec<UiStoryDiagnostic> {
        validate_workflow_graph(self)
    }

    pub fn is_valid(&self) -> bool {
        self.validate().is_empty()
    }

    pub fn topological_nodes(
        &self,
    ) -> Result<Vec<&UiStoryWorkflowNode>, UiStoryWorkflowTopologyError> {
        super::topo::topological_nodes(self)
    }
}
