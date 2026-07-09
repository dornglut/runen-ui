//! Reproducibility assertions for headless fixture artifacts.

use serde::{Deserialize, Serialize};

use crate::HeadlessFixture;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReproducibilityAssertion {
    pub first_cache_key: String,
    pub second_cache_key: String,
    pub first_source_map_entries: usize,
    pub second_source_map_entries: usize,
    pub artifact_equal: bool,
}

impl ReproducibilityAssertion {
    pub fn from_fixture(fixture: &HeadlessFixture) -> Self {
        let first = fixture.compile();
        let second = fixture.compile();
        Self {
            first_cache_key: first.manifest.cache_key.as_str().to_owned(),
            second_cache_key: second.manifest.cache_key.as_str().to_owned(),
            first_source_map_entries: first.manifest.source_map.entries.len(),
            second_source_map_entries: second.manifest.source_map.entries.len(),
            artifact_equal: first == second,
        }
    }

    pub fn passed(&self) -> bool {
        self.artifact_equal
            && self.first_cache_key == self.second_cache_key
            && self.first_source_map_entries == self.second_source_map_entries
    }
}
