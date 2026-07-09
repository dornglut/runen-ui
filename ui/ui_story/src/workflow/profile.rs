use serde::{Deserialize, Serialize};

use crate::identity::UiStoryWorkflowProfileId;

use super::UiStoryWorkflowGraph;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiStoryWorkflowProfile {
    pub profile_id: UiStoryWorkflowProfileId,
    pub graph: UiStoryWorkflowGraph,
}

impl UiStoryWorkflowProfile {
    pub fn new(profile_id: UiStoryWorkflowProfileId, graph: UiStoryWorkflowGraph) -> Self {
        Self { profile_id, graph }
    }

    pub fn profile_id(&self) -> &UiStoryWorkflowProfileId {
        &self.profile_id
    }

    pub fn graph(&self) -> &UiStoryWorkflowGraph {
        &self.graph
    }

    pub fn into_graph(self) -> UiStoryWorkflowGraph {
        self.graph
    }
}

#[cfg(test)]
mod tests {
    use crate::identity::{UiStoryWorkflowNodeId, UiStoryWorkflowProfileId};
    use crate::workflow::{UiStoryWorkflowGraph, UiStoryWorkflowNode};

    use super::*;

    #[test]
    fn profile_wraps_graph_with_stable_id() {
        let profile_id = UiStoryWorkflowProfileId::new("ui_story.workflow.source_load_only");
        let graph = UiStoryWorkflowGraph::new(
            profile_id.clone(),
            [UiStoryWorkflowNode::required("source_load", "Source load")],
            [],
            UiStoryWorkflowNodeId::new("source_load"),
        );

        let profile = UiStoryWorkflowProfile::new(profile_id.clone(), graph);

        assert_eq!(profile.profile_id(), &profile_id);
        assert_eq!(profile.graph().profile_id(), &profile_id);
    }
}
