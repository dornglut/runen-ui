//! Deterministic visual layout diff contracts.

use super::operation::{UiTargetProfileId, UiVisualLayoutOperationId};
use crate::{AuthoredUiNodePath, UiTemplateId};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UiVisualLayoutDiffChangeKind {
    Insert,
    Remove,
    Move,
    Reorder,
    Update,
    Wrap,
    Unwrap,
    ReplaceTemplate,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiVisualLayoutDiffChange {
    pub kind: UiVisualLayoutDiffChangeKind,
    pub path: AuthoredUiNodePath,
    pub before: Option<String>,
    pub after: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiVisualLayoutDiff {
    pub operation_id: UiVisualLayoutOperationId,
    pub source_document: UiTemplateId,
    pub target_profile: UiTargetProfileId,
    pub changes: Vec<UiVisualLayoutDiffChange>,
}
