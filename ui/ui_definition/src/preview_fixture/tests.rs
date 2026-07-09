//! Preview fixture validation tests.

use super::*;
use crate::identity::AuthoredId;
use std::collections::BTreeSet;

fn id(value: &str) -> AuthoredId {
    AuthoredId::from(value)
}

fn ids(values: &[&str]) -> BTreeSet<AuthoredId> {
    values.iter().copied().map(AuthoredId::from).collect()
}

fn profiles(values: &[&str]) -> BTreeSet<UiPreviewTargetProfileId> {
    ids(values)
}

fn fixture(fixture_id: &str, state: UiPreviewDataStateKind) -> UiPreviewFixtureDeclaration {
    UiPreviewFixtureDeclaration {
        id: id(fixture_id),
        data_state: state,
        target_profiles: profiles(&["editor.workbench", "game.runtime"]),
        required_data_packages: ids(&["preview.data"]),
        required_capabilities: ids(&["preview.run"]),
        expected_diagnostics: ids(&["diag.expected"]),
        expected_states: ids(&["state.ready"]),
        source_package: id("test.package"),
        source_location: None,
        preview_only: false,
    }
}

fn all_fixtures() -> Vec<UiPreviewFixtureDeclaration> {
    vec![
        fixture("empty", UiPreviewDataStateKind::Empty),
        fixture("loading", UiPreviewDataStateKind::Loading),
        fixture("error", UiPreviewDataStateKind::Error),
        fixture("denied", UiPreviewDataStateKind::Denied),
        fixture("offline", UiPreviewDataStateKind::Offline),
        fixture("heavy", UiPreviewDataStateKind::Heavy),
        fixture("accessibility", UiPreviewDataStateKind::Accessibility),
    ]
}

fn scenario() -> UiPreviewScenarioDeclaration {
    UiPreviewScenarioDeclaration {
        id: id("open-panel"),
        fixture: id("empty"),
        target_profiles: profiles(&["editor.workbench", "game.runtime"]),
        steps: vec![UiPreviewScenarioStep {
            id: id("step-open"),
            kind: UiPreviewScenarioStepKind::Intent,
            target_node: Some(id("button")),
            expected_state: Some(id("state.ready")),
        }],
        required_capabilities: ids(&["preview.run"]),
        expected_diagnostics: ids(&["diag.expected"]),
        source_package: id("test.package"),
        source_location: None,
        preview_only: false,
    }
}

fn matrix() -> UiPreviewMatrixDeclaration {
    UiPreviewMatrixDeclaration {
        id: id("default-matrix"),
        fixtures: vec![id("empty")],
        scenarios: vec![id("open-panel")],
        target_profiles: profiles(&["editor.workbench", "game.runtime"]),
        axes: vec![
            UiPreviewMatrixAxis {
                kind: UiPreviewMatrixAxisKind::SafeArea,
                value: "hud-safe-area".to_string(),
            },
            UiPreviewMatrixAxis {
                kind: UiPreviewMatrixAxisKind::Input,
                value: "keyboard".to_string(),
            },
            UiPreviewMatrixAxis {
                kind: UiPreviewMatrixAxisKind::PlatformPrompt,
                value: "keyboard-prompts".to_string(),
            },
            UiPreviewMatrixAxis {
                kind: UiPreviewMatrixAxisKind::Localization,
                value: "expanded-text".to_string(),
            },
            UiPreviewMatrixAxis {
                kind: UiPreviewMatrixAxisKind::Accessibility,
                value: "high-contrast".to_string(),
            },
            UiPreviewMatrixAxis {
                kind: UiPreviewMatrixAxisKind::Size,
                value: "split-screen".to_string(),
            },
            UiPreviewMatrixAxis {
                kind: UiPreviewMatrixAxisKind::Performance,
                value: "readability-budget".to_string(),
            },
            UiPreviewMatrixAxis {
                kind: UiPreviewMatrixAxisKind::ViewModelFreshness,
                value: "fresh-required".to_string(),
            },
        ],
        evidence: vec![UiPreviewEvidenceDescriptor {
            id: id("default-evidence"),
            target_profiles: profiles(&["editor.workbench", "game.runtime"]),
            expected_diagnostics: ids(&["diag.expected"]),
            expected_states: ids(&["state.ready"]),
        }],
        validation_mode: UiPreviewValidationMode::Preview,
        source_package: id("test.package"),
        source_location: None,
        preview_only: false,
    }
}

fn library() -> UiPreviewLibrary {
    UiPreviewLibrary {
        fixtures: all_fixtures(),
        scenarios: vec![scenario()],
        matrices: vec![matrix()],
        known_data_packages: ids(&["preview.data"]),
        known_capabilities: ids(&["preview.run"]),
    }
}

fn request(target: &str) -> UiPreviewValidationRequest {
    UiPreviewValidationRequest::preview(target)
        .with_data_package("preview.data")
        .with_actual_diagnostic("diag.expected")
}

fn codes(report: &UiPreviewValidationReport) -> BTreeSet<&str> {
    report
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.code.as_str())
        .collect()
}

#[test]
fn preview_fixture_validates_editor_and_runtime_examples_without_runtime_ownership() {
    let editor = validate_preview_fixtures(&library(), &request("editor.workbench"));
    let runtime = validate_preview_fixtures(&library(), &request("game.runtime"));

    assert!(!editor.has_errors(), "{:?}", editor.diagnostics);
    assert!(!runtime.has_errors(), "{:?}", runtime.diagnostics);
}

