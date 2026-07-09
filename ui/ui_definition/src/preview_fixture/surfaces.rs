//! Preview matrix and evidence declarations.

use crate::UiSourceLocation;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

use super::{
    UiPreviewDiagnosticRef, UiPreviewEvidenceId, UiPreviewFixtureId, UiPreviewMatrixAxis,
    UiPreviewMatrixId, UiPreviewScenarioId, UiPreviewSourcePackageId, UiPreviewStateRef,
    UiPreviewTargetProfileId, UiPreviewValidationMode,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiPreviewEvidenceDescriptor {
    pub id: UiPreviewEvidenceId,
    #[serde(default)]
    pub target_profiles: BTreeSet<UiPreviewTargetProfileId>,
    #[serde(default)]
    pub expected_diagnostics: BTreeSet<UiPreviewDiagnosticRef>,
    #[serde(default)]
    pub expected_states: BTreeSet<UiPreviewStateRef>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiPreviewMatrixDeclaration {
    pub id: UiPreviewMatrixId,
    #[serde(default)]
    pub fixtures: Vec<UiPreviewFixtureId>,
    #[serde(default)]
    pub scenarios: Vec<UiPreviewScenarioId>,
    #[serde(default)]
    pub target_profiles: BTreeSet<UiPreviewTargetProfileId>,
    #[serde(default)]
    pub axes: Vec<UiPreviewMatrixAxis>,
    #[serde(default)]
    pub evidence: Vec<UiPreviewEvidenceDescriptor>,
    pub validation_mode: UiPreviewValidationMode,
    pub source_package: UiPreviewSourcePackageId,
    #[serde(default)]
    pub source_location: Option<UiSourceLocation>,
    pub preview_only: bool,
}
