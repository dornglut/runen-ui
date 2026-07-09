use serde::{Deserialize, Serialize};

use crate::identity::UiStoryManifestSourceId;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiStoryManifestSourceV2 {
    pub source_id: UiStoryManifestSourceId,
    pub path: String,
    pub contents: String,
}

impl UiStoryManifestSourceV2 {
    pub fn new(
        source_id: impl Into<String>,
        path: impl Into<String>,
        contents: impl Into<String>,
    ) -> Self {
        Self {
            source_id: UiStoryManifestSourceId::new(source_id),
            path: path.into(),
            contents: contents.into(),
        }
    }

    pub fn path_is_valid(&self) -> bool {
        !self.path.trim().is_empty() && self.path.trim() == self.path
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn registry_v2_manifest_source_keeps_contents_in_memory() {
        let source = UiStoryManifestSourceV2::new(
            "manifest.basic",
            "virtual/basic.story.ron",
            "schema_version: 2",
        );

        assert_eq!(source.source_id.as_str(), "manifest.basic");
        assert_eq!(source.path, "virtual/basic.story.ron");
        assert_eq!(source.contents, "schema_version: 2");
        assert!(source.path_is_valid());
    }
}
