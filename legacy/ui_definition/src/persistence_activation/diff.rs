//! Persistence diff declarations and validation.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::{CURRENT_UI_DEFINITION_SCHEMA_VERSION, UiSourceLocation};

use super::{
    UiPersistenceDiagnostic, UiPersistenceDiffId, UiPersistenceDocumentDescriptor,
    UiPersistenceDocumentId, UiPersistenceFieldPath, UiPersistenceSourcePackageId,
    UiPersistenceTargetProfileId, UiPersistenceValidationMode,
    validation::PersistenceActivationValidator,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UiPersistenceDiffChangeKind {
    Insert,
    Remove,
    Move,
    Update,
    PreserveUnknown,
    DropUnknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiPersistenceDiffChange {
    pub kind: UiPersistenceDiffChangeKind,
    pub path: UiPersistenceFieldPath,
    #[serde(default)]
    pub before: Option<String>,
    #[serde(default)]
    pub after: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiPersistenceDiffDescriptor {
    pub id: UiPersistenceDiffId,
    pub document: UiPersistenceDocumentId,
    pub before_schema_version: u32,
    pub after_schema_version: u32,
    #[serde(default)]
    pub target_profiles: BTreeSet<UiPersistenceTargetProfileId>,
    #[serde(default)]
    pub changes: Vec<UiPersistenceDiffChange>,
    #[serde(default)]
    pub deterministic_text: Option<String>,
    pub source_package: UiPersistenceSourcePackageId,
    #[serde(default)]
    pub source_location: Option<UiSourceLocation>,
    pub preview_only: bool,
}

impl PersistenceActivationValidator<'_> {
    pub(super) fn validate_diffs(
        &mut self,
        documents: &BTreeMap<UiPersistenceDocumentId, &UiPersistenceDocumentDescriptor>,
        diffs: &BTreeMap<UiPersistenceDiffId, &UiPersistenceDiffDescriptor>,
    ) {
        for diff in diffs.values() {
            if !documents.contains_key(&diff.document) {
                self.diagnostics.push(
                    UiPersistenceDiagnostic::error(
                        "ui.persistence.diff.document_unknown",
                        format!(
                            "Persistence diff '{}' references unknown document '{}'.",
                            diff.id, diff.document
                        ),
                        "Create the referenced persistence document descriptor or update the diff reference.",
                    )
                    .for_diff(diff)
                    .with_target_profile(self.request.target_profile.clone()),
                );
                continue;
            }

            self.validate_diff_target_profile(diff);
            self.validate_diff_target_schema(diff);
            self.validate_deterministic_diff_text(diff);
            self.validate_diff_preview_only(diff);
        }
    }
    fn validate_diff_target_profile(&mut self, diff: &UiPersistenceDiffDescriptor) {
        if !diff.target_profiles.is_empty()
            && !diff.target_profiles.contains(&self.request.target_profile)
        {
            self.diagnostics.push(
                UiPersistenceDiagnostic::error(
                    "ui.persistence.diff.target_profile_unsupported",
                    format!(
                        "Persistence diff '{}' does not support target profile '{}'.",
                        diff.id, self.request.target_profile
                    ),
                    "Add the target profile to the diff descriptor or validate with a compatible target profile.",
                )
                .for_diff(diff)
                .with_target_profile(self.request.target_profile.clone()),
            );
        }
    }
    fn validate_diff_target_schema(&mut self, diff: &UiPersistenceDiffDescriptor) {
        if diff.after_schema_version != CURRENT_UI_DEFINITION_SCHEMA_VERSION {
            self.diagnostics.push(
                UiPersistenceDiagnostic::error(
                    "ui.persistence.diff.target_version_unsupported",
                    format!(
                        "Persistence diff '{}' targets unsupported schema version '{}'.",
                        diff.id, diff.after_schema_version
                    ),
                    "Regenerate the diff against the current UI definition schema version.",
                )
                .for_diff(diff)
                .with_target_profile(self.request.target_profile.clone()),
            );
        }
    }
    fn validate_deterministic_diff_text(&mut self, diff: &UiPersistenceDiffDescriptor) {
        if diff
            .deterministic_text
            .as_ref()
            .is_none_or(|text| text.is_empty())
        {
            self.diagnostics.push(
                UiPersistenceDiagnostic::error(
                    "ui.persistence.diff.non_deterministic",
                    format!("Persistence diff '{}' does not provide deterministic diff text.", diff.id),
                    "Serialize the migration or edit into deterministic textual diff output before activation.",
                )
                .for_diff(diff)
                .with_target_profile(self.request.target_profile.clone()),
            );
        }
    }
    fn validate_diff_preview_only(&mut self, diff: &UiPersistenceDiffDescriptor) {
        if diff.preview_only && self.request.mode == UiPersistenceValidationMode::Activate {
            self.diagnostics.push(
                UiPersistenceDiagnostic::error(
                    "ui.persistence.diff.preview_only_activation",
                    format!(
                        "Persistence diff '{}' is preview-only and cannot activate.",
                        diff.id
                    ),
                    "Use dry-run validation or produce an activation-capable deterministic diff.",
                )
                .for_diff(diff)
                .with_target_profile(self.request.target_profile.clone())
                .preview_only(),
            );
        }
    }
}
