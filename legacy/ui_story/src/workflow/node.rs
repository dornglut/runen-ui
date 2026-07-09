use serde::{Deserialize, Serialize};

use crate::identity::UiStoryWorkflowNodeId;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum UiStoryWorkflowNodePolicy {
    RequiredEvidence,
    OptionalEvidence,
    Derived,
}

impl UiStoryWorkflowNodePolicy {
    pub const fn requires_evidence(self) -> bool {
        matches!(self, Self::RequiredEvidence)
    }

    pub const fn accepts_evidence(self) -> bool {
        matches!(self, Self::RequiredEvidence | Self::OptionalEvidence)
    }

    pub const fn is_derived(self) -> bool {
        matches!(self, Self::Derived)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct UiStoryWorkflowNode {
    pub node_id: UiStoryWorkflowNodeId,
    pub label: String,
    pub policy: UiStoryWorkflowNodePolicy,
}

impl UiStoryWorkflowNode {
    pub fn new(
        node_id: UiStoryWorkflowNodeId,
        label: impl Into<String>,
        policy: UiStoryWorkflowNodePolicy,
    ) -> Self {
        Self {
            node_id,
            label: label.into(),
            policy,
        }
    }

    pub fn required(node_id: impl Into<String>, label: impl Into<String>) -> Self {
        Self::new(
            UiStoryWorkflowNodeId::new(node_id),
            label,
            UiStoryWorkflowNodePolicy::RequiredEvidence,
        )
    }

    pub fn optional(node_id: impl Into<String>, label: impl Into<String>) -> Self {
        Self::new(
            UiStoryWorkflowNodeId::new(node_id),
            label,
            UiStoryWorkflowNodePolicy::OptionalEvidence,
        )
    }

    pub fn derived(node_id: impl Into<String>, label: impl Into<String>) -> Self {
        Self::new(
            UiStoryWorkflowNodeId::new(node_id),
            label,
            UiStoryWorkflowNodePolicy::Derived,
        )
    }

    pub fn node_id(&self) -> &UiStoryWorkflowNodeId {
        &self.node_id
    }

    pub fn requires_evidence(&self) -> bool {
        self.policy.requires_evidence()
    }

    pub fn accepts_evidence(&self) -> bool {
        self.policy.accepts_evidence()
    }

    pub fn is_derived(&self) -> bool {
        self.policy.is_derived()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn node_policies_expose_evidence_semantics() {
        assert!(UiStoryWorkflowNodePolicy::RequiredEvidence.requires_evidence());
        assert!(UiStoryWorkflowNodePolicy::RequiredEvidence.accepts_evidence());
        assert!(!UiStoryWorkflowNodePolicy::RequiredEvidence.is_derived());
        assert!(!UiStoryWorkflowNodePolicy::OptionalEvidence.requires_evidence());
        assert!(UiStoryWorkflowNodePolicy::OptionalEvidence.accepts_evidence());
        assert!(!UiStoryWorkflowNodePolicy::OptionalEvidence.is_derived());
        assert!(!UiStoryWorkflowNodePolicy::Derived.requires_evidence());
        assert!(!UiStoryWorkflowNodePolicy::Derived.accepts_evidence());
        assert!(UiStoryWorkflowNodePolicy::Derived.is_derived());
    }

    #[test]
    fn node_constructors_preserve_stable_ids() {
        let node = UiStoryWorkflowNode::required("source_load", "Source load");

        assert_eq!(node.node_id.as_str(), "source_load");
        assert_eq!(node.label, "Source load");
        assert!(node.requires_evidence());
    }
}
