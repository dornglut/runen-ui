use std::collections::BTreeSet;
use ui_definition::prelude::*;

fn main() {
    let template = authored_template();

    let diagnostics = validate_ui_template(&template);
    assert!(diagnostics.is_empty(), "{diagnostics:?}");

    let normalized = normalize_ui_template(template.clone());
    assert!(!normalized.has_errors(), "{:?}", normalized.diagnostics);

    let context =
        UiVisualLayoutEditContext::with_supported_target_profiles(["editor.workbench".into()]);
    let edit = apply_ui_layout_operation(template, &axis_operation(), &context);
    assert!(!edit.has_errors(), "{:?}", edit.diagnostics);
    assert!(edit.diff.is_some());

    let preview = validate_ui_preview_library(
        &preview_library(),
        &UiPreviewValidationRequest::preview("editor.workbench")
            .with_data_package("preview.data")
            .with_actual_diagnostic("diag.expected"),
    );
    assert!(!preview.has_errors(), "{:?}", preview.diagnostics);

    let persistence = validate_ui_persistence_flow(
        &persistence_library(),
        &UiPersistenceActivationValidationRequest::activate("editor.workbench"),
    );
    assert!(!persistence.has_errors(), "{:?}", persistence.diagnostics);

    let readiness = validate_ui_readiness(
        &readiness_library(),
        &UiReadinessValidationRequest::production("editor.workbench"),
    );
    assert!(!readiness.has_errors(), "{:?}", readiness.diagnostics);
}

fn authored_template() -> AuthoredUiTemplate {
    AuthoredUiTemplate {
        id: "example.template".into(),
        root: UiNodeDefinition::Column {
            id: "root".into(),
            children: vec![UiNodeDefinition::Stack {
                id: "stack".into(),
                axis: UiAxisDefinition::Vertical,
                children: vec![UiNodeDefinition::Label {
                    id: "title".into(),
                    label: UiValueBinding::static_text("Editor Lab"),
                    availability: None,
                }],
            }],
        },
        templates: Vec::new(),
        menus: Vec::new(),
    }
}

fn axis_operation() -> UiVisualLayoutOperation {
    UiVisualLayoutOperation {
        id: "axis.stack".into(),
        source_document: "example.template".into(),
        target_path: AuthoredUiNodePath("root/stack".to_string()),
        expected_node_id: "stack".into(),
        target_profile: "editor.workbench".into(),
        kind: UiVisualLayoutEditKind::ChangeStackAxis {
            axis: UiAxisDefinition::Horizontal,
        },
        source_location: None,
        preview_only: false,
    }
}

fn preview_library() -> UiPreviewLibrary {
    UiPreviewLibrary {
        fixtures: vec![
            fixture("empty", UiPreviewDataStateKind::Empty),
            fixture("loading", UiPreviewDataStateKind::Loading),
            fixture("error", UiPreviewDataStateKind::Error),
            fixture("denied", UiPreviewDataStateKind::Denied),
            fixture("offline", UiPreviewDataStateKind::Offline),
            fixture("heavy", UiPreviewDataStateKind::Heavy),
            fixture("accessibility", UiPreviewDataStateKind::Accessibility),
        ],
        scenarios: vec![UiPreviewScenarioDeclaration {
            id: id("open-editor-lab"),
            fixture: id("empty"),
            target_profiles: ids(&["editor.workbench"]),
            steps: vec![UiPreviewScenarioStep {
                id: id("select-title"),
                kind: UiPreviewScenarioStepKind::Intent,
                target_node: Some(id("title")),
                expected_state: Some(id("state.ready")),
            }],
            required_capabilities: ids(&["preview.run"]),
            expected_diagnostics: ids(&["diag.expected"]),
            source_package: id("ui.example"),
            source_location: None,
            preview_only: false,
        }],
        matrices: vec![UiPreviewMatrixDeclaration {
            id: id("desktop"),
            fixtures: vec![id("empty")],
            scenarios: vec![id("open-editor-lab")],
            target_profiles: ids(&["editor.workbench"]),
            axes: vec![UiPreviewMatrixAxis {
                kind: UiPreviewMatrixAxisKind::Platform,
                value: "desktop".to_string(),
            }],
            evidence: vec![UiPreviewEvidenceDescriptor {
                id: id("retained-debug"),
                target_profiles: ids(&["editor.workbench"]),
                expected_diagnostics: ids(&["diag.expected"]),
                expected_states: ids(&["state.ready"]),
            }],
            validation_mode: ui_definition::UiPreviewValidationMode::Preview,
            source_package: id("ui.example"),
            source_location: None,
            preview_only: false,
        }],
        known_data_packages: ids(&["preview.data"]),
        known_capabilities: ids(&["preview.run"]),
    }
}

fn fixture(fixture_id: &str, state: UiPreviewDataStateKind) -> UiPreviewFixtureDeclaration {
    UiPreviewFixtureDeclaration {
        id: id(fixture_id),
        data_state: state,
        target_profiles: ids(&["editor.workbench"]),
        required_data_packages: ids(&["preview.data"]),
        required_capabilities: ids(&["preview.run"]),
        expected_diagnostics: ids(&["diag.expected"]),
        expected_states: ids(&["state.ready"]),
        source_package: id("ui.example"),
        source_location: None,
        preview_only: false,
    }
}

