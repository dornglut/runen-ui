use std::collections::{BTreeMap, BTreeSet};

use runenui_core::{__runtime::ElementParts, ElementKey};

use crate::ReconciliationDiagnostic;

use super::{
    MountedNodeId,
    tree::{MountedTree, ReconcileStats},
};

pub(crate) struct SiblingMatches {
    pub(crate) old_keys: BTreeMap<ElementKey, Vec<(usize, MountedNodeId)>>,
    pub(crate) old_unkeyed: Vec<(usize, MountedNodeId)>,
    pub(crate) new_keys: BTreeMap<ElementKey, Vec<usize>>,
}

pub(crate) fn analyze_sibling_keys<Action>(
    tree: &MountedTree<Action>,
    old_children: &[MountedNodeId],
    new_parts: &[ElementParts<Action>],
    parent_path: &str,
    stats: &mut ReconcileStats<Action>,
) -> SiblingMatches {
    let mut old_keys: BTreeMap<ElementKey, Vec<(usize, MountedNodeId)>> = BTreeMap::new();
    let mut old_unkeyed = Vec::new();
    for (position, id) in old_children.iter().enumerate() {
        match tree.node(id).and_then(|node| node.key.as_ref()) {
            Some(key) => old_keys
                .entry(key.clone())
                .or_default()
                .push((position, id.clone())),
            None => old_unkeyed.push((position, id.clone())),
        }
    }
    let mut new_keys: BTreeMap<ElementKey, Vec<usize>> = BTreeMap::new();
    for (position, parts) in new_parts.iter().enumerate() {
        if let Some(key) = parts.key() {
            new_keys.entry(key.clone()).or_default().push(position);
        }
    }
    let all_keys: BTreeSet<_> = old_keys.keys().chain(new_keys.keys()).cloned().collect();
    for key in all_keys {
        let old = old_keys.get(&key).map(Vec::as_slice).unwrap_or_default();
        let new = new_keys.get(&key).map(Vec::as_slice).unwrap_or_default();
        if old.len() > 1 || new.len() > 1 {
            stats
                .diagnostics
                .push(ReconciliationDiagnostic::DuplicateSiblingKey {
                    key,
                    parent_path: parent_path.to_owned(),
                    old_occurrence_paths: old
                        .iter()
                        .map(|(position, _)| format!("{parent_path}/{position}"))
                        .collect(),
                    new_occurrence_paths: new
                        .iter()
                        .map(|position| format!("{parent_path}/{position}"))
                        .collect(),
                });
        }
    }
    SiblingMatches {
        old_keys,
        old_unkeyed,
        new_keys,
    }
}
