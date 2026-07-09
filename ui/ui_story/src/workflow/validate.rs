use std::collections::{BTreeMap, BTreeSet};

use crate::diagnostic::{
    UI_STORY_WORKFLOW_CYCLE, UI_STORY_WORKFLOW_EDGE_ENDPOINT_UNKNOWN,
    UI_STORY_WORKFLOW_NODE_DUPLICATE, UI_STORY_WORKFLOW_NODE_MISSING, UiStoryDiagnostic,
    UiStoryDiagnosticOrigin, UiStoryDiagnosticSubject,
};
use crate::identity::UiStoryWorkflowNodeId;

use super::UiStoryWorkflowGraph;

pub fn validate_workflow_graph(graph: &UiStoryWorkflowGraph) -> Vec<UiStoryDiagnostic> {
    let mut diagnostics = Vec::new();
    let mut seen_nodes = BTreeSet::new();
    let mut duplicate_nodes = BTreeSet::new();

    for node in graph.nodes() {
        if !node.node_id.is_valid() {
            diagnostics.push(workflow_error(
                UI_STORY_WORKFLOW_NODE_MISSING,
                UiStoryDiagnosticSubject::WorkflowNode(node.node_id.clone()),
                "workflow node id is empty or has surrounding whitespace",
            ));
        }
        if !seen_nodes.insert(node.node_id.clone()) {
            duplicate_nodes.insert(node.node_id.clone());
        }
    }

    for node_id in duplicate_nodes {
        diagnostics.push(workflow_error(
            UI_STORY_WORKFLOW_NODE_DUPLICATE,
            UiStoryDiagnosticSubject::WorkflowNode(node_id.clone()),
            format!(
                "workflow node `{}` is declared more than once",
                node_id.as_str()
            ),
        ));
    }

    if !graph.terminal_node.is_valid() || !seen_nodes.contains(&graph.terminal_node) {
        diagnostics.push(workflow_error(
            UI_STORY_WORKFLOW_NODE_MISSING,
            UiStoryDiagnosticSubject::WorkflowNode(graph.terminal_node.clone()),
            format!(
                "terminal workflow node `{}` is not present in the graph",
                graph.terminal_node.as_str()
            ),
        ));
    }

    for edge in graph.edges() {
        if !seen_nodes.contains(&edge.from) {
            diagnostics.push(workflow_error(
                UI_STORY_WORKFLOW_EDGE_ENDPOINT_UNKNOWN,
                UiStoryDiagnosticSubject::WorkflowNode(edge.from.clone()),
                format!(
                    "workflow edge source `{}` is not present in the graph",
                    edge.from.as_str()
                ),
            ));
        }
        if !seen_nodes.contains(&edge.to) {
            diagnostics.push(workflow_error(
                UI_STORY_WORKFLOW_EDGE_ENDPOINT_UNKNOWN,
                UiStoryDiagnosticSubject::WorkflowNode(edge.to.clone()),
                format!(
                    "workflow edge target `{}` is not present in the graph",
                    edge.to.as_str()
                ),
            ));
        }
    }

    if diagnostics.is_empty() && graph_has_cycle(graph) {
        diagnostics.push(workflow_error(
            UI_STORY_WORKFLOW_CYCLE,
            UiStoryDiagnosticSubject::WorkflowProfile(graph.profile_id.clone()),
            format!(
                "workflow profile `{}` contains a cycle",
                graph.profile_id.as_str()
            ),
        ));
    }

    diagnostics.sort();
    diagnostics.dedup();
    diagnostics
}

fn workflow_error(
    code: &'static str,
    subject: UiStoryDiagnosticSubject,
    message: impl Into<String>,
) -> UiStoryDiagnostic {
    UiStoryDiagnostic::error(code, UiStoryDiagnosticOrigin::Workflow, subject, message)
}

