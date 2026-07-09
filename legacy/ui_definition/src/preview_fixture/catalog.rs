//! Preview fixture catalog vocabularies.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum UiPreviewDataStateKind {
    Empty,
    Loading,
    Error,
    Denied,
    Offline,
    Heavy,
    Accessibility,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum UiPreviewMatrixAxisKind {
    Platform,
    SafeArea,
    PlatformPrompt,
    Accessibility,
    Localization,
    Input,
    Size,
    Performance,
    ViewModelFreshness,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct UiPreviewMatrixAxis {
    pub kind: UiPreviewMatrixAxisKind,
    pub value: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UiPreviewValidationMode {
    Preview,
    DryRun,
    AcceptanceEvidence,
    Activate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UiPreviewActivationImpact {
    None,
    PreviewOnly,
    BlocksActivation,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UiPreviewDiagnosticDomain {
    UiDefinition,
}
