use serde::{Deserialize, Serialize};

use crate::diagnostic::{UiStoryDiagnostic, UiStoryDiagnosticCode, UiStoryDiagnosticSeverity};
use crate::identity::{UiStoryEvidenceKey, UiStoryEvidenceProducerId, UiStoryWorkflowNodeId};

use super::record::UiStoryEvidence;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct UiStoryDiagnosticExpectation {
    pub workflow_node_id: UiStoryWorkflowNodeId,
    pub producer_id: UiStoryEvidenceProducerId,
    pub evidence_key: UiStoryEvidenceKey,
    pub code: UiStoryDiagnosticCode,
    pub severity: UiStoryDiagnosticSeverity,
}

impl UiStoryDiagnosticExpectation {
    pub fn new(
        workflow_node_id: UiStoryWorkflowNodeId,
        producer_id: UiStoryEvidenceProducerId,
        evidence_key: UiStoryEvidenceKey,
        code: UiStoryDiagnosticCode,
        severity: UiStoryDiagnosticSeverity,
    ) -> Self {
        Self {
            workflow_node_id,
            producer_id,
            evidence_key,
            code,
            severity,
        }
    }

    pub fn from_strings(
        workflow_node_id: impl Into<String>,
        producer_id: impl Into<String>,
        evidence_key: impl Into<String>,
        code: impl Into<String>,
        severity: UiStoryDiagnosticSeverity,
    ) -> Self {
        Self::new(
            UiStoryWorkflowNodeId::new(workflow_node_id),
            UiStoryEvidenceProducerId::new(producer_id),
            UiStoryEvidenceKey::new(evidence_key),
            UiStoryDiagnosticCode::new(code),
            severity,
        )
    }

    pub fn error_from_strings(
        workflow_node_id: impl Into<String>,
        producer_id: impl Into<String>,
        evidence_key: impl Into<String>,
        code: impl Into<String>,
    ) -> Self {
        Self::from_strings(
            workflow_node_id,
            producer_id,
            evidence_key,
            code,
            UiStoryDiagnosticSeverity::Error,
        )
    }

    pub fn matches(&self, evidence: &UiStoryEvidence, diagnostic: &UiStoryDiagnostic) -> bool {
        self.workflow_node_id == evidence.workflow_node_id
            && self.producer_id == evidence.producer_id
            && self.evidence_key == evidence.evidence_key
            && self.code == diagnostic.code
            && self.severity == diagnostic.severity
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostic::{
        UI_STORY_RUN_MISSING_REQUIRED_EVIDENCE, UiStoryDiagnosticOrigin, UiStoryDiagnosticSubject,
    };

    fn source_load_evidence() -> UiStoryEvidence {
        UiStoryEvidence::failed(
            "source_load",
            "runenwerk_editor.ui_gallery.source_loader",
            "ui.gallery.source_load",
            vec![source_load_diagnostic()],
        )
    }

    fn source_load_diagnostic() -> UiStoryDiagnostic {
        UiStoryDiagnostic::error(
            UI_STORY_RUN_MISSING_REQUIRED_EVIDENCE,
            UiStoryDiagnosticOrigin::ExternalProducer(UiStoryEvidenceProducerId::new(
                "runenwerk_editor.ui_gallery.source_loader",
            )),
            UiStoryDiagnosticSubject::WorkflowNode(UiStoryWorkflowNodeId::new("source_load")),
            "source load failed",
        )
    }

    fn matching_expectation() -> UiStoryDiagnosticExpectation {
        UiStoryDiagnosticExpectation::from_strings(
            "source_load",
            "runenwerk_editor.ui_gallery.source_loader",
            "ui.gallery.source_load",
            UI_STORY_RUN_MISSING_REQUIRED_EVIDENCE,
            UiStoryDiagnosticSeverity::Error,
        )
    }

    #[test]
    fn diagnostic_expectation_matches_exact_node_producer_key_code_and_severity() {
        let evidence = source_load_evidence();
        let diagnostic = source_load_diagnostic();
        let expectation = matching_expectation();

        assert!(expectation.matches(&evidence, &diagnostic));
    }

    #[test]
    fn error_expectation_helper_keeps_common_expected_failure_concise() {
        let expectation = UiStoryDiagnosticExpectation::error_from_strings(
            "source_load",
            "runenwerk_editor.ui_gallery.source_loader",
            "ui.gallery.source_load",
            UI_STORY_RUN_MISSING_REQUIRED_EVIDENCE,
        );

        assert_eq!(expectation, matching_expectation());
    }

    #[test]
    fn diagnostic_expectation_rejects_wrong_producer() {
        let evidence = source_load_evidence();
        let diagnostic = source_load_diagnostic();
        let expectation = UiStoryDiagnosticExpectation::from_strings(
            "source_load",
            "runenwerk_editor.ui_gallery.other_loader",
            "ui.gallery.source_load",
            UI_STORY_RUN_MISSING_REQUIRED_EVIDENCE,
            UiStoryDiagnosticSeverity::Error,
        );

        assert!(!expectation.matches(&evidence, &diagnostic));
    }

    #[test]
    fn diagnostic_expectation_rejects_wrong_key() {
        let evidence = source_load_evidence();
        let diagnostic = source_load_diagnostic();
        let expectation = UiStoryDiagnosticExpectation::from_strings(
            "source_load",
            "runenwerk_editor.ui_gallery.source_loader",
            "ui.gallery.other_source_load",
            UI_STORY_RUN_MISSING_REQUIRED_EVIDENCE,
            UiStoryDiagnosticSeverity::Error,
        );

        assert!(!expectation.matches(&evidence, &diagnostic));
    }

    #[test]
    fn diagnostic_expectation_rejects_wrong_code() {
        let evidence = source_load_evidence();
        let diagnostic = source_load_diagnostic();
        let expectation = UiStoryDiagnosticExpectation::from_strings(
            "source_load",
            "runenwerk_editor.ui_gallery.source_loader",
            "ui.gallery.source_load",
            "ui.story.run.other_error",
            UiStoryDiagnosticSeverity::Error,
        );

        assert!(!expectation.matches(&evidence, &diagnostic));
    }

    #[test]
    fn diagnostic_expectation_rejects_wrong_severity() {
        let evidence = source_load_evidence();
        let diagnostic = source_load_diagnostic();
        let expectation = UiStoryDiagnosticExpectation::from_strings(
            "source_load",
            "runenwerk_editor.ui_gallery.source_loader",
            "ui.gallery.source_load",
            UI_STORY_RUN_MISSING_REQUIRED_EVIDENCE,
            UiStoryDiagnosticSeverity::Warning,
        );

        assert!(!expectation.matches(&evidence, &diagnostic));
    }
}