#[test]
fn preview_fixture_rejects_missing_data_state_coverage() {
    let mut library = library();
    library.fixtures.pop();

    let report = validate_preview_fixtures(&library, &request("editor.workbench"));

    assert!(report.has_errors());
    assert!(codes(&report).contains("ui.preview.fixture.data_state_coverage_missing"));
}

#[test]
fn preview_fixture_rejects_missing_data_package() {
    let report = validate_preview_fixtures(
        &library(),
        &UiPreviewValidationRequest::preview("editor.workbench")
            .with_actual_diagnostic("diag.expected"),
    );

    assert!(report.has_errors());
    assert!(codes(&report).contains("ui.preview.fixture.data_package.missing"));
}

#[test]
fn preview_fixture_rejects_denied_capability() {
    let report = validate_preview_fixtures(
        &library(),
        &request("editor.workbench").with_denied_capability("preview.run"),
    );

    assert!(report.has_errors());
    assert!(codes(&report).contains("ui.preview.capability.denied"));
}

#[test]
fn preview_fixture_rejects_denied_scenario_capability() {
    let mut library = library();
    library.scenarios[0].required_capabilities = ids(&["scenario.run"]);
    library.known_capabilities.insert(id("scenario.run"));

    let report = validate_preview_fixtures(
        &library,
        &request("editor.workbench").with_denied_capability("scenario.run"),
    );

    assert!(report.has_errors());
    assert!(codes(&report).contains("ui.preview.capability.denied"));
}

#[test]
fn preview_fixture_rejects_unsupported_target_profile() {
    let report = validate_preview_fixtures(&library(), &request("console.runtime"));

    assert!(report.has_errors());
    assert!(codes(&report).contains("ui.preview.fixture.target_profile.unsupported"));
    assert!(codes(&report).contains("ui.preview.scenario.target_profile.unsupported"));
    assert!(codes(&report).contains("ui.preview.matrix.target_profile.unsupported"));
}

#[test]
fn preview_fixture_rejects_invalid_scenario_steps() {
    let mut library = library();
    library.scenarios[0].steps.clear();

    let report = validate_preview_fixtures(&library, &request("editor.workbench"));

    assert!(report.has_errors());
    assert!(codes(&report).contains("ui.preview.scenario.step.invalid"));
}

#[test]
fn preview_fixture_rejects_matrix_axis_conflicts() {
    let mut library = library();
    library.matrices[0].axes.push(UiPreviewMatrixAxis {
        kind: UiPreviewMatrixAxisKind::Input,
        value: "mobile".to_string(),
    });

    let report = validate_preview_fixtures(&library, &request("editor.workbench"));

    assert!(report.has_errors());
    assert!(codes(&report).contains("ui.preview.matrix.axis.incompatible"));
}

#[test]
fn game_runtime_preview_matrix_requires_all_compatibility_axes() {
    let mut library = library();
    library.matrices[0]
        .axes
        .retain(|axis| axis.kind != UiPreviewMatrixAxisKind::SafeArea);

    let report = validate_preview_fixtures(&library, &request("game.runtime"));

    assert!(report.has_errors());
    assert!(codes(&report).contains("ui.preview.matrix.game_runtime_axis_coverage_missing"));
}

#[test]
fn game_runtime_preview_matrix_rejects_editor_only_evidence() {
    let mut library = library();
    library.matrices[0].evidence[0].target_profiles = profiles(&["editor.workbench"]);

    let report = validate_preview_fixtures(&library, &request("game.runtime"));

    assert!(report.has_errors());
    assert!(codes(&report).contains("ui.preview.evidence.target_profile.unsupported"));
}

#[test]
fn preview_fixture_rejects_expected_diagnostic_mismatches() {
    let report = validate_preview_fixtures(
        &library(),
        &UiPreviewValidationRequest::preview("editor.workbench").with_data_package("preview.data"),
    );

    assert!(report.has_errors());
    assert!(codes(&report).contains("ui.preview.expected_diagnostic.mismatch"));
}

#[test]
fn preview_fixture_rejects_unexpected_actual_diagnostics() {
    let report = validate_preview_fixtures(
        &library(),
        &request("editor.workbench").with_actual_diagnostic("diag.unexpected"),
    );

    assert!(report.has_errors());
    assert!(codes(&report).contains("ui.preview.expected_diagnostic.mismatch"));
}

#[test]
fn preview_fixture_rejects_preview_only_activation() {
    let mut library = library();
    library.fixtures[0].preview_only = true;
    library.scenarios[0].preview_only = true;
    library.matrices[0].preview_only = true;

    let report = validate_preview_fixtures(
        &library,
        &UiPreviewValidationRequest::activate("editor.workbench")
            .with_data_package("preview.data")
            .with_actual_diagnostic("diag.expected"),
    );

    assert!(report.has_errors());
    assert!(codes(&report).contains("ui.preview.fixture.preview_only_activation"));
    assert!(codes(&report).contains("ui.preview.scenario.preview_only_activation"));
    assert!(codes(&report).contains("ui.preview.matrix.preview_only_activation"));
}
