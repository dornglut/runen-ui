//! Persistence activation diagnostics.

use serde::{Deserialize, Serialize};

use crate::{UiDefinitionDiagnosticSeverity, UiSourceLocation};

use super::{
    UiActivationRequestDescriptor, UiMigrationReportDescriptor, UiPersistenceActivationRequestId,
    UiPersistenceDiagnosticRef, UiPersistenceDiffDescriptor, UiPersistenceDiffId,
    UiPersistenceDocumentDescriptor, UiPersistenceDocumentId, UiPersistenceFieldPath,
    UiPersistenceMigrationReportId, UiPersistenceSourcePackageId, UiPersistenceTargetProfileId,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UiPersistenceActivationImpact {
    None,
    PreviewOnly,
    BlocksActivation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UiPersistenceDiagnosticDomain {
    UiDefinition,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiPersistenceDiagnostic {
    pub severity: UiDefinitionDiagnosticSeverity,
    pub code: String,
    pub message: String,
    #[serde(default)]
    pub document: Option<UiPersistenceDocumentId>,
    #[serde(default)]
    pub migration_report: Option<UiPersistenceMigrationReportId>,
    #[serde(default)]
    pub diff: Option<UiPersistenceDiffId>,
    #[serde(default)]
    pub activation_request: Option<UiPersistenceActivationRequestId>,
    #[serde(default)]
    pub path: Option<UiPersistenceFieldPath>,
    #[serde(default)]
    pub source_location: Option<UiSourceLocation>,
    #[serde(default)]
    pub target_profile: Option<UiPersistenceTargetProfileId>,
    #[serde(default)]
    pub source_schema_version: Option<u32>,
    #[serde(default)]
    pub target_schema_version: Option<u32>,
    pub owning_domain: UiPersistenceDiagnosticDomain,
    #[serde(default)]
    pub source_package: Option<UiPersistenceSourcePackageId>,
    #[serde(default)]
    pub expected_diagnostics: Vec<UiPersistenceDiagnosticRef>,
    #[serde(default)]
    pub actual_diagnostics: Vec<UiPersistenceDiagnosticRef>,
    pub activation_impact: UiPersistenceActivationImpact,
    pub suggested_fix: String,
}

impl UiPersistenceDiagnostic {
    pub(super) fn error(
        code: impl Into<String>,
        message: impl Into<String>,
        suggested_fix: impl Into<String>,
    ) -> Self {
        Self {
            severity: UiDefinitionDiagnosticSeverity::Error,
            code: code.into(),
            message: message.into(),
            document: None,
            migration_report: None,
            diff: None,
            activation_request: None,
            path: None,
            source_location: None,
            target_profile: None,
            source_schema_version: None,
            target_schema_version: None,
            owning_domain: UiPersistenceDiagnosticDomain::UiDefinition,
            source_package: None,
            expected_diagnostics: Vec::new(),
            actual_diagnostics: Vec::new(),
            activation_impact: UiPersistenceActivationImpact::BlocksActivation,
            suggested_fix: suggested_fix.into(),
        }
    }

    pub(super) fn for_document(mut self, document: &UiPersistenceDocumentDescriptor) -> Self {
        self.document = Some(document.id.clone());
        self.source_location = document.source_location.clone();
        self.source_schema_version = Some(document.schema_version);
        self.source_package = Some(document.source_package.clone());
        self
    }

    pub(super) fn for_migration_report(mut self, report: &UiMigrationReportDescriptor) -> Self {
        self.migration_report = Some(report.id.clone());
        self.document = Some(report.document.clone());
        self.source_location = report.source_location.clone();
        self.source_schema_version = Some(report.source_schema_version);
        self.target_schema_version = Some(report.target_schema_version);
        self.source_package = Some(report.source_package.clone());
        self
    }

    pub(super) fn for_diff(mut self, diff: &UiPersistenceDiffDescriptor) -> Self {
        self.diff = Some(diff.id.clone());
        self.document = Some(diff.document.clone());
        self.source_location = diff.source_location.clone();
        self.source_schema_version = Some(diff.before_schema_version);
        self.target_schema_version = Some(diff.after_schema_version);
        self.source_package = Some(diff.source_package.clone());
        self
    }

    pub(super) fn for_activation_request(
        mut self,
        activation: &UiActivationRequestDescriptor,
    ) -> Self {
        self.activation_request = Some(activation.id.clone());
        self.document = Some(activation.document.clone());
        self.source_location = activation.source_location.clone();
        self.source_package = Some(activation.source_package.clone());
        self.expected_diagnostics = activation.expected_diagnostics.iter().cloned().collect();
        self
    }

    pub(super) fn with_target_profile(
        mut self,
        target_profile: UiPersistenceTargetProfileId,
    ) -> Self {
        self.target_profile = Some(target_profile);
        self
    }

    pub(super) fn with_path(mut self, path: UiPersistenceFieldPath) -> Self {
        self.path = Some(path);
        self
    }

    pub(super) fn with_diagnostic_mismatch(
        mut self,
        expected: Vec<UiPersistenceDiagnosticRef>,
        actual: Vec<UiPersistenceDiagnosticRef>,
    ) -> Self {
        self.expected_diagnostics = expected;
        self.actual_diagnostics = actual;
        self
    }

    pub(super) fn preview_only(mut self) -> Self {
        self.activation_impact = UiPersistenceActivationImpact::PreviewOnly;
        self
    }
}
