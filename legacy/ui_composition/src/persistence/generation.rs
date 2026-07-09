use std::fmt;

use serde::{Deserialize, Serialize};

use super::CompositionDigest;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct CompositionGenerationId(CompositionDigest);

impl CompositionGenerationId {
    pub fn from_digest(digest: CompositionDigest) -> Self {
        Self(digest)
    }

    pub fn digest(&self) -> &CompositionDigest {
        &self.0
    }

    pub fn directory_name(&self) -> String {
        format!("generation-{}", self.0.hex())
    }
}

impl fmt::Display for CompositionGenerationId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompositionGenerationPointerV1 {
    pub pointer_schema_version: u16,
    pub active: CompositionGenerationId,
    pub previous: Option<CompositionGenerationId>,
}

impl CompositionGenerationPointerV1 {
    pub const POINTER_SCHEMA_VERSION: u16 = 1;

    pub fn new(active: CompositionGenerationId, previous: Option<CompositionGenerationId>) -> Self {
        Self {
            pointer_schema_version: Self::POINTER_SCHEMA_VERSION,
            active,
            previous,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompositionGenerationLoadStatus {
    Active,
    RecoveredLastGood,
}
