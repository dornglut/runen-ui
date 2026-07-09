use std::collections::{BTreeMap, BTreeSet};

use crate::diagnostic::UiStoryDiagnostic;
use crate::identity::UiStoryWorkflowNodeId;

use super::{UiStoryWorkflowGraph, UiStoryWorkflowNode};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UiStoryWorkflowTopologyError {
    diagnostics: Vec<UiStoryDiagnostic>,
}

impl UiStoryWorkflowTopologyError {
    pub fn new(diagnostics: Vec<UiStoryDiagnostic>) -> Self {
        Self { diagnostics }
    }

    pub fn diagnostics(&self) -> &[UiStoryDiagnostic] {
        &self.diagnostics
    }
}

pub fn topological_nodes(
    graph: &UiStoryWorkflowGraph,
) -> Result<Vec<&UiStoryWorkflowNode>, UiStoryWorkflowTopologyError> {
    let diagnostics = graph.validate();
    if !diagnostics.is_empty() {
        return Err(UiStoryWorkflowTopologyError::new(diagnostics));
    }

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
        *indegree
            .get_mut(&edge.to)
            .expect("validated workflow edges should target known nodes") += 1;
        outgoing
            .get_mut(&edge.from)
            .expect("validated workflow edges should source known nodes")
            .insert(edge.to.clone());
    }

    let mut ready = indegree
        .iter()
        .filter_map(|(node_id, count)| (*count == 0).then_some(node_id.clone()))
        .collect::<BTreeSet<_>>();
    let mut ordered = Vec::with_capacity(graph.nodes().len());

    while let Some(node_id) = ready.iter().next().cloned() {
        ready.remove(&node_id);
        ordered.push(node_id.clone());
        if let Some(targets) = outgoing.get(&node_id) {
            for target in targets {
                let count = indegree
                    .get_mut(target)
                    .expect("validated workflow topology should reference known targets");
                *count -= 1;
                if *count == 0 {
                    ready.insert(target.clone());
                }
            }
        }
    }

    Ok(ordered
        .iter()
        .map(|node_id| {
            graph
                .node(node_id)
                .expect("validated workflow topology should reference known nodes")
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use crate::identity::{UiStoryWorkflowNodeId, UiStoryWorkflowProfileId};
    use crate::workflow::{UiStoryWorkflowEdge, UiStoryWorkflowGraph, UiStoryWorkflowNode};

    #[test]
    fn topological_order_is_deterministic() {
        let graph = UiStoryWorkflowGraph::new(
            UiStoryWorkflowProfileId::new("ui_story.workflow.parallel_fixture"),
            [
                UiStoryWorkflowNode::derived("manifest", "Manifest"),
                UiStoryWorkflowNode::required("source_load", "Source load"),
                UiStoryWorkflowNode::required("host_routes", "Host routes"),
                UiStoryWorkflowNode::required("source_parse", "Source parse"),
            ],
            [
                UiStoryWorkflowEdge::requires_completed("manifest", "source_load"),
                UiStoryWorkflowEdge::requires_completed("manifest", "host_routes"),
                UiStoryWorkflowEdge::requires_completed("source_load", "source_parse"),
            ],
            UiStoryWorkflowNodeId::new("source_parse"),
        );

        let order = graph.topological_nodes().expect("graph should be valid");

        assert_eq!(
            order
                .iter()
                .map(|node| node.node_id.as_str())
                .collect::<Vec<_>>(),
            vec!["manifest", "host_routes", "source_load", "source_parse"]
        );
    }

    #[test]
    fn invalid_graph_returns_diagnostics() {
        let graph = UiStoryWorkflowGraph::new(
            UiStoryWorkflowProfileId::new("ui_story.workflow.invalid"),
            [UiStoryWorkflowNode::required("source_load", "Source load")],
            [UiStoryWorkflowEdge::requires_completed(
                "source_load",
                "source_parse",
            )],
            UiStoryWorkflowNodeId::new("source_load"),
        );

        let error = graph
            .topological_nodes()
            .expect_err("invalid graph should not produce topology");

        assert!(!error.diagnostics().is_empty());
    }
}
