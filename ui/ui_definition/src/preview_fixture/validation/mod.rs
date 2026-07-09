//! Preview fixture validation entrypoint.

mod diagnostics;
mod fixtures;
mod matrices;
mod scenarios;

pub use diagnostics::{UiPreviewDiagnostic, UiPreviewValidationReport};

use std::collections::{BTreeMap, BTreeSet};

use super::{
    UiPreviewDiagnosticRef, UiPreviewFixtureDeclaration, UiPreviewFixtureId, UiPreviewLibrary,
    UiPreviewScenarioDeclaration, UiPreviewScenarioId, UiPreviewValidationRequest,
};

pub fn validate_preview_fixtures(
    library: &UiPreviewLibrary,
    request: &UiPreviewValidationRequest,
) -> UiPreviewValidationReport {
    let mut diagnostics = Vec::new();
    let fixtures = index_fixtures(library, request, &mut diagnostics);
    let scenarios = index_scenarios(library, request, &mut diagnostics);

    let mut validator = PreviewValidator {
        library,
        request,
        diagnostics,
    };

    validator.validate_data_state_coverage();
    validator.validate_fixtures(&fixtures);
    validator.validate_scenarios(&fixtures);
    validator.validate_matrices(&fixtures, &scenarios);

    UiPreviewValidationReport {
        diagnostics: validator.diagnostics,
    }
}

fn index_fixtures<'a>(
    library: &'a UiPreviewLibrary,
    request: &UiPreviewValidationRequest,
    diagnostics: &mut Vec<UiPreviewDiagnostic>,
) -> BTreeMap<UiPreviewFixtureId, &'a UiPreviewFixtureDeclaration> {
    let mut fixtures = BTreeMap::new();
    for fixture in &library.fixtures {
        if fixtures.insert(fixture.id.clone(), fixture).is_some() {
            diagnostics.push(
                UiPreviewDiagnostic::error(
                    "ui.preview.fixture.duplicate_id",
                    format!(
                        "Preview fixture '{}' is declared more than once.",
                        fixture.id
                    ),
                    "Keep one fixture declaration for each stable fixture id.",
                )
                .for_fixture(fixture)
                .with_target_profile(request.target_profile.clone()),
            );
        }
    }
    fixtures
}

fn index_scenarios<'a>(
    library: &'a UiPreviewLibrary,
    request: &UiPreviewValidationRequest,
    diagnostics: &mut Vec<UiPreviewDiagnostic>,
) -> BTreeMap<UiPreviewScenarioId, &'a UiPreviewScenarioDeclaration> {
    let mut scenarios = BTreeMap::new();
    for scenario in &library.scenarios {
        if scenarios.insert(scenario.id.clone(), scenario).is_some() {
            diagnostics.push(
                UiPreviewDiagnostic::error(
                    "ui.preview.scenario.duplicate_id",
                    format!(
                        "Preview scenario '{}' is declared more than once.",
                        scenario.id
                    ),
                    "Keep one scenario declaration for each stable scenario id.",
                )
                .for_scenario(scenario)
                .with_target_profile(request.target_profile.clone()),
            );
        }
    }
    scenarios
}

pub(super) struct PreviewValidator<'a> {
    pub(super) library: &'a UiPreviewLibrary,
    pub(super) request: &'a UiPreviewValidationRequest,
    pub(super) diagnostics: Vec<UiPreviewDiagnostic>,
}

impl PreviewValidator<'_> {
    pub(super) fn validate_expected_diagnostics(
        &mut self,
        expected: &BTreeSet<UiPreviewDiagnosticRef>,
        diagnostic: UiPreviewDiagnostic,
    ) {
        if expected != &self.request.actual_diagnostics {
            self.diagnostics.push(diagnostic.with_diagnostic_mismatch(
                expected.iter().cloned().collect(),
                self.request.actual_diagnostics.iter().cloned().collect(),
            ));
        }
    }
}
