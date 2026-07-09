//! Scenario validation passes.

use std::collections::BTreeMap;

use super::super::{
    UiPreviewFixtureDeclaration, UiPreviewFixtureId, UiPreviewScenarioDeclaration,
    UiPreviewValidationMode,
};
use super::{PreviewValidator, UiPreviewDiagnostic};

impl PreviewValidator<'_> {
    pub(super) fn validate_scenarios(
        &mut self,
        fixtures: &BTreeMap<UiPreviewFixtureId, &UiPreviewFixtureDeclaration>,
    ) {
        for scenario in &self.library.scenarios {
            if !fixtures.contains_key(&scenario.fixture) {
                self.diagnostics.push(
                    UiPreviewDiagnostic::error(
                        "ui.preview.scenario.fixture_unknown",
                        format!(
                            "Scenario '{}' references unknown fixture '{}'.",
                            scenario.id, scenario.fixture
                        ),
                        "Add the fixture declaration or update the scenario fixture reference.",
                    )
                    .for_scenario(scenario)
                    .with_target_profile(self.request.target_profile.clone()),
                );
            }

            if !scenario.target_profiles.is_empty()
                && !scenario
                    .target_profiles
                    .contains(&self.request.target_profile)
            {
                self.diagnostics.push(
                    UiPreviewDiagnostic::error(
                        "ui.preview.scenario.target_profile.unsupported",
                        format!(
                            "Scenario '{}' does not support target profile '{}'.",
                            scenario.id, self.request.target_profile
                        ),
                        "Add target-profile support to the scenario or choose a compatible scenario.",
                    )
                    .for_scenario(scenario)
                    .with_target_profile(self.request.target_profile.clone()),
                );
            }

            if scenario.preview_only && self.request.mode == UiPreviewValidationMode::Activate {
                self.diagnostics.push(
                    UiPreviewDiagnostic::error(
                        "ui.preview.scenario.preview_only_activation",
                        format!(
                            "Scenario '{}' is preview-only and cannot activate.",
                            scenario.id
                        ),
                        "Use preview validation or remove the preview-only flag before activation.",
                    )
                    .for_scenario(scenario)
                    .with_target_profile(self.request.target_profile.clone())
                    .preview_only(),
                );
            }

            self.validate_scenario_required_capabilities(scenario);

            if scenario.steps.is_empty()
                || scenario
                    .steps
                    .iter()
                    .any(|step| step.id.as_str().trim().is_empty())
            {
                self.diagnostics.push(
                    UiPreviewDiagnostic::error(
                        "ui.preview.scenario.step.invalid",
                        format!("Scenario '{}' contains invalid replay steps.", scenario.id),
                        "Add at least one replay step and ensure every step has a stable id.",
                    )
                    .for_scenario(scenario)
                    .with_target_profile(self.request.target_profile.clone()),
                );
            }

            self.validate_expected_diagnostics(
                &scenario.expected_diagnostics,
                UiPreviewDiagnostic::error(
                    "ui.preview.expected_diagnostic.mismatch",
                    format!(
                        "Scenario '{}' expected diagnostics did not match actual diagnostics.",
                        scenario.id
                    ),
                    "Update the expected diagnostics or fix the scenario input that produced different diagnostics.",
                )
                .for_scenario(scenario)
                .with_target_profile(self.request.target_profile.clone()),
            );
        }
    }

    fn validate_scenario_required_capabilities(&mut self, scenario: &UiPreviewScenarioDeclaration) {
        let denied: Vec<_> = scenario
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
                    format!("Scenario '{}' requires unavailable capabilities.", scenario.id),
                    "Grant the capability for this target profile or remove the scenario requirement.",
                )
                .for_scenario(scenario)
                .with_target_profile(self.request.target_profile.clone())
                .with_denied_capabilities(denied),
            );
        }
    }
}
