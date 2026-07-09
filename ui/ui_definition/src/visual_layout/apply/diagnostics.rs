//! Visual layout application diagnostics.

use super::super::{UiVisualLayoutDiagnostic, UiVisualLayoutEditContext, UiVisualLayoutOperation};
use crate::AuthoredUiNodePath;

pub(super) fn diagnostic(
    code: impl Into<String>,
    message: impl Into<String>,
    operation: &UiVisualLayoutOperation,
    context: &UiVisualLayoutEditContext,
    path: Option<AuthoredUiNodePath>,
    suggested_fix: impl Into<String>,
) -> UiVisualLayoutDiagnostic {
    UiVisualLayoutDiagnostic::blocking(code, message, operation, context, path, suggested_fix)
}

#[derive(Debug, Clone)]
pub(super) struct PendingDiagnostic {
    code: String,
    message: String,
    path: Option<AuthoredUiNodePath>,
    suggested_fix: String,
}

impl PendingDiagnostic {
    pub(super) fn new(
        code: impl Into<String>,
        message: impl Into<String>,
        path: Option<AuthoredUiNodePath>,
        suggested_fix: impl Into<String>,
    ) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            path,
            suggested_fix: suggested_fix.into(),
        }
    }

    pub(super) fn with_context(self, _context: &UiVisualLayoutEditContext) -> Self {
        self
    }

    pub(super) fn into_diagnostic(
        self,
        operation: &UiVisualLayoutOperation,
        context: &UiVisualLayoutEditContext,
    ) -> UiVisualLayoutDiagnostic {
        diagnostic(
            self.code,
            self.message,
            operation,
            context,
            self.path,
            self.suggested_fix,
        )
    }
}