fn persistence_library() -> UiPersistenceActivationLibrary {
    UiPersistenceActivationLibrary {
        documents: vec![UiPersistenceDocumentDescriptor {
            id: id("editor.lab"),
            schema_version: CURRENT_UI_DEFINITION_SCHEMA_VERSION,
            category: AuthoredUiDefinitionCategory::Editor,
            target_profiles: ids(&["editor.workbench"]),
            compatible_unknown_fields: BTreeSet::new(),
            required_unknown_fields: BTreeSet::new(),
            unpreservable_unknown_fields: BTreeSet::new(),
            source_package: id("ui.example"),
            source_location: None,
            preview_only: false,
        }],
        migration_reports: vec![UiMigrationReportDescriptor {
            id: id("editor.lab.migration"),
            document: id("editor.lab"),
            source_schema_version: CURRENT_UI_DEFINITION_SCHEMA_VERSION,
            target_schema_version: CURRENT_UI_DEFINITION_SCHEMA_VERSION,
            changed_paths: BTreeSet::from([AuthoredUiNodePath("root/title".to_string())]),
            incompatible_paths: BTreeSet::new(),
            preserved_unknown_fields: BTreeSet::new(),
            dropped_unknown_fields: BTreeSet::new(),
            deterministic_preview: Some("schema_version = 1".to_string()),
            source_package: id("ui.example"),
            source_location: None,
            preview_only: false,
        }],
        diffs: vec![UiPersistenceDiffDescriptor {
            id: id("editor.lab.diff"),
            document: id("editor.lab"),
            before_schema_version: CURRENT_UI_DEFINITION_SCHEMA_VERSION,
            after_schema_version: CURRENT_UI_DEFINITION_SCHEMA_VERSION,
            target_profiles: ids(&["editor.workbench"]),
            changes: vec![UiPersistenceDiffChange {
                kind: UiPersistenceDiffChangeKind::Update,
                path: AuthoredUiNodePath("root/title".to_string()),
                before: Some("Old".to_string()),
                after: Some("New".to_string()),
            }],
            deterministic_text: Some("- Old\n+ New\n".to_string()),
            source_package: id("ui.example"),
            source_location: None,
            preview_only: false,
        }],
        activation_requests: vec![UiActivationRequestDescriptor {
            id: id("editor.lab.activate"),
            document: id("editor.lab"),
            target_profiles: ids(&["editor.workbench"]),
            migration_report: Some(id("editor.lab.migration")),
            diff: Some(id("editor.lab.diff")),
            expected_diagnostics: BTreeSet::new(),
            source_package: id("ui.example"),
            source_location: None,
            preview_only: false,
        }],
        known_target_profiles: ids(&["editor.workbench"]),
    }
}

fn readiness_library() -> UiReadinessLibrary {
    let required = BTreeSet::from([
        UiReadinessEvidenceKind::ProjectionSnapshot,
        UiReadinessEvidenceKind::DiagnosticInspection,
        UiReadinessEvidenceKind::AccessibilityReport,
        UiReadinessEvidenceKind::CompatibilityReport,
        UiReadinessEvidenceKind::PerformanceBudgetReport,
        UiReadinessEvidenceKind::GoldenArtifact,
        UiReadinessEvidenceKind::ExampleScenario,
    ]);
    UiReadinessLibrary {
        evidence_packets: vec![UiReadinessEvidencePacket {
            id: id("editor.lab.ready"),
            document: id("editor.lab"),
            target_profiles: ids(&["editor.workbench"]),
            compatibility_axes: BTreeSet::new(),
            required_evidence: required.clone(),
            artifacts: required
                .iter()
                .enumerate()
                .map(|(index, kind)| UiReadinessEvidenceArtifact {
                    id: id(&format!("artifact.{index}")),
                    kind: *kind,
                    target_profiles: ids(&["editor.workbench"]),
                    freshness: UiReadinessArtifactFreshness::Fresh,
                    ownership: UiReadinessArtifactOwnership::ExternalReference,
                })
                .collect(),
            expected_diagnostics: BTreeSet::new(),
            source_package: id("ui.example"),
            source_location: None,
            preview_only: false,
        }],
        inspection_reports: vec![UiReadinessInspectionReport {
            id: id("editor.lab.inspection"),
            target_profiles: ids(&["editor.workbench"]),
            compatibility_axes: BTreeSet::new(),
            diagnostic_groups: BTreeSet::from([UiReadinessDiagnosticGroup::Composition]),
            source_package: id("ui.example"),
            source_location: None,
            preview_only: false,
        }],
        readiness_requests: vec![UiReadinessRequest {
            id: id("editor.lab.production"),
            evidence_packet: id("editor.lab.ready"),
            inspection_report: Some(id("editor.lab.inspection")),
            target_profiles: ids(&["editor.workbench"]),
            required_evidence: required,
            expected_diagnostics: BTreeSet::new(),
            source_package: id("ui.example"),
            source_location: None,
            preview_only: false,
        }],
    }
}

fn ids(values: &[&str]) -> BTreeSet<AuthoredId> {
    values.iter().copied().map(AuthoredId::from).collect()
}

fn id(value: &str) -> AuthoredId {
    AuthoredId::from(value)
}
