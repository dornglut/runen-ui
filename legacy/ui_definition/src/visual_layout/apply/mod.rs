//! Visual layout edit application over authored UI templates.

mod collections;
mod containers;
mod context;
mod controls;
mod diagnostics;
mod dispatch;

#[cfg(test)]
mod tests;

use super::{
    UiVisualLayoutActivationMode, UiVisualLayoutDiagnostic, UiVisualLayoutDiff,
    UiVisualLayoutEditContext, UiVisualLayoutOperation,
};
use crate::{AuthoredUiTemplate, UiDefinitionDiagnosticSeverity};

use context::validate_target;
use diagnostics::diagnostic;
use dispatch::apply_operation;

#[derive(Debug, Clone, PartialEq)]
pub struct UiVisualLayoutEditReport {
    pub template: AuthoredUiTemplate,
    pub diff: Option<UiVisualLayoutDiff>,
    pub diagnostics: Vec<UiVisualLayoutDiagnostic>,
}

impl UiVisualLayoutEditReport {
    pub fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity == UiDefinitionDiagnosticSeverity::Error)
    }
}

pub fn apply_visual_layout_operation(
    mut template: AuthoredUiTemplate,
    operation: &UiVisualLayoutOperation,
    mode: UiVisualLayoutActivationMode,
    context: &UiVisualLayoutEditContext,
) -> UiVisualLayoutEditReport {
    let mut diagnostics = Vec::new();

    if mode == UiVisualLayoutActivationMode::Activate && operation.preview_only {
        diagnostics.push(diagnostic(
            "ui.visual_layout.preview_only_activation",
            "preview-only visual layout edits cannot be activated",
            operation,
            context,
            Some(operation.target_path.clone()),
            "serialize the edit into a deterministic authored definition patch before activation",
        ));
        return report(template, None, diagnostics);
    }

    if !context.supports_target_profile(&operation.target_profile) {
        diagnostics.push(diagnostic(
            "ui.visual_layout.target_profile.unsupported",
            format!(
                "target profile '{}' does not support this visual layout edit context",
                operation.target_profile
            ),
            operation,
            context,
            Some(operation.target_path.clone()),
            "select a supported target profile or add an explicit profile compatibility declaration",
        ));
        return report(template, None, diagnostics);
    }

    match validate_target(&template.root, operation) {
        Ok(()) => {}
        Err(diagnostic_message) => {
            diagnostics.push(diagnostic_message.into_diagnostic(operation, context));
            return report(template, None, diagnostics);
        }
    }

    let changes = match apply_operation(&mut template.root, operation, context) {
        Ok(changes) => changes,
        Err(diagnostic_message) => {
            diagnostics.push(diagnostic_message.into_diagnostic(operation, context));
            return report(template, None, diagnostics);
        }
    };

    let diff = if operation.preview_only {
        None
    } else {
        Some(UiVisualLayoutDiff {
            operation_id: operation.id.clone(),
            source_document: operation.source_document.clone(),
            target_profile: operation.target_profile.clone(),
            changes,
        })
    };

    report(template, diff, diagnostics)
}

pub(super) fn report(
    template: AuthoredUiTemplate,
    diff: Option<UiVisualLayoutDiff>,
    diagnostics: Vec<UiVisualLayoutDiagnostic>,
) -> UiVisualLayoutEditReport {
    UiVisualLayoutEditReport {
        template,
        diff,
        diagnostics,
    }
}
