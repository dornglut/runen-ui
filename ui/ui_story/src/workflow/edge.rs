use serde::{Deserialize, Serialize};

use crate::identity::UiStoryWorkflowNodeId;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum UiStoryWorkflowDependency {
    RequiresCompleted,
}

impl UiStoryWorkflowDependency {
    pub const fn blocks_downstream_when_upstream_missing(self) -> bool {
        matches!(self, Self::RequiresCompleted)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct UiStoryWorkflowEdge {
    pub from: UiStoryWorkflowNodeId,
    pub to: UiStoryWorkflowNodeId,
    pub dependency: UiStoryWorkflowDependency,
}

impl UiStoryWorkflowEdge {
    pub fn new(
        from: UiStoryWorkflowNodeId,
        to: UiStoryWorkflowNodeId,
        dependency: UiStoryWorkflowDependency,
    ) -> Self {
        Self {
            from,
            to,
            dependency,
        }
    }

    pub fn requires_completed(from: impl Into<String>, to: impl Into<String>) -> Self {
        Self::new(
            UiStoryWorkflowNodeId::new(from),
            UiStoryWorkflowNodeId::new(to),
            UiStoryWorkflowDependency::RequiresCompleted,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn requires_completed_dependency_blocks_missing_upstream() {
        assert!(
            UiStoryWorkflowDependency::RequiresCompleted.blocks_downstream_when_upstream_missing()
        );
    }

    #[test]
    fn edge_constructor_preserves_endpoints() {
        let edge = UiStoryWorkflowEdge::requires_completed("source_load", "source_parse");

        assert_eq!(edge.from.as_str(), "source_load");
        assert_eq!(edge.to.as_str(), "source_parse");
    }
}
