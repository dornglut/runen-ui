//! Stable element key identity.

/// Stable identity for data-backed element instances.
///
/// Keys are authored by application code and are intended for preserving runtime
/// identity across reordered list or collection children. They are separate from
/// [`crate::ElementId`], which is a public debug, test, automation, and
/// integration handle.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ElementKey(String);

impl ElementKey {
    /// Creates an element key from a string-like value.
    #[must_use]
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    /// Returns the string value of this element key.
    #[must_use]
    pub const fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

impl From<&str> for ElementKey {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for ElementKey {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl AsRef<str> for ElementKey {
    fn as_ref(&self) -> &str {
        self.as_str()
    }
}
