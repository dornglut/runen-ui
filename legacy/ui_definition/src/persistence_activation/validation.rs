//! Persistence activation validation orchestration.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::UiDefinitionDiagnosticSeverity;

use super::{
    UiActivationRequestDescriptor, UiMigrationReportDescriptor, UiPersistenceActivationLibrary,
    UiPersistenceActivationRequestId, UiPersistenceActivationValidationRequest,
    UiPersistenceDiagnostic, UiPersistenceDiffDescriptor, UiPersistenceDiffId,
    UiPersistenceDocumentDescriptor, UiPersistenceDocumentId, UiPersistenceMigrationReportId,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiPersistenceActivationValidationReport {
    #[serde(default)]
    pub diagnostics: Vec<UiPersistenceDiagnostic>,
}

impl UiPersistenceActivationValidationReport {
    pub fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity == UiDefinitionDiagnosticSeverity::Error)
    }
}

pub fn validate_persistence_activation(
    library: &UiPersistenceActivationLibrary,
    request: &UiPersistenceActivationValidationRequest,
) -> UiPersistenceActivationValidationReport {
    let mut diagnostics = Vec::new();
    let documents = index_documents(library, request, &mut diagnostics);
    let migration_reports = index_migration_reports(library, request, &mut diagnostics);
    let diffs = index_diffs(library, request, &mut diagnostics);
    let activation_requests = index_activation_requests(library, request, &mut diagnostics);

    let mut validator = PersistenceActivationValidator {
        request,
        diagnostics,
    };

    validator.validate_documents(&documents);
    validator.validate_migration_reports(&documents, &migration_reports);
    validator.validate_diffs(&documents, &diffs);
    validator.validate_activation_requests(
        &documents,
        &migration_reports,
        &diffs,
        &activation_requests,
    );

    UiPersistenceActivationValidationReport {
        diagnostics: validator.diagnostics,
    }
}

pub(super) fn index_documents<'a>(
    library: &'a UiPersistenceActivationLibrary,
    request: &UiPersistenceActivationValidationRequest,
    diagnostics: &mut Vec<UiPersistenceDiagnostic>,
) -> BTreeMap<UiPersistenceDocumentId, &'a UiPersistenceDocumentDescriptor> {
    let mut documents = BTreeMap::new();
    for document in &library.documents {
        if documents.insert(document.id.clone(), document).is_some() {
            diagnostics.push(
                UiPersistenceDiagnostic::error(
                    "ui.persistence.document.duplicate_id",
                    format!(
                        "Persistence document '{}' is declared more than once.",
                        document.id
                    ),
                    "Keep one persistence document descriptor for each stable document id.",
                )
                .for_document(document)
                .with_target_profile(request.target_profile.clone()),
            );
        }
    }
    documents
}

pub(super) fn index_migration_reports<'a>(
    library: &'a UiPersistenceActivationLibrary,
    request: &UiPersistenceActivationValidationRequest,
    diagnostics: &mut Vec<UiPersistenceDiagnostic>,
) -> BTreeMap<UiPersistenceMigrationReportId, &'a UiMigrationReportDescriptor> {
    let mut reports = BTreeMap::new();
    for report in &library.migration_reports {
        if reports.insert(report.id.clone(), report).is_some() {
            diagnostics.push(
                UiPersistenceDiagnostic::error(
                    "ui.persistence.migration_report.duplicate_id",
                    format!(
                        "Migration report '{}' is declared more than once.",
                        report.id
                    ),
                    "Keep one migration report descriptor for each stable report id.",
                )
                .for_migration_report(report)
                .with_target_profile(request.target_profile.clone()),
            );
        }
    }
    reports
}

pub(super) fn index_diffs<'a>(
    library: &'a UiPersistenceActivationLibrary,
    request: &UiPersistenceActivationValidationRequest,
    diagnostics: &mut Vec<UiPersistenceDiagnostic>,
) -> BTreeMap<UiPersistenceDiffId, &'a UiPersistenceDiffDescriptor> {
    let mut diffs = BTreeMap::new();
    for diff in &library.diffs {
        if diffs.insert(diff.id.clone(), diff).is_some() {
            diagnostics.push(
                UiPersistenceDiagnostic::error(
                    "ui.persistence.diff.duplicate_id",
                    format!("Persistence diff '{}' is declared more than once.", diff.id),
                    "Keep one diff descriptor for each stable diff id.",
                )
                .for_diff(diff)
                .with_target_profile(request.target_profile.clone()),
            );
        }
    }
    diffs
}

pub(super) fn index_activation_requests<'a>(
    library: &'a UiPersistenceActivationLibrary,
    request: &UiPersistenceActivationValidationRequest,
    diagnostics: &mut Vec<UiPersistenceDiagnostic>,
) -> BTreeMap<UiPersistenceActivationRequestId, &'a UiActivationRequestDescriptor> {
    let mut activations = BTreeMap::new();
    for activation in &library.activation_requests {
        if activations
            .insert(activation.id.clone(), activation)
            .is_some()
        {
            diagnostics.push(
                UiPersistenceDiagnostic::error(
                    "ui.persistence.activation.duplicate_id",
                    format!(
                        "Activation request '{}' is declared more than once.",
                        activation.id
                    ),
                    "Keep one activation request descriptor for each stable activation request id.",
                )
                .for_activation_request(activation)
                .with_target_profile(request.target_profile.clone()),
            );
        }
    }
    activations
}

pub(super) struct PersistenceActivationValidator<'a> {
    pub(super) request: &'a UiPersistenceActivationValidationRequest,
    pub(super) diagnostics: Vec<UiPersistenceDiagnostic>,
}
