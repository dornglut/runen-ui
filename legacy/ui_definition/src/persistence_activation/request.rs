//! Persistence activation request declarations and validation.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::UiSourceLocation;

use super::{
    UiMigrationReportDescriptor, UiPersistenceActivationRequestId, UiPersistenceDiagnostic,
    UiPersistenceDiagnosticRef, UiPersistenceDiffDescriptor, UiPersistenceDiffId,
    UiPersistenceDocumentDescriptor, UiPersistenceDocumentId, UiPersistenceMigrationReportId,
    UiPersistenceSourcePackageId, UiPersistenceTargetProfileId,
    validation::PersistenceActivationValidator,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UiPersistenceValidationMode {
    Inspect,
    DryRun,
    Activate,
    RollbackPreflight,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UiUnknownFieldPolicy {
    PreserveCompatible,
    DropCompatible,
    BlockUnknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiActivationRequestDescriptor {
    pub id: UiPersistenceActivationRequestId,
    pub document: UiPersistenceDocumentId,
    #[serde(default)]
    pub target_profiles: BTreeSet<UiPersistenceTargetProfileId>,
    #[serde(default)]
    pub migration_report: Option<UiPersistenceMigrationReportId>,
    #[serde(default)]
    pub diff: Option<UiPersistenceDiffId>,
    #[serde(default)]
    pub expected_diagnostics: BTreeSet<UiPersistenceDiagnosticRef>,
    pub source_package: UiPersistenceSourcePackageId,
    #[serde(default)]
    pub source_location: Option<UiSourceLocation>,
    pub preview_only: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiPersistenceActivationLibrary {
    #[serde(default)]
    pub documents: Vec<UiPersistenceDocumentDescriptor>,
    #[serde(default)]
    pub migration_reports: Vec<UiMigrationReportDescriptor>,
    #[serde(default)]
    pub diffs: Vec<UiPersistenceDiffDescriptor>,
    #[serde(default)]
    pub activation_requests: Vec<UiActivationRequestDescriptor>,
    #[serde(default)]
    pub known_target_profiles: BTreeSet<UiPersistenceTargetProfileId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiPersistenceActivationValidationRequest {
    pub target_profile: UiPersistenceTargetProfileId,
    pub mode: UiPersistenceValidationMode,
    pub unknown_field_policy: UiUnknownFieldPolicy,
    #[serde(default)]
    pub actual_diagnostics: BTreeSet<UiPersistenceDiagnosticRef>,
}

impl UiPersistenceActivationValidationRequest {
    pub fn activate(target_profile: impl Into<UiPersistenceTargetProfileId>) -> Self {
        Self {
            target_profile: target_profile.into(),
            mode: UiPersistenceValidationMode::Activate,
            unknown_field_policy: UiUnknownFieldPolicy::PreserveCompatible,
            actual_diagnostics: BTreeSet::new(),
        }
    }

    pub fn dry_run(target_profile: impl Into<UiPersistenceTargetProfileId>) -> Self {
        Self {
            target_profile: target_profile.into(),
            mode: UiPersistenceValidationMode::DryRun,
            unknown_field_policy: UiUnknownFieldPolicy::PreserveCompatible,
            actual_diagnostics: BTreeSet::new(),
        }
    }

    pub fn with_unknown_field_policy(mut self, policy: UiUnknownFieldPolicy) -> Self {
        self.unknown_field_policy = policy;
        self
    }

    pub fn with_actual_diagnostic(
        mut self,
        diagnostic: impl Into<UiPersistenceDiagnosticRef>,
    ) -> Self {
        self.actual_diagnostics.insert(diagnostic.into());
        self
    }
}

impl PersistenceActivationValidator<'_> {
    pub(super) fn validate_activation_requests(
        &mut self,
        documents: &BTreeMap<UiPersistenceDocumentId, &UiPersistenceDocumentDescriptor>,
        reports: &BTreeMap<UiPersistenceMigrationReportId, &UiMigrationReportDescriptor>,
        diffs: &BTreeMap<UiPersistenceDiffId, &UiPersistenceDiffDescriptor>,
        activations: &BTreeMap<UiPersistenceActivationRequestId, &UiActivationRequestDescriptor>,
    ) {
        for activation in activations.values() {
            let document = match documents.get(&activation.document) {
                Some(document) => *document,
                None => {
                    self.diagnostics.push(
                        UiPersistenceDiagnostic::error(
                            "ui.persistence.activation.document_unknown",
                            format!(
                                "Activation request '{}' references unknown document '{}'.",
                                activation.id, activation.document
                            ),
                            "Create the referenced persistence document descriptor or update the activation request.",
                        )
                        .for_activation_request(activation)
                        .with_target_profile(self.request.target_profile.clone()),
                    );
                    continue;
                }
            };

            self.validate_activation_target_profile(activation);
            self.validate_activation_preview_only(activation);
            self.validate_activation_migration_report(activation, document, reports);
            self.validate_activation_diff(activation, document, diffs);
            self.validate_activation_diagnostic_expectations(activation);
        }
    }
    fn validate_activation_target_profile(&mut self, activation: &UiActivationRequestDescriptor) {
        if !activation.target_profiles.is_empty()
            && !activation
                .target_profiles
                .contains(&self.request.target_profile)
        {
            self.diagnostics.push(
                UiPersistenceDiagnostic::error(
                    "ui.persistence.activation.target_profile_unsupported",
                    format!(
                        "Activation request '{}' does not support target profile '{}'.",
                        activation.id, self.request.target_profile
                    ),
                    "Add the target profile to the activation request or validate with a compatible target profile.",
                )
                .for_activation_request(activation)
                .with_target_profile(self.request.target_profile.clone()),
            );
        }
    }
    fn validate_activation_preview_only(&mut self, activation: &UiActivationRequestDescriptor) {
        if activation.preview_only && self.request.mode == UiPersistenceValidationMode::Activate {
            self.diagnostics.push(
                UiPersistenceDiagnostic::error(
                    "ui.persistence.activation.preview_only_activation",
                    format!(
                        "Activation request '{}' is preview-only and cannot activate.",
                        activation.id
                    ),
                    "Use dry-run validation or remove the preview-only flag before activation.",
                )
                .for_activation_request(activation)
                .with_target_profile(self.request.target_profile.clone())
                .preview_only(),
            );
        }
    }
    fn validate_activation_migration_report(
        &mut self,
        activation: &UiActivationRequestDescriptor,
        document: &UiPersistenceDocumentDescriptor,
        reports: &BTreeMap<UiPersistenceMigrationReportId, &UiMigrationReportDescriptor>,
    ) {
        let Some(report_id) = &activation.migration_report else {
            self.diagnostics.push(
                UiPersistenceDiagnostic::error(
                    "ui.persistence.activation.migration_report_missing",
                    format!(
                        "Activation request '{}' does not reference a migration report.",
                        activation.id
                    ),
                    "Run migration dry-run and attach the report before activation.",
                )
                .for_activation_request(activation)
                .with_target_profile(self.request.target_profile.clone()),
            );
            return;
        };

        let Some(report) = reports.get(report_id) else {
            self.diagnostics.push(
                UiPersistenceDiagnostic::error(
                    "ui.persistence.activation.migration_report_unknown",
                    format!(
                        "Activation request '{}' references unknown migration report '{}'.",
                        activation.id, report_id
                    ),
                    "Create the referenced migration report or update the activation request.",
                )
                .for_activation_request(activation)
                .with_target_profile(self.request.target_profile.clone()),
            );
            return;
        };

        if report.document != document.id {
            self.diagnostics.push(
                UiPersistenceDiagnostic::error(
                    "ui.persistence.activation.migration_report_document_mismatch",
                    format!(
                        "Activation request '{}' references a migration report for a different document.",
                        activation.id
                    ),
                    "Attach a migration report generated for the activation document.",
                )
                .for_activation_request(activation)
                .with_target_profile(self.request.target_profile.clone()),
            );
        }
    }
    fn validate_activation_diff(
        &mut self,
        activation: &UiActivationRequestDescriptor,
        document: &UiPersistenceDocumentDescriptor,
        diffs: &BTreeMap<UiPersistenceDiffId, &UiPersistenceDiffDescriptor>,
    ) {
        let Some(diff_id) = &activation.diff else {
            self.diagnostics.push(
                UiPersistenceDiagnostic::error(
                    "ui.persistence.activation.diff_missing",
                    format!(
                        "Activation request '{}' does not reference a deterministic diff.",
                        activation.id
                    ),
                    "Attach a deterministic diff descriptor before activation.",
                )
                .for_activation_request(activation)
                .with_target_profile(self.request.target_profile.clone()),
            );
            return;
        };

        let Some(diff) = diffs.get(diff_id) else {
            self.diagnostics.push(
                UiPersistenceDiagnostic::error(
                    "ui.persistence.activation.diff_unknown",
                    format!(
                        "Activation request '{}' references unknown diff '{}'.",
                        activation.id, diff_id
                    ),
                    "Create the referenced deterministic diff or update the activation request.",
                )
                .for_activation_request(activation)
                .with_target_profile(self.request.target_profile.clone()),
            );
            return;
        };

        if diff.document != document.id {
            self.diagnostics.push(
                UiPersistenceDiagnostic::error(
                    "ui.persistence.activation.diff_document_mismatch",
                    format!(
                        "Activation request '{}' references a diff for a different document.",
                        activation.id
                    ),
                    "Attach a deterministic diff generated for the activation document.",
                )
                .for_activation_request(activation)
                .with_target_profile(self.request.target_profile.clone()),
            );
        }
    }
    fn validate_activation_diagnostic_expectations(
        &mut self,
        activation: &UiActivationRequestDescriptor,
    ) {
        if activation.expected_diagnostics != self.request.actual_diagnostics {
            self.diagnostics.push(
                UiPersistenceDiagnostic::error(
                    "ui.persistence.activation.expected_diagnostics_mismatch",
                    format!(
                        "Activation request '{}' expected diagnostics do not match actual diagnostics.",
                        activation.id
                    ),
                    "Update expected diagnostics or fix the persistence input that produced different diagnostics.",
                )
                .for_activation_request(activation)
                .with_target_profile(self.request.target_profile.clone())
                .with_diagnostic_mismatch(
                    activation.expected_diagnostics.iter().cloned().collect(),
                    self.request.actual_diagnostics.iter().cloned().collect(),
                ),
            );
        }
    }
}
