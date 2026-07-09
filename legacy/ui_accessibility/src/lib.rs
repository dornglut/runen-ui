//! File: domain/ui/ui_accessibility/src/lib.rs
//! Crate: ui_accessibility

use serde::{Deserialize, Serialize};
use ui_artifacts::{AccessibilityRow, UiRuntimeArtifact};
use ui_program::{AccessibilityNode, AccessibilityNodeId, AccessibilityRole, BindingEndpointId};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccessibilityTreeNode {
    pub node: AccessibilityNode,
    #[serde(default)]
    pub source_map_index: Option<u32>,
}

impl AccessibilityTreeNode {
    pub fn from_row(row: &AccessibilityRow) -> Self {
        Self {
            node: row.node.clone(),
            source_map_index: row.source_map_index,
        }
    }

    pub fn node_id(&self) -> &AccessibilityNodeId {
        &self.node.node_id
    }

    pub fn label_source(&self) -> Option<&BindingEndpointId> {
        self.node.label_source.as_ref()
    }

    pub fn is_source_mapped(&self) -> bool {
        self.source_map_index.is_some()
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccessibilityTree {
    pub nodes: Vec<AccessibilityTreeNode>,
    pub diagnostics: Vec<AccessibilityDiagnostic>,
}

impl AccessibilityTree {
    pub fn from_artifact(artifact: &UiRuntimeArtifact) -> Self {
        Self::from_rows(&artifact.tables.accessibility.rows)
    }

    pub fn from_rows(rows: &[AccessibilityRow]) -> Self {
        let nodes = rows
            .iter()
            .map(AccessibilityTreeNode::from_row)
            .collect::<Vec<_>>();
        let diagnostics = nodes
            .iter()
            .filter(|node| !node.is_source_mapped())
            .map(|node| AccessibilityDiagnostic {
                code: "ui.accessibility.source_map_missing".to_owned(),
                node_id: node.node.node_id.as_str().to_owned(),
                message: format!(
                    "accessibility node {} has no source-map row",
                    node.node.node_id.as_str()
                ),
            })
            .collect();

        Self { nodes, diagnostics }
    }

    pub fn passed(&self) -> bool {
        self.diagnostics.is_empty()
    }

    pub fn role_count(&self, role: AccessibilityRole) -> usize {
        self.nodes
            .iter()
            .filter(|node| node.node.role == role)
            .count()
    }

    pub fn source_mapped_count(&self) -> usize {
        self.nodes
            .iter()
            .filter(|node| node.is_source_mapped())
            .count()
    }

    pub fn label_source_for_node(
        &self,
        node_id: &AccessibilityNodeId,
    ) -> Option<&BindingEndpointId> {
        self.nodes
            .iter()
            .find(|node| node.node_id() == node_id)
            .and_then(AccessibilityTreeNode::label_source)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccessibilityDiagnostic {
    pub code: String,
    pub node_id: String,
    pub message: String,
}

#[cfg(test)]
mod tests {
    use super::*;
    use ui_artifacts::UiRuntimeArtifact;
    use ui_program::{
        AccessibilityNode, AccessibilityNodeId, AccessibilityRole, BindingEndpointId,
        ControlGraphNode, ControlKindRef, ControlNodeId, ControlPackageRef, UiProgram, UiProgramId,
        UiProgramSourceId, UiProgramSourceMapAttachment, UiProgramSourceMapEntry,
        UiProgramTargetId, UiProgramVersion,
    };

    #[test]
    fn accessibility_contract_projects_artifact_rows_labels_and_source_maps() {
        let artifact = UiRuntimeArtifact::from_program(&accessibility_program());
        let tree = AccessibilityTree::from_artifact(&artifact);

        assert!(tree.passed());
        assert_eq!(tree.nodes.len(), 1);
        assert_eq!(tree.role_count(AccessibilityRole::Label), 1);
        assert_eq!(tree.source_mapped_count(), 1);
        assert_eq!(
            tree.label_source_for_node(&AccessibilityNodeId::new("accessibility.fixture.title"))
                .map(BindingEndpointId::as_str),
            Some("binding.fixture.title.state")
        );
    }

    fn accessibility_program() -> UiProgram {
        let control_id = ControlNodeId::new("control.fixture.title");
        let source_map = UiProgramSourceMapAttachment::new(UiProgramSourceMapEntry::new(
            UiProgramSourceId::new("definition.fixture.title"),
            UiProgramTargetId::new("program.fixture.accessibility.title"),
        ));
        let mut program = UiProgram::new(
            UiProgramId::new("fixture.accessibility"),
            UiProgramVersion::new(1),
        );
        program.graphs.control.add_node(ControlGraphNode::new(
            control_id.clone(),
            ControlPackageRef::new("runenwerk.ui.controls"),
            ControlKindRef::new("runenwerk.ui.controls.label"),
        ));
        program.graphs.accessibility.nodes.push(
            AccessibilityNode::new(
                AccessibilityNodeId::new("accessibility.fixture.title"),
                control_id,
                AccessibilityRole::Label,
            )
            .with_label_source(BindingEndpointId::new("binding.fixture.title.state"))
            .with_source_map(source_map),
        );
        program
    }
}
