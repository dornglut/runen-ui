use serde::{Deserialize, Serialize};

use crate::diagnostic::UiStoryDiagnostic;
use crate::identity::UiStoryId;

use super::UiStoryOutcomeV2;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiStoryWorkflowReportSummaryV2 {
    pub story_id: UiStoryId,
    pub outcome: UiStoryOutcomeV2,
    pub diagnostic_count: usize,
    pub blocking_diagnostic_count: usize,
    pub node_count: usize,
    pub blocked_node_count: usize,
}

impl UiStoryWorkflowReportSummaryV2 {
    pub(crate) fn new(
        story_id: UiStoryId,
        outcome: UiStoryOutcomeV2,
        diagnostics: &[UiStoryDiagnostic],
        node_count: usize,
        blocked_node_count: usize,
    ) -> Self {
        Self {
            story_id,
            outcome,
            diagnostic_count: diagnostics.len(),
            blocking_diagnostic_count: diagnostics
                .iter()
                .filter(|diagnostic| diagnostic.is_blocking())
                .count(),
            node_count,
            blocked_node_count,
        }
    }
}
