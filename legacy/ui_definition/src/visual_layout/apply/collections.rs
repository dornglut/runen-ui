//! Visual layout template-reference operations.

use super::{
    context::{canonical_text, node_mut_at_path},
    diagnostics::PendingDiagnostic,
};
use crate::{AuthoredId, UiNodeDefinition};

use crate::visual_layout::{
    UiVisualLayoutDiffChange, UiVisualLayoutDiffChangeKind, UiVisualLayoutOperation,
};

pub(super) fn apply_replace_template(
    root: &mut UiNodeDefinition,
    operation: &UiVisualLayoutOperation,
    template: AuthoredId,
) -> Result<Vec<UiVisualLayoutDiffChange>, PendingDiagnostic> {
    let node = node_mut_at_path(root, &operation.target_path).expect("target already validated");
    let UiNodeDefinition::TemplateRef {
        template: current_template,
        ..
    } = node
    else {
        return Err(PendingDiagnostic::new(
            "ui.visual_layout.layout_feature.unsupported",
            "only TemplateRef nodes can replace template references",
            Some(operation.target_path.clone()),
            "target a TemplateRef node",
        ));
    };
    let before = canonical_text(current_template, &operation.target_path)?;
    *current_template = template;
    let after = canonical_text(current_template, &operation.target_path)?;
    Ok(vec![UiVisualLayoutDiffChange {
        kind: UiVisualLayoutDiffChangeKind::ReplaceTemplate,
        path: operation.target_path.clone(),
        before: Some(before),
        after: Some(after),
    }])
}