fn graph_has_cycle(graph: &UiStoryWorkflowGraph) -> bool {
    let mut indegree = graph
        .nodes()
        .iter()
        .map(|node| (node.node_id.clone(), 0_usize))
        .collect::<BTreeMap<_, _>>();
    let mut outgoing = graph
        .nodes()
        .iter()
        .map(|node| {
            (
                node.node_id.clone(),
                BTreeSet::<UiStoryWorkflowNodeId>::new(),
            )
        })
        .collect::<BTreeMap<_, _>>();

    for edge in graph.edges() {
        if let Some(target_indegree) = indegree.get_mut(&edge.to) {
            *target_indegree += 1;
        }
        if let Some(outgoing_nodes) = outgoing.get_mut(&edge.from) {
            outgoing_nodes.insert(edge.to.clone());
        }
    }

    let mut ready = indegree
        .iter()
        .filter_map(|(node_id, count)| (*count == 0).then_some(node_id.clone()))
        .collect::<BTreeSet<_>>();
    let mut visited = 0_usize;

    while let Some(node_id) = ready.iter().next().cloned() {
        ready.remove(&node_id);
        visited += 1;
        if let Some(targets) = outgoing.get(&node_id) {
            for target in targets {
                if let Some(count) = indegree.get_mut(target) {
                    *count -= 1;
                    if *count == 0 {
                        ready.insert(target.clone());
                    }
                }
            }
        }
    }

    visited != graph.nodes().len()
}

#[cfg(test)]
mod tests {
    use crate::identity::{UiStoryWorkflowNodeId, UiStoryWorkflowProfileId};
    use crate::workflow::{UiStoryWorkflowEdge, UiStoryWorkflowGraph, UiStoryWorkflowNode};

    use super::*;

    #[test]
    fn duplicate_node_id_rejects_graph() {
        let graph = UiStoryWorkflowGraph::new(
            UiStoryWorkflowProfileId::new("ui_story.workflow.invalid"),
            [
                UiStoryWorkflowNode::required("source_load", "Source load"),
                UiStoryWorkflowNode::required("source_load", "Source load duplicate"),
            ],
            [],
            UiStoryWorkflowNodeId::new("source_load"),
        );

        let diagnostics = graph.validate();

        assert!(
            diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code.as_str() == UI_STORY_WORKFLOW_NODE_DUPLICATE)
        );
    }

    #[test]
    fn unknown_edge_endpoint_rejects_graph() {
        let graph = UiStoryWorkflowGraph::new(
            UiStoryWorkflowProfileId::new("ui_story.workflow.invalid"),
            [UiStoryWorkflowNode::required("source_load", "Source load")],
            [UiStoryWorkflowEdge::requires_completed(
                "source_load",
                "source_parse",
            )],
            UiStoryWorkflowNodeId::new("source_load"),
        );

        let diagnostics = graph.validate();

        assert!(
            diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code.as_str()
                    == UI_STORY_WORKFLOW_EDGE_ENDPOINT_UNKNOWN)
        );
    }

    #[test]
    fn missing_terminal_node_rejects_graph() {
        let graph = UiStoryWorkflowGraph::new(
            UiStoryWorkflowProfileId::new("ui_story.workflow.invalid"),
            [UiStoryWorkflowNode::required("source_load", "Source load")],
            [],
            UiStoryWorkflowNodeId::new("source_parse"),
        );

        let diagnostics = graph.validate();

        assert!(
            diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code.as_str() == UI_STORY_WORKFLOW_NODE_MISSING)
        );
    }

    #[test]
    fn cycle_rejects_graph() {
        let graph = UiStoryWorkflowGraph::new(
            UiStoryWorkflowProfileId::new("ui_story.workflow.invalid"),
            [
                UiStoryWorkflowNode::required("source_load", "Source load"),
                UiStoryWorkflowNode::required("source_parse", "Source parse"),
            ],
            [
                UiStoryWorkflowEdge::requires_completed("source_load", "source_parse"),
                UiStoryWorkflowEdge::requires_completed("source_parse", "source_load"),
            ],
            UiStoryWorkflowNodeId::new("source_parse"),
        );

        let diagnostics = graph.validate();

        assert!(
            diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code.as_str() == UI_STORY_WORKFLOW_CYCLE)
        );
    }
}
