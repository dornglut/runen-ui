//! Preview scenario route and step declarations.

use crate::{UiNodeId, UiSourceLocation};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

use super::{
    UiPreviewCapabilityId, UiPreviewDiagnosticRef, UiPreviewFixtureId, UiPreviewScenarioId,
    UiPreviewSourcePackageId, UiPreviewStateRef, UiPreviewStepId, UiPreviewTargetProfileId,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UiPreviewScenarioStepKind {
    Intent,
    Input,
    Wait,
    AssertState,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiPreviewScenarioStep {
    pub id: UiPreviewStepId,
    pub kind: UiPreviewScenarioStepKind,
    #[serde(default)]
    pub target_node: Option<UiNodeId>,
    #[serde(default)]
    pub expected_state: Option<UiPreviewStateRef>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiPreviewScenarioDeclaration {
    pub id: UiPreviewScenarioId,
    pub fixture: UiPreviewFixtureId,
    #[serde(default)]
    pub target_profiles: BTreeSet<UiPreviewTargetProfileId>,
    #[serde(default)]
    pub steps: Vec<UiPreviewScenarioStep>,
    #[serde(default)]
    pub required_capabilities: BTreeSet<UiPreviewCapabilityId>,
    #[serde(default)]
    pub expected_diagnostics: BTreeSet<UiPreviewDiagnosticRef>,
    pub source_package: UiPreviewSourcePackageId,
    #[serde(default)]
    pub source_location: Option<UiSourceLocation>,
    pub preview_only: bool,
}
