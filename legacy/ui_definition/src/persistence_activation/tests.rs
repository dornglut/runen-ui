//! Persistence activation validation tests.

use super::*;
use crate::{
    AuthoredUiDefinitionCategory, AuthoredUiNodePath, CURRENT_UI_DEFINITION_SCHEMA_VERSION,
    identity::AuthoredId,
};
use std::collections::BTreeSet;

fn id(value: &str) -> AuthoredId {
    AuthoredId::from(value)
}

fn path(value: &str) -> AuthoredUiNodePath {
    AuthoredUiNodePath(value.to_string())
}

fn target_profiles(values: &[&str]) -> BTreeSet<UiPersistenceTargetProfileId> {
    values.iter().map(|value| id(value)).collect()
}

fn source_package() -> UiPersistenceSourcePackageId {
    id("ui.package.core")
}

fn document(
    id_value: &str,
    category: AuthoredUiDefinitionCategory,
) -> UiPersistenceDocumentDescriptor {
    UiPersistenceDocumentDescriptor {
        id: id(id_value),
        schema_version: CURRENT_UI_DEFINITION_SCHEMA_VERSION,
        category,
        target_profiles: target_profiles(&["editor.workbench", "game.runtime"]),
        compatible_unknown_fields: BTreeSet::new(),
        required_unknown_fields: BTreeSet::new(),
        unpreservable_unknown_fields: BTreeSet::new(),
        source_package: source_package(),
        source_location: None,
        preview_only: false,
    }
}

fn migration_report(document_id: &str) -> UiMigrationReportDescriptor {
    UiMigrationReportDescriptor {
        id: id(&format!("{document_id}.migration")),
        document: id(document_id),
        source_schema_version: CURRENT_UI_DEFINITION_SCHEMA_VERSION,
        target_schema_version: CURRENT_UI_DEFINITION_SCHEMA_VERSION,
        changed_paths: BTreeSet::from([path("root/title")]),
        incompatible_paths: BTreeSet::new(),
        preserved_unknown_fields: BTreeSet::new(),
        dropped_unknown_fields: BTreeSet::new(),
        deterministic_preview: Some("schema_version = 1".to_string()),
        source_package: source_package(),
        source_location: None,
        preview_only: false,
    }
}

fn diff(document_id: &str) -> UiPersistenceDiffDescriptor {
    UiPersistenceDiffDescriptor {
        id: id(&format!("{document_id}.diff")),
        document: id(document_id),
        before_schema_version: CURRENT_UI_DEFINITION_SCHEMA_VERSION,
        after_schema_version: CURRENT_UI_DEFINITION_SCHEMA_VERSION,
        target_profiles: target_profiles(&["editor.workbench", "game.runtime"]),
        changes: vec![UiPersistenceDiffChange {
            kind: UiPersistenceDiffChangeKind::Update,
            path: path("root/title"),
            before: Some("Old".to_string()),
            after: Some("New".to_string()),
        }],
        deterministic_text: Some("- title: Old\n+ title: New\n".to_string()),
        source_package: source_package(),
        source_location: None,
        preview_only: false,
    }
}

fn activation(document_id: &str) -> UiActivationRequestDescriptor {
    UiActivationRequestDescriptor {
        id: id(&format!("{document_id}.activate")),
        document: id(document_id),
        target_profiles: target_profiles(&["editor.workbench", "game.runtime"]),
        migration_report: Some(id(&format!("{document_id}.migration"))),
        diff: Some(id(&format!("{document_id}.diff"))),
        expected_diagnostics: BTreeSet::new(),
        source_package: source_package(),
        source_location: None,
        preview_only: false,
    }
}

fn library() -> UiPersistenceActivationLibrary {
    UiPersistenceActivationLibrary {
        documents: vec![
            document("editor.inspector", AuthoredUiDefinitionCategory::Editor),
            document("runtime.hud", AuthoredUiDefinitionCategory::GameUi),
        ],
        migration_reports: vec![
            migration_report("editor.inspector"),
            migration_report("runtime.hud"),
        ],
        diffs: vec![diff("editor.inspector"), diff("runtime.hud")],
        activation_requests: vec![activation("editor.inspector"), activation("runtime.hud")],
        known_target_profiles: target_profiles(&["editor.workbench", "game.runtime"]),
    }
}

fn request(target_profile: &str) -> UiPersistenceActivationValidationRequest {
    UiPersistenceActivationValidationRequest::activate(target_profile)
}

fn codes(report: &UiPersistenceActivationValidationReport) -> BTreeSet<&str> {
    report
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.code.as_str())
        .collect()
}

