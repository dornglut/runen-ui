//! ui_testing assertion error contracts.

use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UiTestingError {
    MissingSourceMap {
        source_id: String,
        target_id: String,
    },
    MissingDiagnostic {
        code: String,
    },
    UnexpectedDiagnostic {
        code: String,
    },
}

impl fmt::Display for UiTestingError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingSourceMap {
                source_id,
                target_id,
            } => write!(
                formatter,
                "missing compiled source-map entry from {source_id} to {target_id}"
            ),
            Self::MissingDiagnostic { code } => {
                write!(formatter, "missing expected diagnostic {code}")
            }
            Self::UnexpectedDiagnostic { code } => {
                write!(formatter, "unexpected diagnostic {code}")
            }
        }
    }
}

impl std::error::Error for UiTestingError {}
