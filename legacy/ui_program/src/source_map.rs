//! UiProgram source and source-map contracts.

use serde::{Deserialize, Serialize};

use crate::ids::{UiProgramSourceId, UiProgramTargetId};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiProgramSourceSpan {
    pub start_byte: u32,
    pub end_byte: u32,
}

impl UiProgramSourceSpan {
    pub const fn new(start_byte: u32, end_byte: u32) -> Self {
        assert!(start_byte <= end_byte);
        Self {
            start_byte,
            end_byte,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum UiProgramSourceKind {
    Authored,
    Generated,
    Migrated,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiProgramSource {
    pub source_id: UiProgramSourceId,
    pub kind: UiProgramSourceKind,
    pub description: String,
}

impl UiProgramSource {
    pub fn authored(source_id: UiProgramSourceId, description: impl Into<String>) -> Self {
        Self {
            source_id,
            kind: UiProgramSourceKind::Authored,
            description: description.into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiProgramSourceMapEntry {
    pub source_id: String,
    pub target_id: String,
}

impl UiProgramSourceMapEntry {
    pub fn new(source_id: UiProgramSourceId, target_id: UiProgramTargetId) -> Self {
        Self {
            source_id: source_id.as_str().to_owned(),
            target_id: target_id.as_str().to_owned(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiProgramSourceMapAttachment {
    pub entry: UiProgramSourceMapEntry,
    pub source_span: Option<UiProgramSourceSpan>,
    pub generated_by: Option<String>,
}

impl UiProgramSourceMapAttachment {
    pub fn new(entry: UiProgramSourceMapEntry) -> Self {
        Self {
            entry,
            source_span: None,
            generated_by: None,
        }
    }

    pub fn with_source_span(mut self, source_span: UiProgramSourceSpan) -> Self {
        self.source_span = Some(source_span);
        self
    }

    pub fn generated_by(mut self, rule_id: impl Into<String>) -> Self {
        self.generated_by = Some(rule_id.into());
        self
    }
}
