//! Preview fixture validation diagnostics.

use crate::{UiDefinitionDiagnosticSeverity, UiSourceLocation};
use serde::{Deserialize, Serialize};

use super::super::{
    UiPreviewActivationImpact, UiPreviewCapabilityId, UiPreviewDiagnosticDomain,
    UiPreviewDiagnosticRef, UiPreviewFixtureDeclaration, UiPreviewFixtureId, UiPreviewMatrixAxis,
    UiPreviewMatrixDeclaration, UiPreviewMatrixId, UiPreviewScenarioDeclaration,
    UiPreviewScenarioId, UiPreviewSourcePackageId, UiPreviewTargetProfileId,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiPreviewDiagnostic {
    pub severity: UiDefinitionDiagnosticSeverity,
    pub code: String,
    pub message: String,
    #[serde(default)]
    pub fixture: Option<UiPreviewFixtureId>,
    #[serde(default)]
    pub scenario: Option<UiPreviewScenarioId>,
    #[serde(default)]
    pub matrix: Option<UiPreviewMatrixId>,
    #[serde(default)]
    pub axis: Option<UiPreviewMatrixAxis>,
    #[serde(default)]
    pub source_location: Option<UiSourceLocation>,
    #[serde(default)]
    pub target_profile: Option<UiPreviewTargetProfileId>,
    pub owning_domain: UiPreviewDiagnosticDomain,
    #[serde(default)]
    pub source_package: Option<UiPreviewSourcePackageId>,
    #[serde(default)]
    pub expected_diagnostics: Vec<UiPreviewDiagnosticRef>,
    #[serde(default)]
    pub actual_diagnostics: Vec<UiPreviewDiagnosticRef>,
    #[serde(default)]
    pub denied_capabilities: Vec<UiPreviewCapabilityId>,
    pub activation_impact: UiPreviewActivationImpact,
    pub suggested_fix: String,
}

impl UiPreviewDiagnostic {
    pub(super) fn error(
        code: impl Into<String>,
        message: impl Into<String>,
        suggested_fix: impl Into<String>,
    ) -> Self {
        Self {
            severity: UiDefinitionDiagnosticSeverity::Error,
            code: code.into(),
            message: message.into(),
            fixture: None,
            scenario: None,
            matrix: None,
            axis: None,
            source_location: None,
            target_profile: None,
            owning_domain: UiPreviewDiagnosticDomain::UiDefinition,
            source_package: None,
            expected_diagnostics: Vec::new(),
            actual_diagnostics: Vec::new(),
            denied_capabilities: Vec::new(),
            activation_impact: UiPreviewActivationImpact::BlocksActivation,
            suggested_fix: suggested_fix.into(),
        }
    }

    pub(super) fn for_fixture(mut self, fixture: &UiPreviewFixtureDeclaration) -> Self {
        self.fixture = Some(fixture.id.clone());
        self.source_location = fixture.source_location.clone();
        self.source_package = Some(fixture.source_package.clone());
        self.expected_diagnostics = fixture.expected_diagnostics.iter().cloned().collect();
        self
    }

    pub(super) fn for_scenario(mut self, scenario: &UiPreviewScenarioDeclaration) -> Self {
        self.scenario = Some(scenario.id.clone());
        self.fixture = Some(scenario.fixture.clone());
        self.source_location = scenario.source_location.clone();
        self.source_package = Some(scenario.source_package.clone());
        self.expected_diagnostics = scenario.expected_diagnostics.iter().cloned().collect();
        self
    }

    pub(super) fn for_matrix(mut self, matrix: &UiPreviewMatrixDeclaration) -> Self {
        self.matrix = Some(matrix.id.clone());
        self.source_location = matrix.source_location.clone();
        self.source_package = Some(matrix.source_package.clone());
        self
    }

    pub(super) fn with_target_profile(mut self, target_profile: UiPreviewTargetProfileId) -> Self {
        self.target_profile = Some(target_profile);
        self
    }

    pub(super) fn with_axis(mut self, axis: UiPreviewMatrixAxis) -> Self {
        self.axis = Some(axis);
        self
    }

    pub(super) fn with_denied_capabilities(
        mut self,
        capabilities: Vec<UiPreviewCapabilityId>,
    ) -> Self {
        self.denied_capabilities = capabilities;
        self
    }

    pub(super) fn with_diagnostic_mismatch(
        mut self,
        expected: Vec<UiPreviewDiagnosticRef>,
        actual: Vec<UiPreviewDiagnosticRef>,
    ) -> Self {
        self.expected_diagnostics = expected;
        self.actual_diagnostics = actual;
        self
    }

    pub(super) fn preview_only(mut self) -> Self {
        self.activation_impact = UiPreviewActivationImpact::PreviewOnly;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiPreviewValidationReport {
    #[serde(default)]
    pub diagnostics: Vec<UiPreviewDiagnostic>,
}

impl UiPreviewValidationReport {
    pub fn has_errors(&self) -> bool {
        self.diagnostics
            .iter()
            .any(|diagnostic| diagnostic.severity == UiDefinitionDiagnosticSeverity::Error)
    }
}
