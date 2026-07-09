//! Preview validation request and library builders.

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

use super::{
    UiPreviewCapabilityId, UiPreviewDataPackageId, UiPreviewDiagnosticRef,
    UiPreviewFixtureDeclaration, UiPreviewMatrixDeclaration, UiPreviewScenarioDeclaration,
    UiPreviewTargetProfileId, UiPreviewValidationMode,
};

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiPreviewLibrary {
    #[serde(default)]
    pub fixtures: Vec<UiPreviewFixtureDeclaration>,
    #[serde(default)]
    pub scenarios: Vec<UiPreviewScenarioDeclaration>,
    #[serde(default)]
    pub matrices: Vec<UiPreviewMatrixDeclaration>,
    #[serde(default)]
    pub known_data_packages: BTreeSet<UiPreviewDataPackageId>,
    #[serde(default)]
    pub known_capabilities: BTreeSet<UiPreviewCapabilityId>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiPreviewValidationRequest {
    pub target_profile: UiPreviewTargetProfileId,
    pub mode: UiPreviewValidationMode,
    #[serde(default)]
    pub available_data_packages: BTreeSet<UiPreviewDataPackageId>,
    #[serde(default)]
    pub denied_capabilities: BTreeSet<UiPreviewCapabilityId>,
    #[serde(default)]
    pub actual_diagnostics: BTreeSet<UiPreviewDiagnosticRef>,
}

impl UiPreviewValidationRequest {
    pub fn preview(target_profile: impl Into<UiPreviewTargetProfileId>) -> Self {
        Self {
            target_profile: target_profile.into(),
            mode: UiPreviewValidationMode::Preview,
            available_data_packages: BTreeSet::new(),
            denied_capabilities: BTreeSet::new(),
            actual_diagnostics: BTreeSet::new(),
        }
    }

    pub fn activate(target_profile: impl Into<UiPreviewTargetProfileId>) -> Self {
        Self {
            target_profile: target_profile.into(),
            mode: UiPreviewValidationMode::Activate,
            available_data_packages: BTreeSet::new(),
            denied_capabilities: BTreeSet::new(),
            actual_diagnostics: BTreeSet::new(),
        }
    }

    pub fn with_data_package(mut self, package: impl Into<UiPreviewDataPackageId>) -> Self {
        self.available_data_packages.insert(package.into());
        self
    }

    pub fn with_denied_capability(mut self, capability: impl Into<UiPreviewCapabilityId>) -> Self {
        self.denied_capabilities.insert(capability.into());
        self
    }

    pub fn with_actual_diagnostic(mut self, diagnostic: impl Into<UiPreviewDiagnosticRef>) -> Self {
        self.actual_diagnostics.insert(diagnostic.into());
        self
    }
}
