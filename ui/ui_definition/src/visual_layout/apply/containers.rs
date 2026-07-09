//! Visual layout container transform operations.

use super::{
    context::{
        canonical_text, child_index_at_path, children_at_path, children_mut_at_path,
        ensure_new_node_ids_are_unique, is_descendant_path, node_at_path, node_mut_at_path,
        parent_path, remove_node_at_path,
    },
    diagnostics::PendingDiagnostic,
};
use crate::{AuthoredUiNodePath, UiAxisDefinition, UiNodeDefinition, UiNodeId};

use crate::visual_layout::{
    UiVisualLayoutDiffChange, UiVisualLayoutDiffChangeKind, UiVisualLayoutEditContext,
    UiVisualLayoutOperation,
};

pub(super) fn apply_move(
    root: &mut UiNodeDefinition,
    operation: &UiVisualLayoutOperation,
    new_parent_path: &AuthoredUiNodePath,
    new_index: usize,
) -> Result<Vec<UiVisualLayoutDiffChange>, PendingDiagnostic> {
    if is_descendant_path(new_parent_path, &operation.target_path) {
        return Err(PendingDiagnostic::new(
            "ui.visual_layout.composition_conflict",
            "a node cannot be moved into one of its descendants",
            Some(operation.target_path.clone()),
            "choose a target parent outside the moved node subtree",
        ));
    }
    let old_parent_path = parent_path(&operation.target_path).ok_or_else(|| {
        PendingDiagnostic::new(
            "ui.visual_layout.root_move.unsupported",
            "root node cannot be moved",
            Some(operation.target_path.clone()),
            "move one of the root node children instead",
        )
    })?;
    let old_index = child_index_at_path(root, &operation.target_path)?;
    let new_parent_len = children_at_path(root, new_parent_path)
        .ok_or_else(|| {
            PendingDiagnostic::new(
                "ui.visual_layout.target.missing",
                "move target parent does not exist",
                Some(new_parent_path.clone()),
                "target an existing layout container",
            )
        })?
        .len();
    let adjusted_index =
        if old_parent_path == *new_parent_path && old_index < new_index && new_index > 0 {
            new_index - 1
        } else {
            new_index
        };
    let effective_len = if old_parent_path == *new_parent_path {
        new_parent_len.saturating_sub(1)
    } else {
        new_parent_len
    };
    if adjusted_index > effective_len {
        return Err(PendingDiagnostic::new(
            "ui.visual_layout.child_index.invalid",
            format!(
                "move index '{}' is outside target parent child count '{}'",
                new_index, effective_len
            ),
            Some(new_parent_path.clone()),
            "choose a move index within the target parent child range",
        ));
    }

    let moved = remove_node_at_path(root, &operation.target_path)?;
    let moved_id = moved.id().clone();
    let moved_text = canonical_text(&moved, &operation.target_path)?;
    let new_path = new_parent_path.child(&moved_id);
    let new_parent = children_mut_at_path(root, new_parent_path).ok_or_else(|| {
        PendingDiagnostic::new(
            "ui.visual_layout.target.missing",
            "move target parent does not exist after removal",
            Some(new_parent_path.clone()),
            "target an existing layout container",
        )
    })?;
    new_parent.insert(adjusted_index, moved);
    Ok(vec![UiVisualLayoutDiffChange {
        kind: UiVisualLayoutDiffChangeKind::Move,
        path: operation.target_path.clone(),
        before: Some(moved_text),
        after: Some(format!("moved to '{}'", new_path.as_str())),
    }])
}

