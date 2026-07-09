//! UI definition diagnostics.

use crate::identity::AuthoredUiNodePath;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum UiDefinitionDiagnosticSeverity {
    Error,
    Warning,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiDefinitionDiagnostic {
    pub severity: UiDefinitionDiagnosticSeverity,
    pub code: String,
    pub message: String,
    pub path: Option<AuthoredUiNodePath>,
}

impl UiDefinitionDiagnostic {
    pub fn error(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            severity: UiDefinitionDiagnosticSeverity::Error,
            code: code.into(),
            message: message.into(),
            path: None,
        }
    }

    pub fn at_path(mut self, path: AuthoredUiNodePath) -> Self {
        self.path = Some(path);
        self
    }
}
