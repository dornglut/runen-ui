//! Visual layout node insert/remove operations.

use super::{
    context::{
        canonical_text, children_mut_at_path, ensure_new_node_ids_are_unique, node_at_path,
        remove_node_at_path,
    },
    diagnostics::PendingDiagnostic,
};
use crate::UiNodeDefinition;

use crate::visual_layout::{
    UiVisualLayoutDiffChange, UiVisualLayoutDiffChangeKind, UiVisualLayoutEditContext,
    UiVisualLayoutOperation,
};

pub(super) fn apply_insert(
    root: &mut UiNodeDefinition,
    operation: &UiVisualLayoutOperation,
    context: &UiVisualLayoutEditContext,
    index: usize,
    node: UiNodeDefinition,
) -> Result<Vec<UiVisualLayoutDiffChange>, PendingDiagnostic> {
    ensure_new_node_ids_are_unique(root, &node, operation, context)?;
    let parent_path = operation.target_path.clone();
    let node_path = parent_path.child(node.id());
    let after = canonical_text(&node, &node_path)?;
    let children = children_mut_at_path(root, &parent_path).ok_or_else(|| {
        PendingDiagnostic::new(
            "ui.visual_layout.layout_feature.unsupported",
            "target node cannot contain inserted children",
            Some(parent_path.clone()),
            "target a layout container node",
        )
    })?;
    if index > children.len() {
        return Err(PendingDiagnostic::new(
            "ui.visual_layout.child_index.invalid",
            format!(
                "insert index '{}' is outside parent child count '{}'",
                index,
                children.len()
            ),
            Some(parent_path),
            "choose an insertion index within the target parent child range",
        ));
    }
    children.insert(index, node);
    Ok(vec![UiVisualLayoutDiffChange {
        kind: UiVisualLayoutDiffChangeKind::Insert,
        path: node_path,
        before: None,
        after: Some(after),
    }])
}

pub(super) fn apply_remove(
    root: &mut UiNodeDefinition,
    operation: &UiVisualLayoutOperation,
) -> Result<Vec<UiVisualLayoutDiffChange>, PendingDiagnostic> {
    let before = canonical_text(
        node_at_path(root, &operation.target_path).expect("target already validated"),
        &operation.target_path,
    )?;
    let removed = remove_node_at_path(root, &operation.target_path)?;
    Ok(vec![UiVisualLayoutDiffChange {
        kind: UiVisualLayoutDiffChangeKind::Remove,
        path: operation.target_path.clone(),
        before: Some(before),
        after: Some(format!("removed '{}'", removed.id())),
    }])
}