pub(super) fn apply_reorder(
    root: &mut UiNodeDefinition,
    operation: &UiVisualLayoutOperation,
    from_index: usize,
    to_index: usize,
    expected_child_id: &UiNodeId,
) -> Result<Vec<UiVisualLayoutDiffChange>, PendingDiagnostic> {
    let children = children_mut_at_path(root, &operation.target_path).ok_or_else(|| {
        PendingDiagnostic::new(
            "ui.visual_layout.layout_feature.unsupported",
            "target node cannot reorder children",
            Some(operation.target_path.clone()),
            "target a layout container node",
        )
    })?;
    if from_index >= children.len() || to_index >= children.len() {
        return Err(PendingDiagnostic::new(
            "ui.visual_layout.child_index.invalid",
            format!(
                "reorder range '{}' -> '{}' is outside child count '{}'",
                from_index,
                to_index,
                children.len()
            ),
            Some(operation.target_path.clone()),
            "choose reorder indices within the target parent child range",
        ));
    }
    if children[from_index].id() != expected_child_id {
        return Err(PendingDiagnostic::new(
            "ui.visual_layout.authored_id.unstable",
            format!(
                "expected child id '{}' at index '{}' but found '{}'",
                expected_child_id,
                from_index,
                children[from_index].id()
            ),
            Some(operation.target_path.clone()),
            "refresh the edit operation against the latest authored definition tree",
        ));
    }

    let moved = children.remove(from_index);
    let moved_id = moved.id().clone();
    children.insert(to_index, moved);
    Ok(vec![UiVisualLayoutDiffChange {
        kind: UiVisualLayoutDiffChangeKind::Reorder,
        path: operation.target_path.child(&moved_id),
        before: Some(format!("index {from_index}")),
        after: Some(format!("index {to_index}")),
    }])
}

pub(super) fn apply_stack_axis(
    root: &mut UiNodeDefinition,
    operation: &UiVisualLayoutOperation,
    axis: UiAxisDefinition,
) -> Result<Vec<UiVisualLayoutDiffChange>, PendingDiagnostic> {
    let node = node_mut_at_path(root, &operation.target_path).expect("target already validated");
    let UiNodeDefinition::Stack {
        axis: current_axis, ..
    } = node
    else {
        return Err(PendingDiagnostic::new(
            "ui.visual_layout.layout_feature.unsupported",
            "only Stack nodes can change stack axis",
            Some(operation.target_path.clone()),
            "target a Stack node or convert the node through an explicit structural edit first",
        ));
    };
    let before = canonical_text(current_axis, &operation.target_path)?;
    *current_axis = axis;
    let after = canonical_text(current_axis, &operation.target_path)?;
    Ok(vec![UiVisualLayoutDiffChange {
        kind: UiVisualLayoutDiffChangeKind::Update,
        path: operation.target_path.clone(),
        before: Some(before),
        after: Some(after),
    }])
}

pub(super) fn apply_split_ratio(
    root: &mut UiNodeDefinition,
    operation: &UiVisualLayoutOperation,
    ratio: f32,
) -> Result<Vec<UiVisualLayoutDiffChange>, PendingDiagnostic> {
    if !(0.0..1.0).contains(&ratio) {
        return Err(PendingDiagnostic::new(
            "ui.visual_layout.split.ratio_out_of_range",
            format!("split ratio '{ratio}' must be greater than 0 and less than 1"),
            Some(operation.target_path.clone()),
            "choose a split ratio in the open range 0.0..1.0",
        ));
    }
    let node = node_mut_at_path(root, &operation.target_path).expect("target already validated");
    let UiNodeDefinition::Split {
        ratio: current_ratio,
        ..
    } = node
    else {
        return Err(PendingDiagnostic::new(
            "ui.visual_layout.layout_feature.unsupported",
            "only Split nodes can change split ratio",
            Some(operation.target_path.clone()),
            "target a Split node or convert the node through an explicit structural edit first",
        ));
    };
    let before = canonical_text(current_ratio, &operation.target_path)?;
    *current_ratio = ratio;
    let after = canonical_text(current_ratio, &operation.target_path)?;
    Ok(vec![UiVisualLayoutDiffChange {
        kind: UiVisualLayoutDiffChangeKind::Update,
        path: operation.target_path.clone(),
        before: Some(before),
        after: Some(after),
    }])
}

