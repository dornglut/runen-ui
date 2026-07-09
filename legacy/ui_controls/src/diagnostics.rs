//! File: domain/ui/ui_controls/src/diagnostics.rs
//! Crate: ui_controls

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::package::ControlKindId;

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ControlDiagnosticId(String);

impl ControlDiagnosticId {
    pub fn new(value: impl Into<String>) -> Self {
        Self::try_new(value).expect("control diagnostic IDs must be namespaced")
    }

    pub fn try_new(value: impl Into<String>) -> Result<Self, ControlDiagnosticContractError> {
        let value = value.into();
        validate_diagnostic_id(&value, "diagnostic")?;
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ControlDiagnosticSeverity {
    Info,
    Warning,
    #[default]
    Error,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ControlDiagnosticKind {
    #[default]
    ContractValidation,
    MissingSchema,
    MissingKernel,
    MissingFixture,
    MissingDiagnostic,
    MissingMigration,
    MissingStory,
    MissingRoute,
    MissingCapability,
    MissingTargetProfile,
    MissingMountEvidence,
    DuplicateId,
    InvalidMigration,
    InvalidDeprecation,
    UnsupportedTargetProfile,
    BudgetViolation,
    AccessibilityRequirementMissing,
    RenderEvidenceMissing,
    BindingRequirementMissing,
    ThemeTokenRequirementMissing,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ControlDiagnosticScope {
    #[default]
    Package,
    ControlKind {
        control_kind_id: String,
    },
    Registry,
    Artifact,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlDiagnosticDescriptor {
    pub diagnostic_id: ControlDiagnosticId,
    pub severity: ControlDiagnosticSeverity,
    pub kind: ControlDiagnosticKind,
    pub scope: ControlDiagnosticScope,
    pub message_template: String,
}

impl ControlDiagnosticDescriptor {
    pub fn new(diagnostic_id: ControlDiagnosticId, message_template: impl Into<String>) -> Self {
        Self {
            diagnostic_id,
            severity: ControlDiagnosticSeverity::Error,
            kind: ControlDiagnosticKind::ContractValidation,
            scope: ControlDiagnosticScope::Package,
            message_template: message_template.into(),
        }
    }

    pub fn contract(
        diagnostic_id: ControlDiagnosticId,
        control_kind_id: ControlKindId,
        message_template: impl Into<String>,
    ) -> Self {
        Self::new(diagnostic_id, message_template)
            .with_kind(ControlDiagnosticKind::ContractValidation)
            .with_scope(ControlDiagnosticScope::ControlKind {
                control_kind_id: control_kind_id.as_str().to_owned(),
            })
            .with_severity(ControlDiagnosticSeverity::Error)
    }

    pub fn with_severity(mut self, severity: ControlDiagnosticSeverity) -> Self {
        self.severity = severity;
        self
    }
    pub fn with_kind(mut self, kind: ControlDiagnosticKind) -> Self {
        self.kind = kind;
        self
    }
    pub fn with_scope(mut self, scope: ControlDiagnosticScope) -> Self {
        self.scope = scope;
        self
    }
    pub fn validate_contract(&self) -> Result<(), ControlDiagnosticContractError> {
        if self.message_template.trim().is_empty() {
            return Err(ControlDiagnosticContractError::EmptyMessageTemplate {
                diagnostic_id: self.diagnostic_id.as_str().to_owned(),
            });
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlDiagnosticRecord {
    pub diagnostic_id: ControlDiagnosticId,
    pub control_kind_id: String,
    pub message: String,
}

impl ControlDiagnosticRecord {
    pub fn new(
        diagnostic_id: ControlDiagnosticId,
        control_kind_id: impl Into<String>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            diagnostic_id,
            control_kind_id: control_kind_id.into(),
            message: message.into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ControlDiagnosticContractError {
    EmptyId { kind: &'static str },
    UnnamespacedId { kind: &'static str, value: String },
    InvalidIdCharacter { kind: &'static str, value: String },
    EmptyMessageTemplate { diagnostic_id: String },
}

impl fmt::Display for ControlDiagnosticContractError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyId { kind } => write!(formatter, "empty control {kind} id"),
            Self::UnnamespacedId { kind, value } => {
                write!(formatter, "control {kind} id {value} is not namespaced")
            }
            Self::InvalidIdCharacter { kind, value } => write!(
                formatter,
                "control {kind} id {value} contains an invalid character"
            ),
            Self::EmptyMessageTemplate { diagnostic_id } => write!(
                formatter,
                "control diagnostic {diagnostic_id} has empty message template"
            ),
        }
    }
}

impl std::error::Error for ControlDiagnosticContractError {}

fn validate_diagnostic_id(
    value: &str,
    kind: &'static str,
) -> Result<(), ControlDiagnosticContractError> {
    if value.is_empty() {
        return Err(ControlDiagnosticContractError::EmptyId { kind });
    }
    if !value.contains('.') {
        return Err(ControlDiagnosticContractError::UnnamespacedId {
            kind,
            value: value.to_owned(),
        });
    }
    if !value
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || matches!(character, '.' | '_' | '-'))
    {
        return Err(ControlDiagnosticContractError::InvalidIdCharacter {
            kind,
            value: value.to_owned(),
        });
    }
    Ok(())
}
