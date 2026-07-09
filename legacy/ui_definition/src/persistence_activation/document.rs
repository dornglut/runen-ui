//! Persistence document declarations and document validation.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::{
    AuthoredUiDefinitionCategory, AuthoredUiNodePath, CURRENT_UI_DEFINITION_SCHEMA_VERSION,
    UiSourceLocation, identity::AuthoredId,
};

use super::{
    UiPersistenceDiagnostic, UiPersistenceValidationMode, UiUnknownFieldPolicy,
    validation::PersistenceActivationValidator,
};

pub type UiPersistenceDocumentId = AuthoredId;
pub type UiPersistenceMigrationReportId = AuthoredId;
pub type UiPersistenceDiffId = AuthoredId;
pub type UiPersistenceActivationRequestId = AuthoredId;
pub type UiPersistenceTargetProfileId = AuthoredId;
pub type UiPersistenceSourcePackageId = AuthoredId;
pub type UiPersistenceDiagnosticRef = AuthoredId;
pub type UiPersistenceFieldPath = AuthoredUiNodePath;
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiPersistenceDocumentDescriptor {
    pub id: UiPersistenceDocumentId,
    pub schema_version: u32,
    pub category: AuthoredUiDefinitionCategory,
    #[serde(default)]
    pub target_profiles: BTreeSet<UiPersistenceTargetProfileId>,
    #[serde(default)]
    pub compatible_unknown_fields: BTreeSet<UiPersistenceFieldPath>,
    #[serde(default)]
    pub required_unknown_fields: BTreeSet<UiPersistenceFieldPath>,
    #[serde(default)]
    pub unpreservable_unknown_fields: BTreeSet<UiPersistenceFieldPath>,
    pub source_package: UiPersistenceSourcePackageId,
    #[serde(default)]
    pub source_location: Option<UiSourceLocation>,
    pub preview_only: bool,
}

impl PersistenceActivationValidator<'_> {
    pub(super) fn validate_documents(
        &mut self,
        documents: &BTreeMap<UiPersistenceDocumentId, &UiPersistenceDocumentDescriptor>,
    ) {
        for document in documents.values() {
            self.validate_document_target_profile(document);
            self.validate_document_schema_version(document);
            self.validate_unknown_required_fields(document);
            self.validate_unpreservable_unknown_fields(document);
            self.validate_unknown_field_policy(document);
            self.validate_preview_only_document(document);
        }
    }
    fn validate_document_target_profile(&mut self, document: &UiPersistenceDocumentDescriptor) {
        if !document.target_profiles.is_empty()
            && !document
                .target_profiles
                .contains(&self.request.target_profile)
        {
            self.diagnostics.push(
                UiPersistenceDiagnostic::error(
                    "ui.persistence.document.target_profile_unsupported",
                    format!(
                        "Persistence document '{}' does not support target profile '{}'.",
                        document.id, self.request.target_profile
                    ),
                    "Add the target profile to the document descriptor or validate with a compatible target profile.",
                )
                .for_document(document)
                .with_target_profile(self.request.target_profile.clone()),
            );
        }
    }
    fn validate_document_schema_version(&mut self, document: &UiPersistenceDocumentDescriptor) {
        if document.schema_version > CURRENT_UI_DEFINITION_SCHEMA_VERSION {
            self.diagnostics.push(
                UiPersistenceDiagnostic::error(
                    "ui.persistence.schema.unsupported_version",
                    format!(
                        "Persistence document '{}' uses unsupported schema version '{}'.",
                        document.id, document.schema_version
                    ),
                    "Run a supported migration path before activation.",
                )
                .for_document(document)
                .with_target_profile(self.request.target_profile.clone()),
            );
        }
    }
    fn validate_unknown_required_fields(&mut self, document: &UiPersistenceDocumentDescriptor) {
        for path in &document.required_unknown_fields {
            self.diagnostics.push(
                UiPersistenceDiagnostic::error(
                    "ui.persistence.unknown_required_field",
                    format!(
                        "Persistence document '{}' contains an unknown required field.",
                        document.id
                    ),
                    "Add a schema migration that understands the required field before activation.",
                )
                .for_document(document)
                .with_path(path.clone())
                .with_target_profile(self.request.target_profile.clone()),
            );
        }
    }
    fn validate_unpreservable_unknown_fields(
        &mut self,
        document: &UiPersistenceDocumentDescriptor,
    ) {
        for path in &document.unpreservable_unknown_fields {
            self.diagnostics.push(
                UiPersistenceDiagnostic::error(
                    "ui.persistence.unknown_field.unpreservable",
                    format!(
                        "Persistence document '{}' contains an unknown field that cannot be preserved.",
                        document.id
                    ),
                    "Reject activation or add an owning migration for the unknown field.",
                )
                .for_document(document)
                .with_path(path.clone())
                .with_target_profile(self.request.target_profile.clone()),
            );
        }
    }
    fn validate_unknown_field_policy(&mut self, document: &UiPersistenceDocumentDescriptor) {
        if self.request.unknown_field_policy == UiUnknownFieldPolicy::BlockUnknown
            && !document.compatible_unknown_fields.is_empty()
        {
            for path in &document.compatible_unknown_fields {
                self.diagnostics.push(
                    UiPersistenceDiagnostic::error(
                        "ui.persistence.unknown_field.blocked_by_policy",
                        format!(
                            "Persistence document '{}' contains an unknown field while unknown fields are blocked.",
                            document.id
                        ),
                        "Use preserve-compatible policy or remove the unknown field before activation.",
                    )
                    .for_document(document)
                    .with_path(path.clone())
                    .with_target_profile(self.request.target_profile.clone()),
                );
            }
        }
    }
    fn validate_preview_only_document(&mut self, document: &UiPersistenceDocumentDescriptor) {
        if document.preview_only && self.request.mode == UiPersistenceValidationMode::Activate {
            self.diagnostics.push(
                UiPersistenceDiagnostic::error(
                    "ui.persistence.document.preview_only_activation",
                    format!(
                        "Persistence document '{}' is preview-only and cannot activate.",
                        document.id
                    ),
                    "Use dry-run validation or remove the preview-only flag before activation.",
                )
                .for_document(document)
                .with_target_profile(self.request.target_profile.clone())
                .preview_only(),
            );
        }
    }
}