pub(super) fn apply_wrap(
    root: &mut UiNodeDefinition,
    operation: &UiVisualLayoutOperation,
    context: &UiVisualLayoutEditContext,
    first_index: usize,
    count: usize,
    mut container: UiNodeDefinition,
) -> Result<Vec<UiVisualLayoutDiffChange>, PendingDiagnostic> {
    ensure_new_node_ids_are_unique(root, &container, operation, context)?;
    if count == 0 {
        return Err(PendingDiagnostic::new(
            "ui.visual_layout.child_count.invalid",
            "wrap selection requires at least one child",
            Some(operation.target_path.clone()),
            "select one or more sibling nodes before wrapping",
        ));
    }
    if !container.children().is_empty() {
        return Err(PendingDiagnostic::new(
            "ui.visual_layout.container.children_non_empty",
            "wrap container must be supplied without authored children",
            Some(operation.target_path.clone()),
            "provide an empty layout container for wrapping selected children",
        ));
    }
    if matches!(container, UiNodeDefinition::Split { .. }) && count != 2 {
        return Err(PendingDiagnostic::new(
            "ui.visual_layout.split.child_count",
            "wrapping in a Split container requires exactly two selected children",
            Some(operation.target_path.clone()),
            "select exactly two siblings or use a non-Split layout container",
        ));
    }
    let container_id = container.id().clone();
    let children = children_mut_at_path(root, &operation.target_path).ok_or_else(|| {
        PendingDiagnostic::new(
            "ui.visual_layout.layout_feature.unsupported",
            "target node cannot wrap children",
            Some(operation.target_path.clone()),
            "target a layout container node",
        )
    })?;
    if first_index >= children.len() || first_index + count > children.len() {
        return Err(PendingDiagnostic::new(
            "ui.visual_layout.child_index.invalid",
            format!(
                "wrap range '{}..{}' is outside child count '{}'",
                first_index,
                first_index + count,
                children.len()
            ),
            Some(operation.target_path.clone()),
            "choose a wrap range inside the target parent child range",
        ));
    }
    let selected: Vec<_> = children.drain(first_index..first_index + count).collect();
    *container.children_mut().ok_or_else(|| {
        PendingDiagnostic::new(
            "ui.visual_layout.layout_feature.unsupported",
            "wrap target is not a layout container",
            Some(operation.target_path.clone()),
            "provide a Panel, Row, Column, Stack, Scroll, or Split container",
        )
    })? = selected;
    let after = canonical_text(&container, &operation.target_path.child(&container_id))?;
    children.insert(first_index, container);
    Ok(vec![UiVisualLayoutDiffChange {
        kind: UiVisualLayoutDiffChangeKind::Wrap,
        path: operation.target_path.child(&container_id),
        before: Some(format!("wrapped {count} children")),
        after: Some(after),
    }])
}

pub(super) fn apply_unwrap(
    root: &mut UiNodeDefinition,
    operation: &UiVisualLayoutOperation,
) -> Result<Vec<UiVisualLayoutDiffChange>, PendingDiagnostic> {
    let children = node_at_path(root, &operation.target_path)
        .expect("target already validated")
        .children();
    if children.is_empty() {
        return Err(PendingDiagnostic::new(
            "ui.visual_layout.child_count.invalid",
            "unwrap requires a layout container with children",
            Some(operation.target_path.clone()),
            "target a non-empty layout container",
        ));
    }
    let parent_path = parent_path(&operation.target_path).ok_or_else(|| {
        PendingDiagnostic::new(
            "ui.visual_layout.root_unwrap.unsupported",
            "root node cannot be unwrapped",
            Some(operation.target_path.clone()),
            "unwrap a child layout container instead",
        )
    })?;
    let index = child_index_at_path(root, &operation.target_path)?;
    let mut container = remove_node_at_path(root, &operation.target_path)?;
    let unwrapped = std::mem::take(container.children_mut().ok_or_else(|| {
        PendingDiagnostic::new(
            "ui.visual_layout.layout_feature.unsupported",
            "target node cannot be unwrapped",
            Some(operation.target_path.clone()),
            "target a layout container node",
        )
    })?);
    let count = unwrapped.len();
    let parent = children_mut_at_path(root, &parent_path).expect("parent already validated");
    for (offset, child) in unwrapped.into_iter().enumerate() {
        parent.insert(index + offset, child);
    }
    Ok(vec![UiVisualLayoutDiffChange {
        kind: UiVisualLayoutDiffChangeKind::Unwrap,
        path: operation.target_path.clone(),
        before: Some(format!("container '{}'", container.id())),
        after: Some(format!("unwrapped {count} children")),
    }])
}
