//! Workflow report model for UI Story V2.
//!
//! Report V2 consumes the interim workflow run result and produces the semantic
//! inspection object used by mount decisions, CLI summaries, and later app
//! integration. It does not depend on the old flat stage report model.

mod node_report;
mod outcome;
mod summary;

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::diagnostic::UiStoryDiagnostic;
use crate::evidence::UiStoryDiagnosticExpectation;
use crate::identity::{UiStoryId, UiStoryWorkflowNodeId};
use crate::manifest_v2::UiStoryExpectedOutcomeV2;
use crate::run_v2::UiStoryWorkflowRunResultV2;
use crate::workflow::{UiStoryWorkflowGraph, UiStoryWorkflowNode};

pub use node_report::{UiStoryWorkflowNodeReportV2, diagnostic_belongs_to_node};
pub use outcome::UiStoryOutcomeV2;
pub use summary::UiStoryWorkflowReportSummaryV2;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiStoryWorkflowReportV2 {
    pub story_id: UiStoryId,
    pub workflow_graph: Option<UiStoryWorkflowGraph>,
    #[serde(default)]
    pub node_reports: Vec<UiStoryWorkflowNodeReportV2>,
    #[serde(default)]
    pub diagnostics: Vec<UiStoryDiagnostic>,
    pub outcome: UiStoryOutcomeV2,
}

impl UiStoryWorkflowReportV2 {
    pub fn from_run_result(
        result: UiStoryWorkflowRunResultV2,
        expected_outcome: UiStoryExpectedOutcomeV2,
    ) -> Self {
        let diagnostics = collect_report_diagnostics(&result);
        let node_reports = build_node_reports(&result);
        let outcome = derive_outcome(&result, &diagnostics, &expected_outcome);

        Self {
            story_id: result.story_id,
            workflow_graph: result.workflow_graph,
            node_reports,
            diagnostics,
            outcome,
        }
    }

    pub const fn outcome(&self) -> UiStoryOutcomeV2 {
        self.outcome
    }

    pub fn node(&self, node_id: &UiStoryWorkflowNodeId) -> Option<&UiStoryWorkflowNodeReportV2> {
        self.node_reports
            .iter()
            .find(|node_report| &node_report.node_id == node_id)
    }

    pub fn diagnostics(&self) -> &[UiStoryDiagnostic] {
        &self.diagnostics
    }

    pub fn first_blocker(&self) -> Option<&UiStoryDiagnostic> {
        self.diagnostics
            .iter()
            .find(|diagnostic| diagnostic.is_blocking())
    }

    pub fn has_blockers(&self) -> bool {
        self.outcome.is_failure()
            || self
                .node_reports
                .iter()
                .any(UiStoryWorkflowNodeReportV2::has_blockers)
            || self.diagnostics.iter().any(UiStoryDiagnostic::is_blocking)
    }

    pub fn summary(&self) -> UiStoryWorkflowReportSummaryV2 {
        UiStoryWorkflowReportSummaryV2::new(
            self.story_id.clone(),
            self.outcome,
            &self.diagnostics,
            self.node_reports.len(),
            self.node_reports
                .iter()
                .filter(|node_report| node_report.has_blockers())
                .count(),
        )
    }
}

fn collect_report_diagnostics(result: &UiStoryWorkflowRunResultV2) -> Vec<UiStoryDiagnostic> {
    let mut diagnostics = result.diagnostics.clone();
    for evidence in &result.evidence {
        diagnostics.extend(evidence.diagnostics.iter().cloned());
    }
    diagnostics.sort();
    diagnostics.dedup();
    diagnostics
}

