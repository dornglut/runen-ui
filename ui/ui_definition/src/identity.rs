//! Authored UI identity types.

use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct AuthoredId(pub String);

impl AuthoredId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for AuthoredId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl fmt::Display for AuthoredId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

pub type UiTemplateId = AuthoredId;
pub type UiNodeId = AuthoredId;
pub type UiRouteSlotId = AuthoredId;
pub type UiAvailabilityId = AuthoredId;
pub type UiValueSlotId = AuthoredId;
pub type UiCollectionSlotId = AuthoredId;
pub type UiSelectionSlotId = AuthoredId;
pub type UiMenuSlotId = AuthoredId;
pub type UiEmbedSlotId = AuthoredId;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct AuthoredUiNodePath(pub String);

impl AuthoredUiNodePath {
    pub fn root(id: &UiNodeId) -> Self {
        Self(id.as_str().to_string())
    }

    pub fn child(&self, id: &UiNodeId) -> Self {
        Self(format!("{}/{}", self.0, id.as_str()))
    }

    pub fn repeated_child(
        &self,
        _repeat_id: &UiNodeId,
        item_key: &str,
        child_id: &UiNodeId,
    ) -> Self {
        Self(format!("{}[{}]/{}", self.0, item_key, child_id.as_str()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}
