//! Stable identity contracts for the UI Story V2 proof model.
//!
//! These identifiers are intentionally small value objects. They do not own
//! workflow, manifest, registry, or report behavior; later V2 modules use them
//! as stable keys for deterministic diagnostics and reports.

use serde::{Deserialize, Serialize};

macro_rules! story_string_id {
    ($name:ident) => {
        #[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(String);

        impl $name {
            pub fn new(value: impl Into<String>) -> Self {
                Self(value.into())
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }

            pub fn is_empty(&self) -> bool {
                self.0.trim().is_empty()
            }

            pub fn is_valid(&self) -> bool {
                !self.is_empty() && self.0.trim() == self.0
            }
        }
    };
}

story_string_id!(UiStoryId);
story_string_id!(UiStoryWorkflowProfileId);
story_string_id!(UiStoryWorkflowNodeId);
story_string_id!(UiStoryEvidenceProducerId);
story_string_id!(UiStoryEvidenceKey);
story_string_id!(UiStoryRunId);
story_string_id!(UiStoryManifestSourceId);
story_string_id!(UiStoryCategoryId);
story_string_id!(UiStoryProgramId);
story_string_id!(UiStoryHostProfileId);
story_string_id!(UiStoryThemeProfileId);
story_string_id!(UiStoryViewportProfileId);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct UiStoryRevision(u64);

impl UiStoryRevision {
    pub const fn new(value: u64) -> Self {
        Self(value)
    }

    pub const fn raw(self) -> u64 {
        self.0
    }

    pub const fn is_initial(self) -> bool {
        self.0 == 0
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use super::*;

    #[test]
    fn story_id_preserves_stable_string() {
        let story_id = UiStoryId::new("ui.gallery.button.basic");

        assert_eq!(story_id.as_str(), "ui.gallery.button.basic");
        assert!(!story_id.is_empty());
        assert!(story_id.is_valid());
    }

    #[test]
    fn ids_sort_deterministically() {
        let mut ids = [
            UiStoryWorkflowProfileId::new("ui_story.workflow.static_preview"),
            UiStoryWorkflowProfileId::new("ui_story.workflow.compiler_only"),
            UiStoryWorkflowProfileId::new("ui_story.workflow.source_load_only"),
        ];

        ids.sort();

        assert_eq!(
            ids.iter()
                .map(UiStoryWorkflowProfileId::as_str)
                .collect::<Vec<_>>(),
            vec![
                "ui_story.workflow.compiler_only",
                "ui_story.workflow.source_load_only",
                "ui_story.workflow.static_preview",
            ]
        );
    }

    #[test]
    fn empty_or_whitespace_ids_are_invalid() {
        assert!(UiStoryId::new("").is_empty());
        assert!(!UiStoryId::new("").is_valid());
        assert!(UiStoryId::new("   ").is_empty());
        assert!(!UiStoryId::new("   ").is_valid());
        assert!(!UiStoryId::new(" ui.story ").is_valid());
    }

    #[test]
    fn workflow_node_ids_can_be_used_as_btreemap_keys() {
        let mut nodes = BTreeMap::new();
        nodes.insert(UiStoryWorkflowNodeId::new("source_parse"), 2);
        nodes.insert(UiStoryWorkflowNodeId::new("source_load"), 1);

        assert_eq!(
            nodes
                .keys()
                .map(UiStoryWorkflowNodeId::as_str)
                .collect::<Vec<_>>(),
            vec!["source_load", "source_parse"]
        );
    }

    #[test]
    fn revision_preserves_raw_value() {
        let revision = UiStoryRevision::new(7);

        assert_eq!(revision.raw(), 7);
        assert!(!revision.is_initial());
        assert!(UiStoryRevision::new(0).is_initial());
    }
}
