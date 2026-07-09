//! Fixture validation passes.

use std::collections::{BTreeMap, BTreeSet};

use super::super::{
    UiPreviewDataStateKind, UiPreviewFixtureDeclaration, UiPreviewFixtureId,
    UiPreviewValidationMode,
};
use super::{PreviewValidator, UiPreviewDiagnostic};

impl PreviewValidator<'_> {
    pub(super) fn validate_data_state_coverage(&mut self) {
        let covered: BTreeSet<_> = self
            .library
            .fixtures
            .iter()
            .map(|fixture| fixture.data_state)
            .collect();
        let required = BTreeSet::from([
            UiPreviewDataStateKind::Empty,
            UiPreviewDataStateKind::Loading,
            UiPreviewDataStateKind::Error,
            UiPreviewDataStateKind::Denied,
            UiPreviewDataStateKind::Offline,
            UiPreviewDataStateKind::Heavy,
            UiPreviewDataStateKind::Accessibility,
        ]);

        if !required.is_subset(&covered) {
            self.diagnostics.push(
                UiPreviewDiagnostic::error(
                    "ui.preview.fixture.data_state_coverage_missing",
                    "Preview fixture library does not cover all required data states.",
                    "Add empty, loading, error, denied, offline, heavy, and accessibility fixtures.",
                )
                .with_target_profile(self.request.target_profile.clone()),
            );
        }
    }

    pub(super) fn validate_fixtures(
        &mut self,
        fixtures: &BTreeMap<UiPreviewFixtureId, &UiPreviewFixtureDeclaration>,
    ) {
        for fixture in fixtures.values() {
            self.validate_fixture_target_profile(fixture);
            self.validate_preview_only_fixture(fixture);
            self.validate_required_data_packages(fixture);
            self.validate_required_capabilities(fixture);
            self.validate_expected_diagnostics_for_fixture(fixture);
        }
    }

    fn validate_fixture_target_profile(&mut self, fixture: &UiPreviewFixtureDeclaration) {
        if !fixture.target_profiles.is_empty()
            && !fixture
                .target_profiles
                .contains(&self.request.target_profile)
        {
            self.diagnostics.push(
                UiPreviewDiagnostic::error(
                    "ui.preview.fixture.target_profile.unsupported",
                    format!(
                        "Fixture '{}' does not support target profile '{}'.",
                        fixture.id, self.request.target_profile
                    ),
                    "Add target-profile support to the fixture or choose a compatible fixture.",
                )
                .for_fixture(fixture)
                .with_target_profile(self.request.target_profile.clone()),
            );
        }
    }

    fn validate_preview_only_fixture(&mut self, fixture: &UiPreviewFixtureDeclaration) {
        if fixture.preview_only && self.request.mode == UiPreviewValidationMode::Activate {
            self.diagnostics.push(
                UiPreviewDiagnostic::error(
                    "ui.preview.fixture.preview_only_activation",
                    format!(
                        "Fixture '{}' is preview-only and cannot activate.",
                        fixture.id
                    ),
                    "Use preview validation or remove the preview-only flag before activation.",
                )
                .for_fixture(fixture)
                .with_target_profile(self.request.target_profile.clone())
                .preview_only(),
            );
        }
    }

    fn validate_required_data_packages(&mut self, fixture: &UiPreviewFixtureDeclaration) {
        let missing: Vec<_> = fixture
            .required_data_packages
            .iter()
            .filter(|package| {
                !self.library.known_data_packages.contains(*package)
                    || !self.request.available_data_packages.contains(*package)
            })
            .cloned()
            .collect();
        if !missing.is_empty() {
            self.diagnostics.push(
                UiPreviewDiagnostic::error(
                    "ui.preview.fixture.data_package.missing",
                    format!("Fixture '{}' requires missing data packages.", fixture.id),
                    "Provide the data packages or remove them from the fixture requirement.",
                )
                .for_fixture(fixture)
                .with_target_profile(self.request.target_profile.clone()),
            );
        }
    }

    fn validate_required_capabilities(&mut self, fixture: &UiPreviewFixtureDeclaration) {
        let denied: Vec<_> = fixture
            .required_capabilities
            .iter()
            .filter(|capability| {
                !self.library.known_capabilities.contains(*capability)
                    || self.request.denied_capabilities.contains(*capability)
            })
            .cloned()
            .collect();
        if !denied.is_empty() {
            self.diagnostics.push(
                UiPreviewDiagnostic::error(
                    "ui.preview.capability.denied",
                    format!("Fixture '{}' requires unavailable capabilities.", fixture.id),
                    "Grant the capability for this target profile or remove the fixture requirement.",
                )
                .for_fixture(fixture)
                .with_target_profile(self.request.target_profile.clone())
                .with_denied_capabilities(denied),
            );
        }
    }

    fn validate_expected_diagnostics_for_fixture(&mut self, fixture: &UiPreviewFixtureDeclaration) {
        self.validate_expected_diagnostics(
            &fixture.expected_diagnostics,
            UiPreviewDiagnostic::error(
                "ui.preview.expected_diagnostic.mismatch",
                format!("Fixture '{}' expected diagnostics did not match actual diagnostics.", fixture.id),
                "Update the expected diagnostics or fix the preview input that produced different diagnostics.",
            )
            .for_fixture(fixture)
            .with_target_profile(self.request.target_profile.clone()),
        );
    }
}
