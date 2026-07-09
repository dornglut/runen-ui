use serde::{Deserialize, Serialize};

use crate::identity::UiStoryWorkflowNodeId;
use crate::manifest_v2::UiStoryMountPolicyV2;
use crate::report_v2::{UiStoryOutcomeV2, UiStoryWorkflowReportV2};
use crate::workflow::NODE_PREVIEW_FRAME;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum UiStoryMountBlockReasonV2 {
    Allowed,
    BlockedFailedOutcome,
    BlockedExpectedFailure,
    BlockedPolicyNever,
    BlockedPolicyGalleryOnly,
    BlockedMissingPreviewProof,
    BlockedInvalidWorkflow,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct UiStoryMountDecisionV2 {
    pub allowed: bool,
    pub reason: UiStoryMountBlockReasonV2,
}

impl UiStoryMountDecisionV2 {
    pub const fn allowed() -> Self {
        Self {
            allowed: true,
            reason: UiStoryMountBlockReasonV2::Allowed,
        }
    }

    pub const fn blocked(reason: UiStoryMountBlockReasonV2) -> Self {
        Self {
            allowed: false,
            reason,
        }
    }

    pub fn from_report(
        report: &UiStoryWorkflowReportV2,
        mount_policy: UiStoryMountPolicyV2,
    ) -> Self {
        match report.outcome() {
            UiStoryOutcomeV2::InvalidWorkflow => {
                return Self::blocked(UiStoryMountBlockReasonV2::BlockedInvalidWorkflow);
            }
            UiStoryOutcomeV2::ExpectedFailureMatched => {
                return Self::blocked(UiStoryMountBlockReasonV2::BlockedExpectedFailure);
            }
            _ => {}
        }

        match mount_policy {
            UiStoryMountPolicyV2::Never => {
                return Self::blocked(UiStoryMountBlockReasonV2::BlockedPolicyNever);
            }
            UiStoryMountPolicyV2::GalleryOnly => {
                return Self::blocked(UiStoryMountBlockReasonV2::BlockedPolicyGalleryOnly);
            }
            UiStoryMountPolicyV2::EligibleWhenPassed => {}
        }

        let preview_node_id = UiStoryWorkflowNodeId::new(NODE_PREVIEW_FRAME);
        let Some(preview_report) = report.node(&preview_node_id) else {
            return Self::blocked(UiStoryMountBlockReasonV2::BlockedMissingPreviewProof);
        };

        if !preview_report.passed_without_blockers() {
            return Self::blocked(UiStoryMountBlockReasonV2::BlockedMissingPreviewProof);
        }

        match report.outcome() {
            UiStoryOutcomeV2::Passed => Self::allowed(),
            UiStoryOutcomeV2::Failed
            | UiStoryOutcomeV2::Blocked
            | UiStoryOutcomeV2::InvalidManifest => {
                Self::blocked(UiStoryMountBlockReasonV2::BlockedFailedOutcome)
            }
            UiStoryOutcomeV2::ExpectedFailureMatched => {
                Self::blocked(UiStoryMountBlockReasonV2::BlockedExpectedFailure)
            }
            UiStoryOutcomeV2::InvalidWorkflow => {
                Self::blocked(UiStoryMountBlockReasonV2::BlockedInvalidWorkflow)
            }
        }
    }

    pub const fn is_allowed(self) -> bool {
        self.allowed
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostic::{
        UiStoryDiagnostic, UiStoryDiagnosticOrigin, UiStoryDiagnosticSeverity,
        UiStoryDiagnosticSubject,
    };
    use crate::evidence::{UiStoryDiagnosticExpectation, UiStoryEvidence};
    use crate::identity::{UiStoryEvidenceProducerId, UiStoryId, UiStoryWorkflowNodeId};
    use crate::manifest_v2::UiStoryExpectedOutcomeV2;
    use crate::report_v2::UiStoryWorkflowReportV2;
    use crate::run_v2::UiStoryWorkflowRunV2;
    use crate::workflow::{
        NODE_COMPILER, NODE_PREVIEW_FRAME, NODE_PROGRAM_FORMATION, NODE_RENDER_DATA,
        NODE_RENDER_PRIMITIVES, NODE_RUNTIME_VIEW, NODE_SOURCE_LOAD, NODE_SOURCE_PARSE,
        NODE_STATIC_MOUNT, UiStoryBuiltinWorkflowProfile,
    };

    const STORY_ID: &str = "ui.gallery.button.basic";
    const PRODUCER_ID: &str = "runenwerk_editor.ui_gallery.test_producer";

    fn passed_evidence(node_id: &str) -> UiStoryEvidence {
        UiStoryEvidence::passed(node_id, PRODUCER_ID, format!("ui.gallery.{node_id}"))
    }

    fn passed_static_preview_report() -> UiStoryWorkflowReportV2 {
        let mut run = UiStoryWorkflowRunV2::new(
            UiStoryId::new(STORY_ID),
            UiStoryBuiltinWorkflowProfile::StaticPreview.graph(),
        );
        run.record_many([
            passed_evidence(NODE_SOURCE_LOAD),
            passed_evidence(NODE_SOURCE_PARSE),
            passed_evidence(NODE_PROGRAM_FORMATION),
            passed_evidence(NODE_COMPILER),
            passed_evidence(NODE_RUNTIME_VIEW),
            passed_evidence(NODE_RENDER_PRIMITIVES),
            passed_evidence(NODE_RENDER_DATA),
            passed_evidence(NODE_STATIC_MOUNT),
            passed_evidence(NODE_PREVIEW_FRAME),
        ]);
        run.finish().into_report(UiStoryExpectedOutcomeV2::Pass)
    }

    fn expected_failure_report() -> UiStoryWorkflowReportV2 {
        let mut run = UiStoryWorkflowRunV2::new(
            UiStoryId::new(STORY_ID),
            UiStoryBuiltinWorkflowProfile::SourceLoadOnly.graph(),
        );
        let diagnostic = UiStoryDiagnostic::error(
            "ui_gallery.story.source.read_failed",
            UiStoryDiagnosticOrigin::ExternalProducer(UiStoryEvidenceProducerId::new(PRODUCER_ID)),
            UiStoryDiagnosticSubject::WorkflowNode(UiStoryWorkflowNodeId::new(NODE_SOURCE_LOAD)),
            "source load failed",
        );
        run.record(UiStoryEvidence::failed(
            NODE_SOURCE_LOAD,
            PRODUCER_ID,
            "ui.gallery.source_load",
            vec![diagnostic],
        ));
        run.finish()
            .into_report(UiStoryExpectedOutcomeV2::expected_failure(
                UiStoryDiagnosticExpectation::from_strings(
                    NODE_SOURCE_LOAD,
                    PRODUCER_ID,
                    "ui.gallery.source_load",
                    "ui_gallery.story.source.read_failed",
                    UiStoryDiagnosticSeverity::Error,
                ),
            ))
    }

    fn static_preview_missing_preview_report() -> UiStoryWorkflowReportV2 {
        let mut run = UiStoryWorkflowRunV2::new(
            UiStoryId::new(STORY_ID),
            UiStoryBuiltinWorkflowProfile::StaticPreview.graph(),
        );
        run.record_many([
            passed_evidence(NODE_SOURCE_LOAD),
            passed_evidence(NODE_SOURCE_PARSE),
            passed_evidence(NODE_PROGRAM_FORMATION),
            passed_evidence(NODE_COMPILER),
            passed_evidence(NODE_RUNTIME_VIEW),
            passed_evidence(NODE_RENDER_PRIMITIVES),
            passed_evidence(NODE_RENDER_DATA),
            passed_evidence(NODE_STATIC_MOUNT),
        ]);
        run.finish().into_report(UiStoryExpectedOutcomeV2::Pass)
    }

    #[test]
    fn mount_v2_allows_passed_static_preview_with_preview_frame() {
        let decision = UiStoryMountDecisionV2::from_report(
            &passed_static_preview_report(),
            UiStoryMountPolicyV2::EligibleWhenPassed,
        );

        assert!(decision.is_allowed());
        assert_eq!(decision.reason, UiStoryMountBlockReasonV2::Allowed);
    }

    #[test]
    fn mount_v2_blocks_expected_failure_matched() {
        let decision = UiStoryMountDecisionV2::from_report(
            &expected_failure_report(),
            UiStoryMountPolicyV2::EligibleWhenPassed,
        );

        assert!(!decision.is_allowed());
        assert_eq!(
            decision.reason,
            UiStoryMountBlockReasonV2::BlockedExpectedFailure
        );
    }

    #[test]
    fn mount_v2_blocks_gallery_only_policy() {
        let decision = UiStoryMountDecisionV2::from_report(
            &passed_static_preview_report(),
            UiStoryMountPolicyV2::GalleryOnly,
        );

        assert!(!decision.is_allowed());
        assert_eq!(
            decision.reason,
            UiStoryMountBlockReasonV2::BlockedPolicyGalleryOnly
        );
    }

    #[test]
    fn mount_v2_blocks_missing_preview_proof() {
        let decision = UiStoryMountDecisionV2::from_report(
            &static_preview_missing_preview_report(),
            UiStoryMountPolicyV2::EligibleWhenPassed,
        );

        assert!(!decision.is_allowed());
        assert_eq!(
            decision.reason,
            UiStoryMountBlockReasonV2::BlockedMissingPreviewProof
        );
    }
}
