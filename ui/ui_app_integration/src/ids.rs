//! Stable IDs for the app-integration proof bridge.

use serde::{Deserialize, Serialize};

fn validate_namespaced(kind: &'static str, value: String) -> String {
    assert!(!value.is_empty(), "{kind} IDs must not be empty");
    assert!(value.contains('.'), "{kind} IDs must be namespaced");
    assert!(
        value
            .chars()
            .all(|character| character.is_ascii_alphanumeric()
                || matches!(character, '.' | '_' | '-')),
        "{kind} IDs must contain only ASCII alphanumerics, '.', '_' or '-'"
    );
    value
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct UiAppScreenId(String);

impl UiAppScreenId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(validate_namespaced("screen", value.into()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct UiAppActionId(String);

impl UiAppActionId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(validate_namespaced("action", value.into()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct UiAppRouteBindingId(String);

impl UiAppRouteBindingId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(validate_namespaced("route binding", value.into()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct UiAppProofId(String);

impl UiAppProofId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(validate_namespaced("proof", value.into()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}
