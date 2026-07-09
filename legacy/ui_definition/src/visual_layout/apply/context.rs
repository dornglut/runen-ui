//! Visual layout application path and serialization helpers.

use ron::ser::{PrettyConfig, to_string_pretty};
use serde::Serialize;
use std::collections::BTreeSet;

use super::diagnostics::PendingDiagnostic;
use crate::{AuthoredUiNodePath, UiNodeDefinition, UiNodeId};

use crate::visual_layout::{UiVisualLayoutEditContext, UiVisualLayoutOperation};

pub(super) fn validate_target(
    root: &UiNodeDefinition,
    operation: &UiVisualLayoutOperation,
) -> Result<(), PendingDiagnostic> {
    let target = node_at_path(root, &operation.target_path).ok_or_else(|| {
        PendingDiagnostic::new(
            "ui.visual_layout.target.missing",
            format!(
                "target path '{}' does not exist",
                operation.target_path.as_str()
            ),
            Some(operation.target_path.clone()),
            "refresh the edit operation against the latest authored definition tree",
        )
    })?;
    if target.id() != &operation.expected_node_id {
        return Err(PendingDiagnostic::new(
            "ui.visual_layout.authored_id.unstable",
            format!(
                "target path '{}' expected node id '{}' but found '{}'",
                operation.target_path.as_str(),
                operation.expected_node_id,
                target.id()
            ),
            Some(operation.target_path.clone()),
            "refresh the edit operation against the latest authored definition tree",
        ));
    }
    Ok(())
}

pub(super) fn ensure_new_node_ids_are_unique(
    root: &UiNodeDefinition,
    node: &UiNodeDefinition,
    operation: &UiVisualLayoutOperation,
    context: &UiVisualLayoutEditContext,
) -> Result<(), PendingDiagnostic> {
    let mut existing = BTreeSet::new();
    collect_node_ids(root, &mut existing);
    let mut incoming = BTreeSet::new();
    if let Some(id) = collect_node_ids_unique(node, &mut incoming) {
        return Err(PendingDiagnostic::new(
            "ui.visual_layout.authored_id.duplicate",
            format!("new layout node id '{}' appears more than once", id),
            Some(operation.target_path.clone()),
            "allocate unique stable authored ids for every node in the inserted subtree",
        )
        .with_context(context));
    }
    for id in incoming {
        if existing.contains(&id) {
            return Err(PendingDiagnostic::new(
                "ui.visual_layout.authored_id.duplicate",
                format!("new layout node id '{}' already exists", id),
                Some(operation.target_path.clone()),
                "allocate a stable authored id that is not already present in the definition tree",
            )
            .with_context(context));
        }
    }
    Ok(())
}

fn collect_node_ids(node: &UiNodeDefinition, ids: &mut BTreeSet<UiNodeId>) {
    ids.insert(node.id().clone());
    for child in node.children() {
        collect_node_ids(child, ids);
    }
}

pub(super) fn collect_node_ids_unique(
    node: &UiNodeDefinition,
    ids: &mut BTreeSet<UiNodeId>,
) -> Option<UiNodeId> {
    if !ids.insert(node.id().clone()) {
        return Some(node.id().clone());
    }
    for child in node.children() {
        if let Some(duplicate) = collect_node_ids_unique(child, ids) {
            return Some(duplicate);
        }
    }
    None
}

pub(super) fn node_at_path<'a>(
    root: &'a UiNodeDefinition,
    path: &AuthoredUiNodePath,
) -> Option<&'a UiNodeDefinition> {
    let segments = path_segments(path);
    node_at_segments(root, &segments)
}

fn node_at_segments<'a>(
    node: &'a UiNodeDefinition,
    segments: &[&str],
) -> Option<&'a UiNodeDefinition> {
    let (head, tail) = segments.split_first()?;
    if *head != node.id().as_str() {
        return None;
    }
    if tail.is_empty() {
        return Some(node);
    }
    node.children()
        .iter()
        .find(|child| child.id().as_str() == tail[0])
        .and_then(|child| node_at_segments(child, tail))
}

pub(super) fn node_mut_at_path<'a>(
    root: &'a mut UiNodeDefinition,
    path: &AuthoredUiNodePath,
) -> Option<&'a mut UiNodeDefinition> {
    let segments = path_segments(path);
    node_mut_at_segments(root, &segments)
}

fn node_mut_at_segments<'a>(
    node: &'a mut UiNodeDefinition,
    segments: &[&str],
) -> Option<&'a mut UiNodeDefinition> {
    let (head, tail) = segments.split_first()?;
    if *head != node.id().as_str() {
        return None;
    }
    if tail.is_empty() {
        return Some(node);
    }
    let children = node.children_mut()?;
    children
        .iter_mut()
        .find(|child| child.id().as_str() == tail[0])
        .and_then(|child| node_mut_at_segments(child, tail))
}

