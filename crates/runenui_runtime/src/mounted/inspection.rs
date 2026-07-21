use super::{
    MountedNodeId,
    diagnostics::{DuplicateIdentityKind, IdentityDiagnostic},
    node::{MountedNodeRef, MountedTreeIndex},
    tree::MountedTree,
};
use runenui_core::{ElementId, ElementKey};
use std::collections::{BTreeMap, HashMap};

impl<Action> MountedTree<Action> {
    pub(super) fn preorder_ids(&self) -> Vec<MountedNodeId> {
        fn visit<Action>(
            tree: &MountedTree<Action>,
            id: &MountedNodeId,
            out: &mut Vec<MountedNodeId>,
        ) {
            let Some(node) = tree.node(id) else {
                return;
            };
            out.push(id.clone());
            for child in &node.children {
                visit(tree, child, out);
            }
        }
        let mut ids = Vec::new();
        if let Some(root) = &self.root {
            visit(self, root, &mut ids);
        }
        ids
    }

    pub(crate) fn publication_preorder_ids(&self) -> Vec<MountedNodeId> {
        self.preorder_ids()
    }

    pub(crate) fn index(&mut self) -> MountedTreeIndex<'_, Action> {
        let ids = self.preorder_ids();
        for id in &ids {
            let _ = self.activation(id);
        }
        let diagnostics = self.identity_diagnostics(&ids);
        let nodes = ids
            .iter()
            .filter_map(|id| self.node(id))
            .map(|node| MountedNodeRef { node })
            .collect();
        MountedTreeIndex { nodes, diagnostics }
    }

    fn identity_diagnostics(&self, ids: &[MountedNodeId]) -> Vec<IdentityDiagnostic> {
        let mut diagnostics = Vec::new();
        let mut authored: BTreeMap<ElementId, String> = BTreeMap::new();
        let mut paths = HashMap::new();
        if let Some(root) = &self.root {
            self.collect_paths(root, "root", &mut paths);
        }
        for (preorder_index, id) in ids.iter().enumerate() {
            let node = self
                .node(id)
                .unwrap_or_else(|| unreachable!("preorder contains live nodes"));
            let path = paths.get(id).cloned().unwrap_or_default();
            for authoring in &node.authoring_diagnostics {
                diagnostics.push(IdentityDiagnostic {
                    kind: if authoring.field() == "id" {
                        DuplicateIdentityKind::InvalidElementId
                    } else {
                        DuplicateIdentityKind::InvalidElementKey
                    },
                    value: authoring.value().to_owned(),
                    first_path: path.clone(),
                    duplicate_path: path.clone(),
                    preorder_index,
                });
            }
            if let Some(authored_id) = &node.authored_id {
                if let Some(first) = authored.get(authored_id) {
                    diagnostics.push(IdentityDiagnostic {
                        kind: DuplicateIdentityKind::ElementId,
                        value: authored_id.as_str().to_owned(),
                        first_path: first.clone(),
                        duplicate_path: path.clone(),
                        preorder_index,
                    });
                } else {
                    authored.insert(authored_id.clone(), path.clone());
                }
            }
            let mut keys: BTreeMap<ElementKey, String> = BTreeMap::new();
            for child in &node.children {
                if let Some(child_node) = self.node(child)
                    && let Some(key) = &child_node.key
                {
                    let child_path = paths.get(child).cloned().unwrap_or_default();
                    if let Some(first) = keys.get(key) {
                        diagnostics.push(IdentityDiagnostic {
                            kind: DuplicateIdentityKind::SiblingKey,
                            value: key.as_str().to_owned(),
                            first_path: first.clone(),
                            duplicate_path: child_path,
                            preorder_index: ids
                                .iter()
                                .position(|candidate| candidate == child)
                                .unwrap_or(preorder_index),
                        });
                    } else {
                        keys.insert(key.clone(), child_path);
                    }
                }
            }
        }
        diagnostics.sort_by_key(IdentityDiagnostic::preorder_index);
        diagnostics
    }

    fn collect_paths(
        &self,
        id: &MountedNodeId,
        path: &str,
        paths: &mut HashMap<MountedNodeId, String>,
    ) {
        paths.insert(id.clone(), path.to_owned());
        if let Some(node) = self.node(id) {
            for (index, child) in node.children.iter().enumerate() {
                self.collect_paths(child, &format!("{path}/{index}"), paths);
            }
        }
    }
}
