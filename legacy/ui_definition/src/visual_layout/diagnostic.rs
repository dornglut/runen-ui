//! Visual layout diagnostics.

use super::operation::{
    UiTargetProfileId, UiVisualLayoutEditContext, UiVisualLayoutHostId, UiVisualLayoutOperation,
    UiVisualLayoutOperationId, UiVisualLayoutSuiteId, UiVisualLayoutSurfaceId,
};
use crate::{
    AuthoredUiNodePath, UiDefinitionDiagnostic, UiDefinitionDiagnosticSeverity, UiSourceLocation,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UiVisualLayoutDiagnosticDomain {
    UiDefinition,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UiVisualLayoutActivationImpact {
    None,
    BlocksActivation,
    PreviewOnly,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiVisualLayoutDiagnostic {
    pub severity: UiDefinitionDiagnosticSeverity,
    pub code: String,
    pub message: String,
    pub path: Option<AuthoredUiNodePath>,
    pub source_location: Option<UiSourceLocation>,
    pub target_profile: UiTargetProfileId,
    pub host: Option<UiVisualLayoutHostId>,
    pub suite: Option<UiVisualLayoutSuiteId>,
    pub surface: Option<UiVisualLayoutSurfaceId>,
    pub owning_domain: UiVisualLayoutDiagnosticDomain,
    pub operation_id: UiVisualLayoutOperationId,
    pub activation_impact: UiVisualLayoutActivationImpact,
    pub suggested_fix: String,
}

impl UiVisualLayoutDiagnostic {
    pub(crate) fn blocking(
        code: impl Into<String>,
        message: impl Into<String>,
        operation: &UiVisualLayoutOperation,
        context: &UiVisualLayoutEditContext,
        path: Option<AuthoredUiNodePath>,
        suggested_fix: impl Into<String>,
    ) -> Self {
        Self {
            severity: UiDefinitionDiagnosticSeverity::Error,
            code: code.into(),
            message: message.into(),
            path,
            source_location: operation.source_location.clone(),
            target_profile: operation.target_profile.clone(),
            host: context.host.clone(),
            suite: context.suite.clone(),
            surface: context.surface.clone(),
            owning_domain: UiVisualLayoutDiagnosticDomain::UiDefinition,
            operation_id: operation.id.clone(),
            activation_impact: UiVisualLayoutActivationImpact::BlocksActivation,
            suggested_fix: suggested_fix.into(),
        }
    }

    pub fn as_definition_diagnostic(&self) -> UiDefinitionDiagnostic {
        UiDefinitionDiagnostic {
            severity: self.severity,
            code: self.code.clone(),
            message: self.message.clone(),
            path: self.path.clone(),
        }
    }
}
