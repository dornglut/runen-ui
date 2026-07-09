use serde::{Deserialize, Serialize};

use crate::identity::UiStoryManifestSourceId;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum UiStorySourceKindV2 {
    Node,
    Template,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct UiStorySourceRef {
    pub source_id: UiStoryManifestSourceId,
    pub path: String,
    pub kind: UiStorySourceKindV2,
}

impl UiStorySourceRef {
    pub fn new(
        source_id: UiStoryManifestSourceId,
        path: impl Into<String>,
        kind: UiStorySourceKindV2,
    ) -> Self {
        Self {
            source_id,
            path: path.into(),
            kind,
        }
    }

    pub fn node(path: impl Into<String>, source_id: impl Into<String>) -> Self {
        Self::new(
            UiStoryManifestSourceId::new(source_id),
            path,
            UiStorySourceKindV2::Node,
        )
    }

    pub fn template(path: impl Into<String>, source_id: impl Into<String>) -> Self {
        Self::new(
            UiStoryManifestSourceId::new(source_id),
            path,
            UiStorySourceKindV2::Template,
        )
    }

    pub fn path_is_valid(&self) -> bool {
        !self.path.trim().is_empty() && self.path.trim() == self.path
    }

    pub fn is_valid(&self) -> bool {
        self.source_id.is_valid() && self.path_is_valid()
    }
}
