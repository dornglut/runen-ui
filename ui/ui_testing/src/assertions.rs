//! Structural fixture assertions over runtime artifacts.

use serde::{Deserialize, Serialize};
use ui_artifacts::{RuntimeTableKind, UiRuntimeArtifact};

use crate::UiTestingError;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceMapAssertion {
    pub source_id: String,
    pub target_id: String,
    #[serde(default)]
    pub table: Option<RuntimeTableKind>,
}

impl SourceMapAssertion {
    pub fn target_in_table(
        source_id: impl Into<String>,
        target_id: impl Into<String>,
        table: RuntimeTableKind,
    ) -> Self {
        Self {
            source_id: source_id.into(),
            target_id: target_id.into(),
            table: Some(table),
        }
    }

    pub fn assert_artifact(&self, artifact: &UiRuntimeArtifact) -> Result<(), UiTestingError> {
        let matched = artifact.manifest.source_map.entries.iter().any(|entry| {
            entry.source_id == self.source_id
                && entry.target_id == self.target_id
                && self.table.is_none_or(|table| entry.table == table)
        });
        if matched {
            Ok(())
        } else {
            Err(UiTestingError::MissingSourceMap {
                source_id: self.source_id.clone(),
                target_id: self.target_id.clone(),
            })
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DiagnosticAssertion {
    pub code: String,
    pub expectation: DiagnosticExpectation,
}

impl DiagnosticAssertion {
    pub fn code_present(code: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            expectation: DiagnosticExpectation::Present,
        }
    }

    pub fn code_absent(code: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            expectation: DiagnosticExpectation::Absent,
        }
    }

    pub fn assert_artifact(&self, artifact: &UiRuntimeArtifact) -> Result<(), UiTestingError> {
        let present = artifact
            .manifest
            .diagnostics
            .iter()
            .any(|diagnostic| diagnostic.code == self.code);
        match (self.expectation, present) {
            (DiagnosticExpectation::Present, true) | (DiagnosticExpectation::Absent, false) => {
                Ok(())
            }
            (DiagnosticExpectation::Present, false) => Err(UiTestingError::MissingDiagnostic {
                code: self.code.clone(),
            }),
            (DiagnosticExpectation::Absent, true) => Err(UiTestingError::UnexpectedDiagnostic {
                code: self.code.clone(),
            }),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DiagnosticExpectation {
    Present,
    Absent,
}
