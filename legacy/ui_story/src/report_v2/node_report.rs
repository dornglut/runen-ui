use serde::{Deserialize, Serialize};

use crate::diagnostic::{UiStoryDiagnostic, UiStoryDiagnosticSubject};
use crate::evidence::UiStoryEvidence;
use crate::identity::UiStoryWorkflowNodeId;
use crate::workflow::UiStoryWorkflowNodePolicy;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiStoryWorkflowNodeReportV2 {
    pub node_id: UiStoryWorkflowNodeId,
    pub policy: UiStoryWorkflowNodePolicy,
    #[serde(default)]
    pub evidence: Vec<UiStoryEvidence>,
    #[serde(default)]
    pub diagnostics: Vec<UiStoryDiagnostic>,
    pub missing_required: bool,
    pub blocked_by_dependency: bool,
}

impl UiStoryWorkflowNodeReportV2 {
    pub fn new(
        node_id: UiStoryWorkflowNodeId,
        policy: UiStoryWorkflowNodePolicy,
        evidence: Vec<UiStoryEvidence>,
        diagnostics: Vec<UiStoryDiagnostic>,
        missing_required: bool,
        blocked_by_dependency: bool,
    ) -> Self {
        Self {
            node_id,
            policy,
            evidence,
            diagnostics,
            missing_required,
            blocked_by_dependency,
        }
    }

    pub fn has_blockers(&self) -> bool {
        self.missing_required
            || self.blocked_by_dependency
            || self.diagnostics.iter().any(UiStoryDiagnostic::is_blocking)
            || self.evidence.iter().any(UiStoryEvidence::blocks_node)
    }

    pub fn passed_without_blockers(&self) -> bool {
        !self.has_blockers()
            && self
                .evidence
                .iter()
                .any(UiStoryEvidence::passed_without_blockers)
    }
}

pub fn diagnostic_belongs_to_node(
    diagnostic: &UiStoryDiagnostic,
    node_id: &UiStoryWorkflowNodeId,
) -> bool {
    matches!(
        &diagnostic.subject,
        UiStoryDiagnosticSubject::WorkflowNode(subject_node_id) if subject_node_id == node_id
    )
}