#[test]
fn persistence_activation_validates_editor_and_runtime_examples_without_shared_ownership() {
    let editor = validate_persistence_activation(&library(), &request("editor.workbench"));
    let runtime = validate_persistence_activation(&library(), &request("game.runtime"));

    assert!(
        !editor.has_errors(),
        "editor diagnostics: {:?}",
        editor.diagnostics
    );
    assert!(
        !runtime.has_errors(),
        "runtime diagnostics: {:?}",
        runtime.diagnostics
    );
}

#[test]
fn persistence_activation_rejects_unsupported_schema_version() {
    let mut library = library();
    library.documents[0].schema_version = CURRENT_UI_DEFINITION_SCHEMA_VERSION + 1;

    let report = validate_persistence_activation(&library, &request("editor.workbench"));

    assert!(codes(&report).contains("ui.persistence.schema.unsupported_version"));
}

#[test]
fn persistence_activation_rejects_incompatible_migration() {
    let mut library = library();
    library.migration_reports[0]
        .incompatible_paths
        .insert(path("root/legacy"));

    let report = validate_persistence_activation(&library, &request("editor.workbench"));

    assert!(codes(&report).contains("ui.persistence.migration.incompatible_path"));
}

#[test]
fn persistence_activation_preserves_compatible_unknown_fields() {
    let mut library = library();
    library.documents[0]
        .compatible_unknown_fields
        .insert(path("root/extension"));
    library.migration_reports[0]
        .preserved_unknown_fields
        .insert(path("root/extension"));

    let report = validate_persistence_activation(&library, &request("editor.workbench"));

    assert!(
        !report.has_errors(),
        "diagnostics: {:?}",
        report.diagnostics
    );
}

#[test]
fn persistence_activation_rejects_unpreservable_unknown_fields() {
    let mut library = library();
    library.documents[0]
        .unpreservable_unknown_fields
        .insert(path("root/plugin_state"));

    let report = validate_persistence_activation(&library, &request("editor.workbench"));

    assert!(codes(&report).contains("ui.persistence.unknown_field.unpreservable"));
}

#[test]
fn persistence_activation_rejects_non_deterministic_diff() {
    let mut library = library();
    library.diffs[0].deterministic_text = None;

    let report = validate_persistence_activation(&library, &request("editor.workbench"));

    assert!(codes(&report).contains("ui.persistence.diff.non_deterministic"));
}

#[test]
fn persistence_activation_requires_migration_report_and_diff() {
    let mut library = library();
    library.activation_requests[0].migration_report = None;
    library.activation_requests[0].diff = None;

    let report = validate_persistence_activation(&library, &request("editor.workbench"));
    let codes = codes(&report);

    assert!(codes.contains("ui.persistence.activation.migration_report_missing"));
    assert!(codes.contains("ui.persistence.activation.diff_missing"));
}

#[test]
fn game_runtime_persistence_activation_requires_migration_diff_and_determinism() {
    let mut library = library();
    library.activation_requests[0].migration_report = None;
    library.activation_requests[0].diff = None;
    library.diffs[0].deterministic_text = None;

    let report = validate_persistence_activation(&library, &request("game.runtime"));
    let codes = codes(&report);

    assert!(codes.contains("ui.persistence.activation.migration_report_missing"));
    assert!(codes.contains("ui.persistence.activation.diff_missing"));
    assert!(codes.contains("ui.persistence.diff.non_deterministic"));
}

#[test]
fn persistence_activation_rejects_unsupported_target_profile() {
    let report = validate_persistence_activation(&library(), &request("console.runtime"));
    let codes = codes(&report);

    assert!(codes.contains("ui.persistence.document.target_profile_unsupported"));
    assert!(codes.contains("ui.persistence.diff.target_profile_unsupported"));
    assert!(codes.contains("ui.persistence.activation.target_profile_unsupported"));
}

#[test]
fn persistence_activation_rejects_expected_diagnostic_mismatches() {
    let mut library = library();
    library.activation_requests[0]
        .expected_diagnostics
        .insert(id("ui.persistence.expected"));

    let report = validate_persistence_activation(
        &library,
        &UiPersistenceActivationValidationRequest::activate("editor.workbench")
            .with_actual_diagnostic("ui.persistence.actual"),
    );

    assert!(codes(&report).contains("ui.persistence.activation.expected_diagnostics_mismatch"));
}

#[test]
fn persistence_activation_rejects_preview_only_activation() {
    let mut library = library();
    library.documents[0].preview_only = true;
    library.migration_reports[0].preview_only = true;
    library.diffs[0].preview_only = true;
    library.activation_requests[0].preview_only = true;

    let report = validate_persistence_activation(&library, &request("editor.workbench"));
    let codes = codes(&report);

    assert!(codes.contains("ui.persistence.document.preview_only_activation"));
    assert!(codes.contains("ui.persistence.migration_report.preview_only_activation"));
    assert!(codes.contains("ui.persistence.diff.preview_only_activation"));
    assert!(codes.contains("ui.persistence.activation.preview_only_activation"));
}
