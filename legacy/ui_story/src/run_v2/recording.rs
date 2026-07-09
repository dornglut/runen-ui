use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::diagnostic::{
    UI_STORY_RUN_BLOCKED_DEPENDENCY, UI_STORY_RUN_DUPLICATE_EVIDENCE,
    UI_STORY_RUN_MISSING_REQUIRED_EVIDENCE, UI_STORY_WORKFLOW_NODE_MISSING, UiStoryDiagnostic,
    UiStoryDiagnosticOrigin, UiStoryDiagnosticSubject,
};
use crate::evidence::UiStoryEvidence;
use crate::identity::{
    UiStoryEvidenceKey, UiStoryEvidenceProducerId, UiStoryId, UiStoryWorkflowNodeId,
};
use crate::manifest_v2::UiStoryExpectedOutcomeV2;
use crate::report_v2::UiStoryWorkflowReportV2;
use crate::workflow::{UiStoryWorkflowDependency, UiStoryWorkflowGraph, UiStoryWorkflowNodePolicy};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct UiStoryWorkflowRunV2 {
    pub story_id: UiStoryId,
    pub workflow_graph: UiStoryWorkflowGraph,
    #[serde(default)]
    pub recorded_evidence: Vec<UiStoryEvidence>,
    #[serde(default)]
    pub diagnostics: Vec<UiStoryDiagnostic>,
}

impl UiStoryWorkflowRunV2 {
    pub(crate) fn new(story_id: UiStoryId, workflow_graph: UiStoryWorkflowGraph) -> Self {
        Self {
            story_id,
            workflow_graph,
            recorded_evidence: Vec::new(),
            diagnostics: Vec::new(),
        }
    }

    pub fn record(&mut self, evidence: UiStoryEvidence) -> &mut Self {
        self.recorded_evidence.push(evidence);
        self
    }

    pub fn record_many(
        &mut self,
        evidence: impl IntoIterator<Item = UiStoryEvidence>,
    ) -> &mut Self {
        self.recorded_evidence.extend(evidence);
        self
    }