fn build_node_reports(result: &UiStoryWorkflowRunResultV2) -> Vec<UiStoryWorkflowNodeReportV2> {
    let Some(graph) = &result.workflow_graph else {
        return Vec::new();
    };

    let mut evidence_by_node = BTreeMap::<UiStoryWorkflowNodeId, Vec<_>>::new();
    for evidence in &result.evidence {
        evidence_by_node
            .entry(evidence.workflow_node_id.clone())
            .or_default()
            .push(evidence.clone());
    }

    let missing_required_nodes = result
        .missing_required_nodes
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();
    let blocked_nodes = result
        .blocked_nodes
        .iter()
        .cloned()
        .collect::<BTreeSet<_>>();

    ordered_nodes(graph)
        .into_iter()
        .map(|node| {
            let evidence = evidence_by_node.remove(&node.node_id).unwrap_or_default();
            let mut diagnostics = result
                .diagnostics
                .iter()
                .filter(|diagnostic| diagnostic_belongs_to_node(diagnostic, &node.node_id))
                .cloned()
                .collect::<Vec<_>>();
            for evidence_record in &evidence {
                diagnostics.extend(evidence_record.diagnostics.iter().cloned());
            }
            diagnostics.sort();
            diagnostics.dedup();

            UiStoryWorkflowNodeReportV2::new(
                node.node_id.clone(),
                node.policy,
                evidence,
                diagnostics,
                missing_required_nodes.contains(&node.node_id),
                blocked_nodes.contains(&node.node_id),
            )
        })
        .collect()
}

fn ordered_nodes(graph: &UiStoryWorkflowGraph) -> Vec<UiStoryWorkflowNode> {
    match graph.topological_nodes() {
        Ok(nodes) => nodes.into_iter().cloned().collect(),
        Err(_) => graph.nodes().to_vec(),
    }
}

fn derive_outcome(
    result: &UiStoryWorkflowRunResultV2,
    diagnostics: &[UiStoryDiagnostic],
    expected_outcome: &UiStoryExpectedOutcomeV2,
) -> UiStoryOutcomeV2 {
    if result.workflow_graph.is_none() && diagnostics.iter().any(UiStoryDiagnostic::is_blocking) {
        return UiStoryOutcomeV2::InvalidWorkflow;
    }

    let has_blockers = !result.duplicate_evidence_keys.is_empty()
        || !result.missing_required_nodes.is_empty()
        || !result.blocked_nodes.is_empty()
        || diagnostics.iter().any(UiStoryDiagnostic::is_blocking)
        || result
            .evidence
            .iter()
            .any(|evidence| evidence.blocks_node());

    if expected_failure_matches(expected_outcome, result) {
        return UiStoryOutcomeV2::ExpectedFailureMatched;
    }

    if has_blockers {
        UiStoryOutcomeV2::Failed
    } else {
        UiStoryOutcomeV2::Passed
    }
}

fn expected_failure_matches(
    expected_outcome: &UiStoryExpectedOutcomeV2,
    result: &UiStoryWorkflowRunResultV2,
) -> bool {
    let UiStoryExpectedOutcomeV2::ExpectedFailure { expectation } = expected_outcome else {
        return false;
    };

    recorded_diagnostic_matches(expectation, result)
}

