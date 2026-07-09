use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::identity::UiStoryId;
use crate::manifest_v2::UiStoryManifestV2;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ValidatedUiStoryRegistryV2 {
    stories: BTreeMap<UiStoryId, UiStoryManifestV2>,
}

impl ValidatedUiStoryRegistryV2 {
    pub(crate) fn new(stories: BTreeMap<UiStoryId, UiStoryManifestV2>) -> Self {
        Self { stories }
    }

    pub fn get(&self, story_id: &UiStoryId) -> Option<&UiStoryManifestV2> {
        self.stories.get(story_id)
    }

    pub fn contains(&self, story_id: &UiStoryId) -> bool {
        self.stories.contains_key(story_id)
    }

    pub fn stories(
        &self,
    ) -> impl DoubleEndedIterator<Item = &UiStoryManifestV2> + ExactSizeIterator {
        self.stories.values()
    }

    pub fn story_ids(&self) -> impl DoubleEndedIterator<Item = &UiStoryId> + ExactSizeIterator {
        self.stories.keys()
    }

    pub fn len(&self) -> usize {
        self.stories.len()
    }

    pub fn is_empty(&self) -> bool {
        self.stories.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validated_registry_v2_can_be_empty_for_incremental_builds() {
        let registry = ValidatedUiStoryRegistryV2::new(BTreeMap::new());

        assert!(registry.is_empty());
        assert_eq!(registry.len(), 0);
    }
}
