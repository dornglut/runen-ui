//! UiProgram diagnostic contracts.

use serde::{Deserialize, Serialize};

use crate::source_map::UiProgramSourceMapEntry;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiProgramDiagnostic {
    pub code: String,
    pub message: String,
    #[serde(default)]
    pub severity: UiProgramDiagnosticSeverity,
    #[serde(default)]
    pub source_map: Option<UiProgramSourceMapEntry>,
}

impl UiProgramDiagnostic {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            severity: UiProgramDiagnosticSeverity::Error,
            source_map: None,
        }
    }

    pub fn with_severity(mut self, severity: UiProgramDiagnosticSeverity) -> Self {
        self.severity = severity;
        self
    }

    pub fn with_source_map(mut self, source_map: UiProgramSourceMapEntry) -> Self {
        self.source_map = Some(source_map);
        self
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum UiProgramDiagnosticSeverity {
    Info,
    Warning,
    #[default]
    Error,
}
