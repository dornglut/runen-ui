//! Persistence migration dry-run declarations and validation.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::{CURRENT_UI_DEFINITION_SCHEMA_VERSION, UiSourceLocation};

use super::{
    UiPersistenceDiagnostic, UiPersistenceDocumentDescriptor, UiPersistenceDocumentId,
    UiPersistenceFieldPath, UiPersistenceMigrationReportId, UiPersistenceSourcePackageId,
    UiPersistenceValidationMode, UiUnknownFieldPolicy, validation::PersistenceActivationValidator,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiMigrationReportDescriptor {
    pub id: UiPersistenceMigrationReportId,
    pub document: UiPersistenceDocumentId,
    pub source_schema_version: u32,
    pub target_schema_version: u32,
    #[serde(default)]
    pub changed_paths: BTreeSet<UiPersistenceFieldPath>,
    #[serde(default)]
    pub incompatible_paths: BTreeSet<UiPersistenceFieldPath>,
    #[serde(default)]
    pub preserved_unknown_fields: BTreeSet<UiPersistenceFieldPath>,
    #[serde(default)]
    pub dropped_unknown_fields: BTreeSet<UiPersistenceFieldPath>,
    #[serde(default)]
    pub deterministic_preview: Option<String>,
    pub source_package: UiPersistenceSourcePackageId,
    #[serde(default)]
    pub source_location: Option<UiSourceLocation>,
    pub preview_only: bool,
}

impl PersistenceActivationValidator<'_> {
    pub(super) fn validate_migration_reports(
        &mut self,
        documents: &BTreeMap<UiPersistenceDocumentId, &UiPersistenceDocumentDescriptor>,
        reports: &BTreeMap<UiPersistenceMigrationReportId, &UiMigrationReportDescriptor>,
    ) {
        for report in reports.values() {
            let document = match documents.get(&report.document) {
                Some(document) => *document,
                None => {
                    self.diagnostics.push(
                        UiPersistenceDiagnostic::error(
                            "ui.persistence.migration_report.document_unknown",
                            format!(
                                "Migration report '{}' references unknown document '{}'.",
                                report.id, report.document
                            ),
                            "Create the referenced persistence document descriptor or update the migration report reference.",
                        )
                        .for_migration_report(report)
                        .with_target_profile(self.request.target_profile.clone()),
                    );
                    continue;
                }
            };

            self.validate_migration_schema_match(report, document);
            self.validate_migration_target_schema(report);
            self.validate_incompatible_migration_paths(report);
            self.validate_migration_unknown_field_policy(report, document);
            self.validate_migration_preview_only(report);
        }
    }
    fn validate_migration_schema_match(
        &mut self,
        report: &UiMigrationReportDescriptor,
        document: &UiPersistenceDocumentDescriptor,
    ) {
        if report.source_schema_version != document.schema_version {
            self.diagnostics.push(
                UiPersistenceDiagnostic::error(
                    "ui.persistence.migration_report.schema_mismatch",
                    format!(
                        "Migration report '{}' source schema version does not match document '{}'.",
                        report.id, document.id
                    ),
                    "Regenerate the migration report from the current persisted document.",
                )
                .for_migration_report(report)
                .with_target_profile(self.request.target_profile.clone()),
            );
        }
    }
    fn validate_migration_target_schema(&mut self, report: &UiMigrationReportDescriptor) {
        if report.target_schema_version != CURRENT_UI_DEFINITION_SCHEMA_VERSION {
            self.diagnostics.push(
                UiPersistenceDiagnostic::error(
                    "ui.persistence.migration_report.target_version_unsupported",
                    format!(
                        "Migration report '{}' targets unsupported schema version '{}'.",
                        report.id, report.target_schema_version
                    ),
                    "Target the current UI definition schema version before activation.",
                )
                .for_migration_report(report)
                .with_target_profile(self.request.target_profile.clone()),
            );
        }
    }
    fn validate_incompatible_migration_paths(&mut self, report: &UiMigrationReportDescriptor) {
        for path in &report.incompatible_paths {
            self.diagnostics.push(
                UiPersistenceDiagnostic::error(
                    "ui.persistence.migration.incompatible_path",
                    format!(
                        "Migration report '{}' contains an incompatible field path.",
                        report.id
                    ),
                    "Resolve the incompatible migration before activation.",
                )
                .for_migration_report(report)
                .with_path(path.clone())
                .with_target_profile(self.request.target_profile.clone()),
            );
        }
    }
    fn validate_migration_unknown_field_policy(
        &mut self,
        report: &UiMigrationReportDescriptor,
        document: &UiPersistenceDocumentDescriptor,
    ) {
        if self.request.unknown_field_policy == UiUnknownFieldPolicy::PreserveCompatible {
            for path in document
                .compatible_unknown_fields
                .difference(&report.preserved_unknown_fields)
            {
                self.diagnostics.push(
                    UiPersistenceDiagnostic::error(
                        "ui.persistence.unknown_field.not_preserved",
                        format!(
                            "Migration report '{}' does not preserve a compatible unknown field.",
                            report.id
                        ),
                        "Preserve compatible unknown fields or change policy before activation.",
                    )
                    .for_migration_report(report)
                    .with_path(path.clone())
                    .with_target_profile(self.request.target_profile.clone()),
                );
            }
        }

        if self.request.unknown_field_policy == UiUnknownFieldPolicy::PreserveCompatible {
            for path in &report.dropped_unknown_fields {
                self.diagnostics.push(
                    UiPersistenceDiagnostic::error(
                        "ui.persistence.unknown_field.dropped",
                        format!(
                            "Migration report '{}' drops an unknown field while preserve-compatible policy is active.",
                            report.id
                        ),
                        "Preserve the field or mark it unpreservable with a blocking diagnostic.",
                    )
                    .for_migration_report(report)
                    .with_path(path.clone())
                    .with_target_profile(self.request.target_profile.clone()),
                );
            }
        }
    }
    fn validate_migration_preview_only(&mut self, report: &UiMigrationReportDescriptor) {
        if report.preview_only && self.request.mode == UiPersistenceValidationMode::Activate {
            self.diagnostics.push(
                UiPersistenceDiagnostic::error(
                    "ui.persistence.migration_report.preview_only_activation",
                    format!(
                        "Migration report '{}' is preview-only and cannot activate.",
                        report.id
                    ),
                    "Use dry-run validation or regenerate an activation-capable migration report.",
                )
                .for_migration_report(report)
                .with_target_profile(self.request.target_profile.clone())
                .preview_only(),
            );
        }
    }
}