    pub fn finish(self) -> UiStoryWorkflowRunResultV2 {
        finish_workflow_run(self)
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct UiStoryWorkflowRunResultV2 {
    pub story_id: UiStoryId,
    pub workflow_graph: Option<UiStoryWorkflowGraph>,
    #[serde(default)]
    pub evidence: Vec<UiStoryEvidence>,
    #[serde(default)]
    pub diagnostics: Vec<UiStoryDiagnostic>,
    #[serde(default)]
    pub missing_required_nodes: Vec<UiStoryWorkflowNodeId>,
    #[serde(default)]
    pub duplicate_evidence_keys: Vec<UiStoryEvidenceKey>,
    #[serde(default)]
    pub blocked_nodes: Vec<UiStoryWorkflowNodeId>,
}

impl UiStoryWorkflowRunResultV2 {
    pub(crate) fn failed_seed(
        story_id: UiStoryId,
        workflow_graph: Option<UiStoryWorkflowGraph>,
        diagnostic: UiStoryDiagnostic,
    ) -> Self {
        Self {
            story_id,
            workflow_graph,
            evidence: Vec::new(),
            diagnostics: vec![diagnostic],
            missing_required_nodes: Vec::new(),
            duplicate_evidence_keys: Vec::new(),
            blocked_nodes: Vec::new(),
        }
    }

    pub fn has_blockers(&self) -> bool {
        self.diagnostics.iter().any(UiStoryDiagnostic::is_blocking)
            || !self.missing_required_nodes.is_empty()
            || !self.duplicate_evidence_keys.is_empty()
            || !self.blocked_nodes.is_empty()
            || self.evidence.iter().any(UiStoryEvidence::blocks_node)
    }

    pub fn into_report(
        self,
        expected_outcome: UiStoryExpectedOutcomeV2,
    ) -> UiStoryWorkflowReportV2 {
        UiStoryWorkflowReportV2::from_run_result(self, expected_outcome)
    }
}

type EvidenceIdentity = (
    UiStoryWorkflowNodeId,
    UiStoryEvidenceProducerId,
    UiStoryEvidenceKey,
);

fn finish_workflow_run(run: UiStoryWorkflowRunV2) -> UiStoryWorkflowRunResultV2 {
    let node_ids = run
        .workflow_graph
        .nodes()
        .iter()
        .map(|node| node.node_id.clone())
        .collect::<BTreeSet<_>>();
    let required_nodes = run
        .workflow_graph
        .nodes()
        .iter()
        .filter(|node| node.policy == UiStoryWorkflowNodePolicy::RequiredEvidence)
        .map(|node| node.node_id.clone())
        .collect::<BTreeSet<_>>();

    let mut diagnostics = run.diagnostics.clone();
    let mut evidence_by_node: BTreeMap<UiStoryWorkflowNodeId, usize> = BTreeMap::new();
    let mut seen_evidence = BTreeSet::<EvidenceIdentity>::new();
    let mut duplicate_evidence_keys = BTreeSet::<UiStoryEvidenceKey>::new();
    let mut direct_blocking_nodes = BTreeSet::<UiStoryWorkflowNodeId>::new();

    for evidence in &run.recorded_evidence {
        diagnostics.extend(evidence.diagnostics.iter().cloned());

        if !node_ids.contains(&evidence.workflow_node_id) {
            diagnostics.push(
                UiStoryDiagnostic::error(
                    UI_STORY_WORKFLOW_NODE_MISSING,
                    UiStoryDiagnosticOrigin::Runner,
                    UiStoryDiagnosticSubject::WorkflowNode(evidence.workflow_node_id.clone()),
                    format!(
                        "evidence targets unknown workflow node {}",
                        evidence.workflow_node_id.as_str()
                    ),
                )
                .with_context("producer_id", evidence.producer_id.as_str().to_owned())
                .with_context("evidence_key", evidence.evidence_key.as_str().to_owned()),
            );
            continue;
        }

        *evidence_by_node
            .entry(evidence.workflow_node_id.clone())
            .or_insert(0) += 1;

        let identity = (
            evidence.workflow_node_id.clone(),
            evidence.producer_id.clone(),
            evidence.evidence_key.clone(),
        );
        if !seen_evidence.insert(identity) {
            duplicate_evidence_keys.insert(evidence.evidence_key.clone());
            diagnostics.push(
                UiStoryDiagnostic::error(
                    UI_STORY_RUN_DUPLICATE_EVIDENCE,
                    UiStoryDiagnosticOrigin::Runner,
                    UiStoryDiagnosticSubject::Evidence(evidence.evidence_key.clone()),
                    format!(
                        "duplicate evidence for workflow node {}, producer {}, key {}",
                        evidence.workflow_node_id.as_str(),
                        evidence.producer_id.as_str(),
                        evidence.evidence_key.as_str()
                    ),
                )
                .with_context(
                    "workflow_node_id",
                    evidence.workflow_node_id.as_str().to_owned(),
                )
                .with_context("producer_id", evidence.producer_id.as_str().to_owned())
                .with_context("evidence_key", evidence.evidence_key.as_str().to_owned()),
            );
        }

        if evidence.blocks_node() {
            direct_blocking_nodes.insert(evidence.workflow_node_id.clone());
        }
    }

    let mut missing_required_nodes = BTreeSet::<UiStoryWorkflowNodeId>::new();
    for node_id in required_nodes {
        if !evidence_by_node.contains_key(&node_id) {
            missing_required_nodes.insert(node_id.clone());
            diagnostics.push(
                UiStoryDiagnostic::error(
                    UI_STORY_RUN_MISSING_REQUIRED_EVIDENCE,
                    UiStoryDiagnosticOrigin::Runner,
                    UiStoryDiagnosticSubject::WorkflowNode(node_id.clone()),
                    format!(
                        "missing required evidence for workflow node {}",
                        node_id.as_str()
                    ),
                )
                .with_context("workflow_node_id", node_id.as_str().to_owned()),
            );
        }
    }

    let blocked_nodes = propagate_blocked_nodes(
        &run.workflow_graph,
        direct_blocking_nodes
            .into_iter()
            .chain(missing_required_nodes.iter().cloned())
            .collect(),
        &mut diagnostics,
    );

    UiStoryWorkflowRunResultV2 {
        story_id: run.story_id,
        workflow_graph: Some(run.workflow_graph),
        evidence: run.recorded_evidence,
        diagnostics,
        missing_required_nodes: missing_required_nodes.into_iter().collect(),
        duplicate_evidence_keys: duplicate_evidence_keys.into_iter().collect(),
        blocked_nodes: blocked_nodes.into_iter().collect(),
    }
}

fn propagate_blocked_nodes(
    graph: &UiStoryWorkflowGraph,
    mut blockers: BTreeSet<UiStoryWorkflowNodeId>,
    diagnostics: &mut Vec<UiStoryDiagnostic>,
) -> BTreeSet<UiStoryWorkflowNodeId> {
    let initial_blockers = blockers.clone();
    let mut blocked_nodes = BTreeSet::<UiStoryWorkflowNodeId>::new();
    let mut emitted_blocked_diagnostics = BTreeSet::<UiStoryWorkflowNodeId>::new();

    let mut changed = true;
    while changed {
        changed = false;
        for edge in graph.edges() {
            if edge.dependency != UiStoryWorkflowDependency::RequiresCompleted {
                continue;
            }
            if blockers.contains(&edge.from) && blockers.insert(edge.to.clone()) {
                changed = true;
                if !initial_blockers.contains(&edge.to) {
                    blocked_nodes.insert(edge.to.clone());
                }
            }
            if blockers.contains(&edge.from)
                && !initial_blockers.contains(&edge.to)
                && emitted_blocked_diagnostics.insert(edge.to.clone())
            {
                diagnostics.push(
                    UiStoryDiagnostic::error(
                        UI_STORY_RUN_BLOCKED_DEPENDENCY,
                        UiStoryDiagnosticOrigin::Runner,
                        UiStoryDiagnosticSubject::WorkflowNode(edge.to.clone()),
                        format!(
                            "workflow node {} is blocked by upstream dependency {}",
                            edge.to.as_str(),
                            edge.from.as_str()
                        ),
                    )
                    .with_context("workflow_node_id", edge.to.as_str().to_owned())
                    .with_context("blocked_by", edge.from.as_str().to_owned()),
                );
            }
        }
    }

    blocked_nodes
}
