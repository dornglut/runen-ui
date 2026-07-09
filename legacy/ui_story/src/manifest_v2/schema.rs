use serde::{Deserialize, Serialize};

use crate::diagnostic::UI_STORY_MANIFEST_SCHEMA_UNSUPPORTED;

pub const UI_STORY_MANIFEST_V2_SCHEMA_VERSION: u32 = 2;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiStoryManifestV2ParseError {
    pub code: String,
    pub message: String,
}

impl UiStoryManifestV2ParseError {
    pub fn new(code: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            code: code.into(),
            message: message.into(),
        }
    }

    pub fn parse_failed(message: impl Into<String>) -> Self {
        Self::new(UI_STORY_MANIFEST_SCHEMA_UNSUPPORTED, message)
    }
}
