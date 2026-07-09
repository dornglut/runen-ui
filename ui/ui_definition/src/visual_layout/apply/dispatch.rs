//! Visual layout operation dispatch.

use super::{
    collections::apply_replace_template,
    containers::{
        apply_move, apply_reorder, apply_split_ratio, apply_stack_axis, apply_unwrap, apply_wrap,
    },
    controls::{apply_insert, apply_remove},
    diagnostics::PendingDiagnostic,
};
use crate::UiNodeDefinition;

use crate::visual_layout::{
    UiVisualLayoutDiffChange, UiVisualLayoutEditContext, UiVisualLayoutEditKind,
    UiVisualLayoutOperation,
};

pub(super) fn apply_operation(
    root: &mut UiNodeDefinition,
    operation: &UiVisualLayoutOperation,
    context: &UiVisualLayoutEditContext,
) -> Result<Vec<UiVisualLayoutDiffChange>, PendingDiagnostic> {
    match &operation.kind {
        UiVisualLayoutEditKind::InsertNode { index, node } => {
            apply_insert(root, operation, context, *index, node.clone())
        }
        UiVisualLayoutEditKind::RemoveNode => apply_remove(root, operation),
        UiVisualLayoutEditKind::MoveNode {
            new_parent_path,
            new_index,
        } => apply_move(root, operation, new_parent_path, *new_index),
        UiVisualLayoutEditKind::ReorderSibling {
            from_index,
            to_index,
            expected_child_id,
        } => apply_reorder(root, operation, *from_index, *to_index, expected_child_id),
        UiVisualLayoutEditKind::ChangeStackAxis { axis } => {
            apply_stack_axis(root, operation, *axis)
        }
        UiVisualLayoutEditKind::ChangeSplitRatio { ratio } => {
            apply_split_ratio(root, operation, *ratio)
        }
        UiVisualLayoutEditKind::WrapSelectionInContainer {
            first_index,
            count,
            container,
        } => apply_wrap(
            root,
            operation,
            context,
            *first_index,
            *count,
            container.clone(),
        ),
        UiVisualLayoutEditKind::UnwrapContainer => apply_unwrap(root, operation),
        UiVisualLayoutEditKind::ReplaceTemplateReference { template } => {
            apply_replace_template(root, operation, template.clone())
        }
    }
}