pub(super) fn children_at_path<'a>(
    root: &'a UiNodeDefinition,
    path: &AuthoredUiNodePath,
) -> Option<&'a [UiNodeDefinition]> {
    Some(node_at_path(root, path)?.children())
}

pub(super) fn children_mut_at_path<'a>(
    root: &'a mut UiNodeDefinition,
    path: &AuthoredUiNodePath,
) -> Option<&'a mut Vec<UiNodeDefinition>> {
    node_mut_at_path(root, path)?.children_mut()
}

pub(super) fn remove_node_at_path(
    root: &mut UiNodeDefinition,
    path: &AuthoredUiNodePath,
) -> Result<UiNodeDefinition, PendingDiagnostic> {
    let parent_path = parent_path(path).ok_or_else(|| {
        PendingDiagnostic::new(
            "ui.visual_layout.root_remove.unsupported",
            "root node cannot be removed",
            Some(path.clone()),
            "remove one of the root node children instead",
        )
    })?;
    let child_id = last_path_segment(path).ok_or_else(|| {
        PendingDiagnostic::new(
            "ui.visual_layout.target.missing",
            format!("target path '{}' is empty", path.as_str()),
            Some(path.clone()),
            "target an existing authored node path",
        )
    })?;
    let parent = children_mut_at_path(root, &parent_path).ok_or_else(|| {
        PendingDiagnostic::new(
            "ui.visual_layout.target.missing",
            format!(
                "target parent path '{}' does not exist",
                parent_path.as_str()
            ),
            Some(parent_path.clone()),
            "refresh the edit operation against the latest authored definition tree",
        )
    })?;
    let index = parent
        .iter()
        .position(|node| node.id().as_str() == child_id)
        .ok_or_else(|| {
            PendingDiagnostic::new(
                "ui.visual_layout.target.missing",
                format!("target child '{}' does not exist", child_id),
                Some(path.clone()),
                "refresh the edit operation against the latest authored definition tree",
            )
        })?;
    Ok(parent.remove(index))
}

pub(super) fn child_index_at_path(
    root: &UiNodeDefinition,
    path: &AuthoredUiNodePath,
) -> Result<usize, PendingDiagnostic> {
    let parent_path = parent_path(path).ok_or_else(|| {
        PendingDiagnostic::new(
            "ui.visual_layout.root_index.unsupported",
            "root node has no sibling index",
            Some(path.clone()),
            "target one of the root node children instead",
        )
    })?;
    let child_id = last_path_segment(path).ok_or_else(|| {
        PendingDiagnostic::new(
            "ui.visual_layout.target.missing",
            format!("target path '{}' is empty", path.as_str()),
            Some(path.clone()),
            "target an existing authored node path",
        )
    })?;
    children_at_path(root, &parent_path)
        .and_then(|children| {
            children
                .iter()
                .position(|node| node.id().as_str() == child_id)
        })
        .ok_or_else(|| {
            PendingDiagnostic::new(
                "ui.visual_layout.target.missing",
                format!("target child '{}' does not exist", child_id),
                Some(path.clone()),
                "refresh the edit operation against the latest authored definition tree",
            )
        })
}

pub(super) fn parent_path(path: &AuthoredUiNodePath) -> Option<AuthoredUiNodePath> {
    path.as_str()
        .rsplit_once('/')
        .map(|(parent, _)| AuthoredUiNodePath(parent.to_string()))
}

pub(super) fn last_path_segment(path: &AuthoredUiNodePath) -> Option<&str> {
    path.as_str()
        .rsplit('/')
        .next()
        .filter(|value| !value.is_empty())
}

pub(super) fn is_descendant_path(
    candidate: &AuthoredUiNodePath,
    ancestor: &AuthoredUiNodePath,
) -> bool {
    candidate
        .as_str()
        .strip_prefix(ancestor.as_str())
        .is_some_and(|rest| rest.starts_with('/'))
}

pub(super) fn path_segments(path: &AuthoredUiNodePath) -> Vec<&str> {
    path.as_str()
        .split('/')
        .filter(|segment| !segment.is_empty())
        .collect()
}

pub(super) fn canonical_text<T: Serialize + ?Sized>(
    value: &T,
    path: &AuthoredUiNodePath,
) -> Result<String, PendingDiagnostic> {
    to_string_pretty(value, PrettyConfig::default()).map_err(|error| {
        PendingDiagnostic::new(
            "ui.visual_layout.diff.non_deterministic",
            format!("failed to serialize deterministic diff text: {error}"),
            Some(path.clone()),
            "ensure the edited value uses deterministic serde-compatible definition data",
        )
    })
}
