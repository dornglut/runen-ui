use super::*;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiRuntimeArtifactDiagnostic {
    pub code: String,
    pub message: String,
    pub severity: UiRuntimeArtifactDiagnosticSeverity,
    #[serde(default)]
    pub source_map_index: Option<u32>,
}

impl UiRuntimeArtifactDiagnostic {
    pub fn error(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            severity: UiRuntimeArtifactDiagnosticSeverity::Error,
            source_map_index: None,
        }
    }

    pub fn warning(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            severity: UiRuntimeArtifactDiagnosticSeverity::Warning,
            source_map_index: None,
        }
    }

    pub fn info(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
            severity: UiRuntimeArtifactDiagnosticSeverity::Info,
            source_map_index: None,
        }
    }

    pub fn with_source_map_index(mut self, source_map_index: u32) -> Self {
        self.source_map_index = Some(source_map_index);
        self
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UiRuntimeArtifactDiagnosticSeverity {
    Info,
    Warning,
    #[default]
    Error,
}

impl From<UiProgramDiagnosticSeverity> for UiRuntimeArtifactDiagnosticSeverity {
    fn from(value: UiProgramDiagnosticSeverity) -> Self {
        match value {
            UiProgramDiagnosticSeverity::Info => Self::Info,
            UiProgramDiagnosticSeverity::Warning => Self::Warning,
            UiProgramDiagnosticSeverity::Error => Self::Error,
        }
    }
}
