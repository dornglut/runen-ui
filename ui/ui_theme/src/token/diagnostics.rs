//! Theme token diagnostics.

use serde::{Deserialize, Serialize};

use super::{
    ThemeTargetProfileId, ThemeTokenDeclaration, ThemeTokenId, ThemeTokenResolveRequest,
    ThemeTokenSourceId, ThemeTokenSourcePath,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThemeTokenDiagnosticSeverity {
    Error,
    Warning,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ThemeTokenActivationImpact {
    None,
    BlocksActivation,
    PreviewOnly,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ThemeTokenDiagnostic {
    pub severity: ThemeTokenDiagnosticSeverity,
    pub code: String,
    pub message: String,
    pub token: Option<ThemeTokenId>,
    pub target_profile: ThemeTargetProfileId,
    pub owning_domain: &'static str,
    pub source: Option<ThemeTokenSourceId>,
    pub source_path: Option<ThemeTokenSourcePath>,
    pub alias_path: Vec<ThemeTokenId>,
    pub winning_source: Option<ThemeTokenSourceId>,
    pub losing_sources: Vec<ThemeTokenSourceId>,
    pub activation_impact: ThemeTokenActivationImpact,
    pub suggested_fix: String,
}

pub(super) fn error(
    code: impl Into<String>,
    message: impl Into<String>,
    token: Option<ThemeTokenId>,
    request: &ThemeTokenResolveRequest,
    source: Option<ThemeTokenSourceId>,
    suggested_fix: impl Into<String>,
) -> ThemeTokenDiagnostic {
    ThemeTokenDiagnostic {
        severity: ThemeTokenDiagnosticSeverity::Error,
        code: code.into(),
        message: message.into(),
        token,
        target_profile: request.target_profile.clone(),
        owning_domain: "domain/ui/ui_theme",
        source,
        source_path: None,
        alias_path: Vec::new(),
        winning_source: None,
        losing_sources: Vec::new(),
        activation_impact: ThemeTokenActivationImpact::BlocksActivation,
        suggested_fix: suggested_fix.into(),
    }
}

pub(super) fn error_for_declaration(
    code: impl Into<String>,
    message: impl Into<String>,
    declaration: &ThemeTokenDeclaration,
    request: &ThemeTokenResolveRequest,
    suggested_fix: impl Into<String>,
) -> ThemeTokenDiagnostic {
    error(
        code,
        message,
        Some(declaration.id.clone()),
        request,
        Some(declaration.source.clone()),
        suggested_fix,
    )
    .with_source_path(declaration.source_path.clone())
}

impl ThemeTokenDiagnostic {
    pub(super) fn with_source_path(mut self, source_path: Option<ThemeTokenSourcePath>) -> Self {
        self.source_path = source_path;
        self
    }

    pub(super) fn with_alias_path(mut self, alias_path: Vec<ThemeTokenId>) -> Self {
        self.alias_path = alias_path;
        self
    }

    pub(super) fn with_sources(
        mut self,
        winning_source: Option<ThemeTokenSourceId>,
        losing_sources: Vec<ThemeTokenSourceId>,
    ) -> Self {
        self.winning_source = winning_source;
        self.losing_sources = losing_sources;
        self
    }

    pub(super) fn with_activation_impact(
        mut self,
        activation_impact: ThemeTokenActivationImpact,
    ) -> Self {
        self.activation_impact = activation_impact;
        self
    }
}
