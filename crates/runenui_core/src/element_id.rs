//! Stable element identity.

/// Stable identity for an element in a UI tree.
///
/// IDs are authored by application code and are used later by the runtime for
/// focus, testing, tracing, accessibility, and state retention.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ElementId(String);

impl ElementId {
    /// Creates an element ID from a string-like value.
    #[must_use]
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Returns the string value of this element ID.
    #[must_use]
    pub const fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl From<&str> for ElementId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for ElementId {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl AsRef<str> for ElementId {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}
