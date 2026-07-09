//! CLI-friendly summaries for UI Story V2 workflow reports.
//!
//! CLI V2 is a pure formatter over Report V2 and Mount V2. It does not execute
//! filesystem discovery, compilers, renderers, static mount, app/editor behavior,
//! or the old flat-stage story report model.

use serde::{Deserialize, Serialize};

use crate::identity::UiStoryId;
use crate::manifest_v2::UiStoryMountPolicyV2;
use crate::mount_v2::{UiStoryMountBlockReasonV2, UiStoryMountDecisionV2};
use crate::report_v2::{UiStoryOutcomeV2, UiStoryWorkflowReportV2};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiStoryCliReportV2 {
    pub stories: Vec<UiStoryCliStorySummaryV2>,
    pub passed: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiStoryCliStorySummaryV2 {
    pub story_id: UiStoryId,
    pub outcome: UiStoryOutcomeV2,
    pub mount_allowed: bool,
    pub mount_reason: UiStoryMountBlockReasonV2,
    pub diagnostic_count: usize,
    pub blocking_diagnostic_count: usize,
    pub node_count: usize,
    pub blocked_node_count: usize,
}

impl UiStoryCliReportV2 {
    pub fn from_reports<'a>(
        reports: impl IntoIterator<Item = (&'a UiStoryWorkflowReportV2, UiStoryMountPolicyV2)>,
    ) -> Self {
        let mut stories = reports
            .into_iter()
            .map(|(report, mount_policy)| {
                UiStoryCliStorySummaryV2::from_report(report, mount_policy)
            })
            .collect::<Vec<_>>();
        stories.sort_by(|left, right| left.story_id.cmp(&right.story_id));
        let passed = stories.iter().all(UiStoryCliStorySummaryV2::is_proof_green);

        Self { stories, passed }
    }

    pub const fn passed(&self) -> bool {
        self.passed
    }

    pub fn render_text(&self) -> String {
        let total_count = self.stories.len();
        let passed_count = self
            .stories
            .iter()
            .filter(|story| story.is_proof_green())
            .count();
        let failed_count = total_count.saturating_sub(passed_count);
        let status = if self.passed { "PASSED" } else { "FAILED" };

        let mut output = String::new();
        output.push_str(&format!(
            "UI Story V2 report: {status} ({passed_count}/{total_count} passed, {failed_count} failed)\n"
        ));

        for story in &self.stories {
            output.push_str(&format!(
                "- {}: outcome={:?}, mount={:?}, mount_allowed={}, diagnostics={} blocking={}, nodes={} blocked={}\n",
                story.story_id.as_str(),
                story.outcome,
                story.mount_reason,
                story.mount_allowed,
                story.diagnostic_count,
                story.blocking_diagnostic_count,
                story.node_count,
                story.blocked_node_count,
            ));
        }

        output
    }
}

impl UiStoryCliStorySummaryV2 {
    pub fn from_report(
        report: &UiStoryWorkflowReportV2,
        mount_policy: UiStoryMountPolicyV2,
    ) -> Self {
        let mount_decision = UiStoryMountDecisionV2::from_report(report, mount_policy);
        let report_summary = report.summary();

        Self {
            story_id: report_summary.story_id,
            outcome: report_summary.outcome,
            mount_allowed: mount_decision.allowed,
            mount_reason: mount_decision.reason,
            diagnostic_count: report_summary.diagnostic_count,
            blocking_diagnostic_count: report_summary.blocking_diagnostic_count,
            node_count: report_summary.node_count,
            blocked_node_count: report_summary.blocked_node_count,
        }
    }

    fn is_proof_green(&self) -> bool {
        match self.outcome {
            UiStoryOutcomeV2::Passed => {
                self.blocking_diagnostic_count == 0 && self.blocked_node_count == 0
            }
            UiStoryOutcomeV2::ExpectedFailureMatched => true,
            UiStoryOutcomeV2::Failed
            | UiStoryOutcomeV2::Blocked
            | UiStoryOutcomeV2::InvalidManifest
            | UiStoryOutcomeV2::InvalidWorkflow => false,
        }
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
    use crate::identity::{UiStoryEvidenceProducerId, UiStoryWorkflowNodeId};
    use crate::manifest_v2::UiStoryExpectedOutcomeV2;
    use crate::run_v2::UiStoryWorkflowRunV2;
    use crate::workflow::{
        NODE_COMPILER, NODE_PREVIEW_FRAME, NODE_PROGRAM_FORMATION, NODE_RENDER_DATA,
        NODE_RENDER_PRIMITIVES, NODE_RUNTIME_VIEW, NODE_SOURCE_LOAD, NODE_SOURCE_PARSE,
        NODE_STATIC_MOUNT, UiStoryBuiltinWorkflowProfile,
    };

    const PRODUCER_ID: &str = "runenwerk_editor.ui_gallery.test_producer";

    fn passed_evidence(node_id: &str) -> UiStoryEvidence {
        UiStoryEvidence::passed(node_id, PRODUCER_ID, format!("ui.gallery.{node_id}"))
    }

    fn passed_static_preview_report(story_id: &str) -> UiStoryWorkflowReportV2 {
        let mut run = UiStoryWorkflowRunV2::new(
            UiStoryId::new(story_id),
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

    fn failed_source_load_diagnostic() -> UiStoryDiagnostic {
        UiStoryDiagnostic::error(
            "ui_gallery.story.source.read_failed",
            UiStoryDiagnosticOrigin::ExternalProducer(UiStoryEvidenceProducerId::new(PRODUCER_ID)),
            UiStoryDiagnosticSubject::WorkflowNode(UiStoryWorkflowNodeId::new(NODE_SOURCE_LOAD)),
            "source load failed",
        )
    }

    fn expected_failure_report(story_id: &str) -> UiStoryWorkflowReportV2 {
        let mut run = UiStoryWorkflowRunV2::new(
            UiStoryId::new(story_id),
            UiStoryBuiltinWorkflowProfile::SourceLoadOnly.graph(),
        );
        run.record(UiStoryEvidence::failed(
            NODE_SOURCE_LOAD,
            PRODUCER_ID,
            "ui.gallery.source_load",
            vec![failed_source_load_diagnostic()],
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

    fn failed_report(story_id: &str) -> UiStoryWorkflowReportV2 {
        UiStoryWorkflowRunV2::new(
            UiStoryId::new(story_id),
            UiStoryBuiltinWorkflowProfile::SourceLoadOnly.graph(),
        )
        .finish()
        .into_report(UiStoryExpectedOutcomeV2::Pass)
    }

    #[test]
    fn cli_v2_reports_passed_story() {
        let report = passed_static_preview_report("ui.gallery.button.basic");
        let cli_report =
            UiStoryCliReportV2::from_reports([(&report, UiStoryMountPolicyV2::EligibleWhenPassed)]);

        assert!(cli_report.passed());
        assert_eq!(cli_report.stories.len(), 1);
        assert_eq!(cli_report.stories[0].outcome, UiStoryOutcomeV2::Passed);
        assert!(cli_report.stories[0].mount_allowed);
        assert_eq!(
            cli_report.stories[0].mount_reason,
            UiStoryMountBlockReasonV2::Allowed
        );
    }

    #[test]
    fn cli_v2_reports_expected_failure_as_green_but_not_mountable() {
        let report = expected_failure_report("ui.gallery.button.missing_source");
        let cli_report =
            UiStoryCliReportV2::from_reports([(&report, UiStoryMountPolicyV2::EligibleWhenPassed)]);

        assert!(cli_report.passed());
        assert_eq!(
            cli_report.stories[0].outcome,
            UiStoryOutcomeV2::ExpectedFailureMatched
        );
        assert!(!cli_report.stories[0].mount_allowed);
        assert_eq!(
            cli_report.stories[0].mount_reason,
            UiStoryMountBlockReasonV2::BlockedExpectedFailure
        );
    }

    #[test]
    fn cli_v2_reports_failed_story() {
        let report = failed_report("ui.gallery.button.failed");
        let cli_report =
            UiStoryCliReportV2::from_reports([(&report, UiStoryMountPolicyV2::EligibleWhenPassed)]);

        assert!(!cli_report.passed());
        assert_eq!(cli_report.stories[0].outcome, UiStoryOutcomeV2::Failed);
        assert!(cli_report.stories[0].blocking_diagnostic_count > 0);
        assert!(cli_report.stories[0].blocked_node_count > 0);
    }

    #[test]
    fn cli_v2_render_text_includes_outcome_and_mount_reason() {
        let report = passed_static_preview_report("ui.gallery.button.basic");
        let cli_report =
            UiStoryCliReportV2::from_reports([(&report, UiStoryMountPolicyV2::EligibleWhenPassed)]);
        let rendered = cli_report.render_text();

        assert!(rendered.contains("UI Story V2 report: PASSED"));
        assert!(rendered.contains("ui.gallery.button.basic"));
        assert!(rendered.contains("outcome=Passed"));
        assert!(rendered.contains("mount=Allowed"));
    }

    #[test]
    fn cli_v2_orders_stories_by_story_id() {
        let z_report = passed_static_preview_report("ui.gallery.zeta");
        let a_report = expected_failure_report("ui.gallery.alpha");
        let m_report = failed_report("ui.gallery.middle");
        let cli_report = UiStoryCliReportV2::from_reports([
            (&z_report, UiStoryMountPolicyV2::EligibleWhenPassed),
            (&a_report, UiStoryMountPolicyV2::EligibleWhenPassed),
            (&m_report, UiStoryMountPolicyV2::EligibleWhenPassed),
        ]);

        let ordered_ids = cli_report
            .stories
            .iter()
            .map(|story| story.story_id.as_str())
            .collect::<Vec<_>>();

        assert_eq!(
            ordered_ids,
            vec!["ui.gallery.alpha", "ui.gallery.middle", "ui.gallery.zeta"]
        );
    }

    #[test]
    fn cli_v2_does_not_use_old_stage_report_types() {
        let report = passed_static_preview_report("ui.gallery.button.basic");
        let cli_report =
            UiStoryCliReportV2::from_reports([(&report, UiStoryMountPolicyV2::GalleryOnly)]);
        let rendered = cli_report.render_text();

        assert!(cli_report.passed());
        assert!(rendered.contains("mount=BlockedPolicyGalleryOnly"));
        assert!(!rendered.contains("UiStoryStageReport"));
        assert!(!rendered.contains("UiStoryRunReport"));
        assert!(!rendered.contains("UiStoryStageKind"));
    }
}