fn recorded_diagnostic_matches(
    expectation: &UiStoryDiagnosticExpectation,
    result: &UiStoryWorkflowRunResultV2,
) -> bool {
    result.evidence.iter().any(|evidence| {
        evidence
            .diagnostics
            .iter()
            .any(|diagnostic| expectation.matches(evidence, diagnostic))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostic::{
        UI_STORY_RUN_MISSING_REQUIRED_EVIDENCE, UI_STORY_RUN_UNKNOWN_STORY, UiStoryDiagnostic,
        UiStoryDiagnosticOrigin, UiStoryDiagnosticSeverity, UiStoryDiagnosticSubject,
    };
    use crate::evidence::UiStoryEvidence;
    use crate::identity::{UiStoryEvidenceProducerId, UiStoryId, UiStoryWorkflowNodeId};
    use crate::run_v2::UiStoryWorkflowRunV2;
    use crate::workflow::{
        NODE_COMPILER, NODE_PREVIEW_FRAME, NODE_PROGRAM_FORMATION, NODE_RENDER_DATA,
        NODE_RENDER_PRIMITIVES, NODE_RUNTIME_VIEW, NODE_SOURCE_LOAD, NODE_SOURCE_PARSE,
        NODE_STATIC_MOUNT, UiStoryBuiltinWorkflowProfile,
    };

    const STORY_ID: &str = "ui.gallery.button.basic";
    const PRODUCER_ID: &str = "runenwerk_editor.ui_gallery.source_loader";
    const EVIDENCE_KEY: &str = "ui.gallery.source_load";

    fn source_load_only_run() -> UiStoryWorkflowRunV2 {
        UiStoryWorkflowRunV2::new(
            UiStoryId::new(STORY_ID),
            UiStoryBuiltinWorkflowProfile::SourceLoadOnly.graph(),
        )
    }

    fn static_preview_run() -> UiStoryWorkflowRunV2 {
        UiStoryWorkflowRunV2::new(
            UiStoryId::new(STORY_ID),
            UiStoryBuiltinWorkflowProfile::StaticPreview.graph(),
        )
    }

    fn passed_evidence(node_id: &str) -> UiStoryEvidence {
        UiStoryEvidence::passed(
            node_id,
            "runenwerk_editor.ui_gallery.test_producer",
            format!("ui.gallery.{node_id}"),
        )
    }

    fn expected_source_load_diagnostic() -> UiStoryDiagnostic {
        UiStoryDiagnostic::error(
            "ui_gallery.story.source.read_failed",
            UiStoryDiagnosticOrigin::ExternalProducer(UiStoryEvidenceProducerId::new(PRODUCER_ID)),
            UiStoryDiagnosticSubject::WorkflowNode(UiStoryWorkflowNodeId::new(NODE_SOURCE_LOAD)),
            "source load failed",
        )
    }

    fn failed_source_load_evidence() -> UiStoryEvidence {
        UiStoryEvidence::failed(
            NODE_SOURCE_LOAD,
            PRODUCER_ID,
            EVIDENCE_KEY,
            vec![expected_source_load_diagnostic()],
        )
    }

    fn expected_source_load_failure() -> UiStoryExpectedOutcomeV2 {
        UiStoryExpectedOutcomeV2::expected_failure(UiStoryDiagnosticExpectation::from_strings(
            NODE_SOURCE_LOAD,
            PRODUCER_ID,
            EVIDENCE_KEY,
            "ui_gallery.story.source.read_failed",
            UiStoryDiagnosticSeverity::Error,
        ))
    }

    fn passed_static_preview_result() -> UiStoryWorkflowRunResultV2 {
        let mut run = static_preview_run();
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
        run.finish()
    }

    #[test]
    fn report_v2_passes_clean_source_load_only_run() {
        let mut run = source_load_only_run();
        run.record(UiStoryEvidence::passed(
            NODE_SOURCE_LOAD,
            PRODUCER_ID,
            EVIDENCE_KEY,
        ));

        let report =
            UiStoryWorkflowReportV2::from_run_result(run.finish(), UiStoryExpectedOutcomeV2::Pass);

        assert_eq!(report.outcome(), UiStoryOutcomeV2::Passed);
        assert!(!report.has_blockers());
        assert_eq!(report.node_reports.len(), 2);
        assert!(
            report
                .node(&UiStoryWorkflowNodeId::new(NODE_SOURCE_LOAD))
                .is_some()
        );
    }

    #[test]
    fn report_v2_fails_missing_required_evidence() {
        let report = UiStoryWorkflowReportV2::from_run_result(
            source_load_only_run().finish(),
            UiStoryExpectedOutcomeV2::Pass,
        );

        assert_eq!(report.outcome(), UiStoryOutcomeV2::Failed);
        assert!(report.has_blockers());
        assert!(
            report.diagnostics().iter().any(
                |diagnostic| diagnostic.code.as_str() == UI_STORY_RUN_MISSING_REQUIRED_EVIDENCE
            )
        );
    }

    #[test]
    fn report_v2_fails_duplicate_evidence() {
        let mut run = source_load_only_run();
        run.record(UiStoryEvidence::passed(
            NODE_SOURCE_LOAD,
            PRODUCER_ID,
            EVIDENCE_KEY,
        ));
        run.record(UiStoryEvidence::passed(
            NODE_SOURCE_LOAD,
            PRODUCER_ID,
            EVIDENCE_KEY,
        ));

        let report =
            UiStoryWorkflowReportV2::from_run_result(run.finish(), UiStoryExpectedOutcomeV2::Pass);

        assert_eq!(report.outcome(), UiStoryOutcomeV2::Failed);
        assert!(report.first_blocker().is_some());
    }

    #[test]
    fn report_v2_marks_expected_failure_matched() {
        let mut run = source_load_only_run();
        run.record(failed_source_load_evidence());

        let report =
            UiStoryWorkflowReportV2::from_run_result(run.finish(), expected_source_load_failure());

        assert_eq!(report.outcome(), UiStoryOutcomeV2::ExpectedFailureMatched);
        assert!(report.has_blockers());
    }

    #[test]
    fn report_v2_rejects_wrong_expected_failure_diagnostic() {
        let mut run = source_load_only_run();
        run.record(failed_source_load_evidence());
        let wrong_expectation =
            UiStoryExpectedOutcomeV2::expected_failure(UiStoryDiagnosticExpectation::from_strings(
                NODE_SOURCE_LOAD,
                PRODUCER_ID,
                EVIDENCE_KEY,
                "ui_gallery.story.other_error",
                UiStoryDiagnosticSeverity::Error,
            ));

        let report = UiStoryWorkflowReportV2::from_run_result(run.finish(), wrong_expectation);

        assert_eq!(report.outcome(), UiStoryOutcomeV2::Failed);
    }

    #[test]
    fn report_v2_invalid_workflow_for_unknown_profile_seed() {
        let result = UiStoryWorkflowRunResultV2::failed_seed(
            UiStoryId::new(STORY_ID),
            None,
            UiStoryDiagnostic::error(
                UI_STORY_RUN_UNKNOWN_STORY,
                UiStoryDiagnosticOrigin::Runner,
                UiStoryDiagnosticSubject::Story(UiStoryId::new(STORY_ID)),
                "unknown story",
            ),
        );

        let report =
            UiStoryWorkflowReportV2::from_run_result(result, UiStoryExpectedOutcomeV2::Pass);

        assert_eq!(report.outcome(), UiStoryOutcomeV2::InvalidWorkflow);
        assert!(report.workflow_graph.is_none());
    }

    #[test]
    fn run_result_v2_into_report_uses_report_v2() {
        let mut run = source_load_only_run();
        run.record(UiStoryEvidence::passed(
            NODE_SOURCE_LOAD,
            PRODUCER_ID,
            EVIDENCE_KEY,
        ));

        let report = run.finish().into_report(UiStoryExpectedOutcomeV2::Pass);

        assert_eq!(report.outcome(), UiStoryOutcomeV2::Passed);
    }

    #[test]
    fn report_v2_summary_counts_blocked_nodes() {
        let report = UiStoryWorkflowReportV2::from_run_result(
            source_load_only_run().finish(),
            UiStoryExpectedOutcomeV2::Pass,
        );
        let summary = report.summary();

        assert_eq!(summary.story_id.as_str(), STORY_ID);
        assert_eq!(summary.outcome, UiStoryOutcomeV2::Failed);
        assert_eq!(summary.node_count, 2);
        assert!(summary.blocked_node_count > 0);
    }

    pub(crate) fn passed_static_preview_report() -> UiStoryWorkflowReportV2 {
        UiStoryWorkflowReportV2::from_run_result(
            passed_static_preview_result(),
            UiStoryExpectedOutcomeV2::Pass,
        )
    }
}
