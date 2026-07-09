use serde::{Deserialize, Serialize};

use crate::diagnostic::UiStoryDiagnostic;
use crate::identity::{UiStoryEvidenceKey, UiStoryEvidenceProducerId, UiStoryWorkflowNodeId};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum UiStoryEvidenceStatus {
    Passed,
    Failed,
    Skipped,
    Blocked,
}

impl UiStoryEvidenceStatus {
    pub const fn blocks_node(self) -> bool {
        matches!(self, Self::Failed | Self::Blocked)
    }

    pub const fn is_passed(self) -> bool {
        matches!(self, Self::Passed)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct UiStoryEvidence {
    pub workflow_node_id: UiStoryWorkflowNodeId,
    pub producer_id: UiStoryEvidenceProducerId,
    pub evidence_key: UiStoryEvidenceKey,
    pub status: UiStoryEvidenceStatus,
    #[serde(default)]
    pub diagnostics: Vec<UiStoryDiagnostic>,
    pub elapsed_micros: Option<u64>,
}

impl UiStoryEvidence {
    pub fn new(
        workflow_node_id: UiStoryWorkflowNodeId,
        producer_id: UiStoryEvidenceProducerId,
        evidence_key: UiStoryEvidenceKey,
        status: UiStoryEvidenceStatus,
        diagnostics: Vec<UiStoryDiagnostic>,
    ) -> Self {
        Self {
            workflow_node_id,
            producer_id,
            evidence_key,
            status,
            diagnostics,
            elapsed_micros: None,
        }
    }

    pub fn from_strings(
        workflow_node_id: impl Into<String>,
        producer_id: impl Into<String>,
        evidence_key: impl Into<String>,
        status: UiStoryEvidenceStatus,
        diagnostics: Vec<UiStoryDiagnostic>,
    ) -> Self {
        Self::new(
            UiStoryWorkflowNodeId::new(workflow_node_id),
            UiStoryEvidenceProducerId::new(producer_id),
            UiStoryEvidenceKey::new(evidence_key),
            status,
            diagnostics,
        )
    }

    pub fn from_result(
        workflow_node_id: impl Into<String>,
        producer_id: impl Into<String>,
        evidence_key: impl Into<String>,
        passed: bool,
        diagnostics: Vec<UiStoryDiagnostic>,
    ) -> Self {
        let status = if passed {
            UiStoryEvidenceStatus::Passed
        } else {
            UiStoryEvidenceStatus::Failed
        };
        Self::from_strings(
            workflow_node_id,
            producer_id,
            evidence_key,
            status,
            diagnostics,
        )
    }

    pub fn failed_with_diagnostic(
        workflow_node_id: impl Into<String>,
        producer_id: impl Into<String>,
        evidence_key: impl Into<String>,
        diagnostic: UiStoryDiagnostic,
    ) -> Self {
        Self::failed(
            workflow_node_id,
            producer_id,
            evidence_key,
            vec![diagnostic],
        )
    }

    pub fn passed(
        workflow_node_id: impl Into<String>,
        producer_id: impl Into<String>,
        evidence_key: impl Into<String>,
    ) -> Self {
        Self::new(
            UiStoryWorkflowNodeId::new(workflow_node_id),
            UiStoryEvidenceProducerId::new(producer_id),
            UiStoryEvidenceKey::new(evidence_key),
            UiStoryEvidenceStatus::Passed,
            Vec::new(),
        )
    }

    pub fn failed(
        workflow_node_id: impl Into<String>,
        producer_id: impl Into<String>,
        evidence_key: impl Into<String>,
        diagnostics: Vec<UiStoryDiagnostic>,
    ) -> Self {
        Self::new(
            UiStoryWorkflowNodeId::new(workflow_node_id),
            UiStoryEvidenceProducerId::new(producer_id),
            UiStoryEvidenceKey::new(evidence_key),
            UiStoryEvidenceStatus::Failed,
            diagnostics,
        )
    }

    pub fn skipped(
        workflow_node_id: impl Into<String>,
        producer_id: impl Into<String>,
        evidence_key: impl Into<String>,
        diagnostics: Vec<UiStoryDiagnostic>,
    ) -> Self {
        Self::new(
            UiStoryWorkflowNodeId::new(workflow_node_id),
            UiStoryEvidenceProducerId::new(producer_id),
            UiStoryEvidenceKey::new(evidence_key),
            UiStoryEvidenceStatus::Skipped,
            diagnostics,
        )
    }

    pub fn blocked(
        workflow_node_id: impl Into<String>,
        producer_id: impl Into<String>,
        evidence_key: impl Into<String>,
        diagnostics: Vec<UiStoryDiagnostic>,
    ) -> Self {
        Self::new(
            UiStoryWorkflowNodeId::new(workflow_node_id),
            UiStoryEvidenceProducerId::new(producer_id),
            UiStoryEvidenceKey::new(evidence_key),
            UiStoryEvidenceStatus::Blocked,
            diagnostics,
        )
    }

    pub fn with_elapsed_micros(mut self, elapsed_micros: u64) -> Self {
        self.elapsed_micros = Some(elapsed_micros);
        self
    }

    pub fn with_diagnostic(mut self, diagnostic: UiStoryDiagnostic) -> Self {
        self.diagnostics.push(diagnostic);
        self
    }

    pub fn has_blocking_diagnostics(&self) -> bool {
        self.diagnostics.iter().any(UiStoryDiagnostic::is_blocking)
    }

    pub fn blocks_node(&self) -> bool {
        self.status.blocks_node() || self.has_blocking_diagnostics()
    }

    pub fn passed_without_blockers(&self) -> bool {
        self.status.is_passed() && !self.has_blocking_diagnostics()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostic::{
        UI_STORY_RUN_MISSING_REQUIRED_EVIDENCE, UiStoryDiagnosticOrigin, UiStoryDiagnosticSubject,
    };

    fn blocking_diagnostic() -> UiStoryDiagnostic {
        UiStoryDiagnostic::error(
            UI_STORY_RUN_MISSING_REQUIRED_EVIDENCE,
            UiStoryDiagnosticOrigin::Evidence,
            UiStoryDiagnosticSubject::WorkflowNode(UiStoryWorkflowNodeId::new("source_load")),
            "source load evidence is missing",
        )
    }

    #[test]
    fn passed_evidence_does_not_block() {
        let evidence = UiStoryEvidence::passed(
            "source_load",
            "runenwerk_editor.ui_gallery.source_loader",
            "ui.gallery.source_load",
        );

        assert!(!evidence.blocks_node());
        assert!(evidence.passed_without_blockers());
    }

    #[test]
    fn failed_evidence_blocks() {
        let evidence = UiStoryEvidence::failed(
            "source_load",
            "runenwerk_editor.ui_gallery.source_loader",
            "ui.gallery.source_load",
            vec![blocking_diagnostic()],
        );

        assert!(evidence.blocks_node());
        assert!(!evidence.passed_without_blockers());
    }

    #[test]
    fn result_evidence_chooses_status_without_hiding_diagnostics() {
        let passed = UiStoryEvidence::from_result(
            "source_load",
            "runenwerk_editor.ui_gallery.source_loader",
            "ui.gallery.source_load",
            true,
            Vec::new(),
        );
        let failed = UiStoryEvidence::from_result(
            "source_load",
            "runenwerk_editor.ui_gallery.source_loader",
            "ui.gallery.source_load",
            false,
            vec![blocking_diagnostic()],
        );

        assert_eq!(passed.status, UiStoryEvidenceStatus::Passed);
        assert_eq!(failed.status, UiStoryEvidenceStatus::Failed);
        assert!(failed.blocks_node());
    }

    #[test]
    fn blocking_diagnostic_blocks_otherwise_passed_evidence() {
        let evidence = UiStoryEvidence::passed(
            "source_load",
            "runenwerk_editor.ui_gallery.source_loader",
            "ui.gallery.source_load",
        )
        .with_diagnostic(blocking_diagnostic());

        assert!(evidence.has_blocking_diagnostics());
        assert!(evidence.blocks_node());
        assert!(!evidence.passed_without_blockers());
    }

    #[test]
    fn elapsed_micros_is_preserved() {
        let evidence = UiStoryEvidence::passed(
            "source_load",
            "runenwerk_editor.ui_gallery.source_loader",
            "ui.gallery.source_load",
        )
        .with_elapsed_micros(42);

        assert_eq!(evidence.elapsed_micros, Some(42));
    }

    #[test]
    fn evidence_records_sort_deterministically_by_node_producer_and_key() {
        let mut records = vec![
            UiStoryEvidence::passed(
                "source_parse",
                "runenwerk_editor.ui_gallery.parser",
                "ui.gallery.source_parse",
            ),
            UiStoryEvidence::passed(
                "source_load",
                "runenwerk_editor.ui_gallery.source_loader.b",
                "ui.gallery.source_load",
            ),
            UiStoryEvidence::passed(
                "source_load",
                "runenwerk_editor.ui_gallery.source_loader.a",
                "ui.gallery.source_load",
            ),
        ];

        records.sort();

        assert_eq!(
            records
                .iter()
                .map(|record| (
                    record.workflow_node_id.as_str(),
                    record.producer_id.as_str(),
                    record.evidence_key.as_str(),
                ))
                .collect::<Vec<_>>(),
            vec![
                (
                    "source_load",
                    "runenwerk_editor.ui_gallery.source_loader.a",
                    "ui.gallery.source_load",
                ),
                (
                    "source_load",
                    "runenwerk_editor.ui_gallery.source_loader.b",
                    "ui.gallery.source_load",
                ),
                (
                    "source_parse",
                    "runenwerk_editor.ui_gallery.parser",
                    "ui.gallery.source_parse",
                ),
            ]
        );
    }
}
