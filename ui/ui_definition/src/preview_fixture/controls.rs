//! Preview fixture declarations.

use crate::UiSourceLocation;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

use super::{
    UiPreviewCapabilityId, UiPreviewDataPackageId, UiPreviewDataStateKind, UiPreviewDiagnosticRef,
    UiPreviewFixtureId, UiPreviewSourcePackageId, UiPreviewStateRef, UiPreviewTargetProfileId,
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiPreviewFixtureDeclaration {
    pub id: UiPreviewFixtureId,
    pub data_state: UiPreviewDataStateKind,
    #[serde(default)]
    pub target_profiles: BTreeSet<UiPreviewTargetProfileId>,
    #[serde(default)]
    pub required_data_packages: BTreeSet<UiPreviewDataPackageId>,
    #[serde(default)]
    pub required_capabilities: BTreeSet<UiPreviewCapabilityId>,
    #[serde(default)]
    pub expected_diagnostics: BTreeSet<UiPreviewDiagnosticRef>,
    #[serde(default)]
    pub expected_states: BTreeSet<UiPreviewStateRef>,
    pub source_package: UiPreviewSourcePackageId,
    #[serde(default)]
    pub source_location: Option<UiSourceLocation>,
    pub preview_only: bool,
}
