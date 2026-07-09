//! Matrix and evidence validation passes.

use std::collections::{BTreeMap, BTreeSet};

use super::super::{
    UiPreviewFixtureDeclaration, UiPreviewFixtureId, UiPreviewMatrixAxisKind,
    UiPreviewMatrixDeclaration, UiPreviewScenarioDeclaration, UiPreviewScenarioId,
    UiPreviewValidationMode,
};
use super::{PreviewValidator, UiPreviewDiagnostic};

impl PreviewValidator<'_> {
    pub(super) fn validate_matrices(
        &mut self,
        fixtures: &BTreeMap<UiPreviewFixtureId, &UiPreviewFixtureDeclaration>,
        scenarios: &BTreeMap<UiPreviewScenarioId, &UiPreviewScenarioDeclaration>,
    ) {
        for matrix in &self.library.matrices {
            if !matrix.target_profiles.is_empty()
                && !matrix
                    .target_profiles
                    .contains(&self.request.target_profile)
            {
                self.diagnostics.push(
                    UiPreviewDiagnostic::error(
                        "ui.preview.matrix.target_profile.unsupported",
                        format!(
                            "Matrix '{}' does not support target profile '{}'.",
                            matrix.id, self.request.target_profile
                        ),
                        "Add target-profile support to the matrix or choose a compatible matrix.",
                    )
                    .for_matrix(matrix)
                    .with_target_profile(self.request.target_profile.clone()),
                );
            }

            if matrix.preview_only && self.request.mode == UiPreviewValidationMode::Activate {
                self.diagnostics.push(
                    UiPreviewDiagnostic::error(
                        "ui.preview.matrix.preview_only_activation",
                        format!(
                            "Matrix '{}' is preview-only and cannot activate.",
                            matrix.id
                        ),
                        "Use preview validation or remove the preview-only flag before activation.",
                    )
                    .for_matrix(matrix)
                    .with_target_profile(self.request.target_profile.clone())
                    .preview_only(),
                );
            }

            for fixture_id in &matrix.fixtures {
                if !fixtures.contains_key(fixture_id) {
                    self.diagnostics.push(
                        UiPreviewDiagnostic::error(
                            "ui.preview.matrix.fixture_unknown",
                            format!(
                                "Matrix '{}' references unknown fixture '{}'.",
                                matrix.id, fixture_id
                            ),
                            "Add the fixture declaration or remove it from the matrix.",
                        )
                        .for_matrix(matrix)
                        .with_target_profile(self.request.target_profile.clone()),
                    );
                }
            }

            for scenario_id in &matrix.scenarios {
                if !scenarios.contains_key(scenario_id) {
                    self.diagnostics.push(
                        UiPreviewDiagnostic::error(
                            "ui.preview.matrix.scenario_unknown",
                            format!(
                                "Matrix '{}' references unknown scenario '{}'.",
                                matrix.id, scenario_id
                            ),
                            "Add the scenario declaration or remove it from the matrix.",
                        )
                        .for_matrix(matrix)
                        .with_target_profile(self.request.target_profile.clone()),
                    );
                }
            }

            let mut axis_kinds = BTreeSet::new();
            for axis in &matrix.axes {
                if !axis_kinds.insert(axis.kind) || axis.value.trim().is_empty() {
                    self.diagnostics.push(
                        UiPreviewDiagnostic::error(
                            "ui.preview.matrix.axis.incompatible",
                            format!("Matrix '{}' contains an incompatible axis.", matrix.id),
                            "Keep one non-empty value for each matrix axis kind in this bounded contract.",
                        )
                        .for_matrix(matrix)
                        .with_target_profile(self.request.target_profile.clone())
                        .with_axis(axis.clone()),
                    );
                }
            }

            self.validate_game_runtime_axis_coverage(matrix, &axis_kinds);

            for evidence in &matrix.evidence {
                if !evidence.target_profiles.is_empty()
                    && !evidence
                        .target_profiles
                        .contains(&self.request.target_profile)
                {
                    self.diagnostics.push(
                        UiPreviewDiagnostic::error(
                            "ui.preview.evidence.target_profile.unsupported",
                            format!(
                                "Evidence descriptor '{}' does not support target profile '{}'.",
                                evidence.id, self.request.target_profile
                            ),
                            "Add target-profile support to the evidence descriptor or choose compatible evidence.",
                        )
                        .for_matrix(matrix)
                        .with_target_profile(self.request.target_profile.clone()),
                    );
                }

                self.validate_expected_diagnostics(
                    &evidence.expected_diagnostics,
                    UiPreviewDiagnostic::error(
                        "ui.preview.expected_diagnostic.mismatch",
                        format!(
                            "Evidence descriptor '{}' expected diagnostics did not match actual diagnostics.",
                            evidence.id
                        ),
                        "Update the evidence descriptor expected diagnostics or refresh the preview evidence.",
                    )
                    .for_matrix(matrix)
                    .with_target_profile(self.request.target_profile.clone()),
                );
            }
        }
    }

    fn validate_game_runtime_axis_coverage(
        &mut self,
        matrix: &UiPreviewMatrixDeclaration,
        axis_kinds: &BTreeSet<UiPreviewMatrixAxisKind>,
    ) {
        if self.request.target_profile.as_str() != "game.runtime" {
            return;
        }

        let applies_to_runtime = matrix.target_profiles.is_empty()
            || matrix
                .target_profiles
                .contains(&self.request.target_profile);
        if !applies_to_runtime {
            return;
        }

        let required = BTreeSet::from([
            UiPreviewMatrixAxisKind::SafeArea,
            UiPreviewMatrixAxisKind::Input,
            UiPreviewMatrixAxisKind::PlatformPrompt,
            UiPreviewMatrixAxisKind::Localization,
            UiPreviewMatrixAxisKind::Accessibility,
            UiPreviewMatrixAxisKind::Size,
            UiPreviewMatrixAxisKind::Performance,
            UiPreviewMatrixAxisKind::ViewModelFreshness,
        ]);

        if !required.is_subset(axis_kinds) {
            self.diagnostics.push(
                UiPreviewDiagnostic::error(
                    "ui.preview.matrix.game_runtime_axis_coverage_missing",
                    format!(
                        "Matrix '{}' does not cover every game.runtime compatibility axis.",
                        matrix.id
                    ),
                    "Add safe-area, input, platform-prompt, localization, accessibility, size, performance, and view-model freshness axes.",
                )
                .for_matrix(matrix)
                .with_target_profile(self.request.target_profile.clone()),
            );
        }
    }
}
