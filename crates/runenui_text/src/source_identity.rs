use core::{
    fmt,
    hash::{Hash, Hasher},
    ptr,
};
use std::sync::Arc;

use crate::FontSourceRevision;

/// Opaque process-local identity for one immutable font-source universe.
///
/// Two text systems can have the same source policy and numeric revision while observing
/// different system-font snapshots. This identity prevents those universes from aliasing in text
/// cache compatibility without exposing platform font-enumeration details.
#[derive(Clone)]
pub struct FontSourceIdentity(Arc<FontSourceIdentityMarker>);

struct FontSourceIdentityMarker;

impl FontSourceIdentity {
    pub(crate) fn fresh() -> Self {
        Self(Arc::new(FontSourceIdentityMarker))
    }
}

impl fmt::Debug for FontSourceIdentity {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("FontSourceIdentity(opaque)")
    }
}

impl PartialEq for FontSourceIdentity {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0)
    }
}

impl Eq for FontSourceIdentity {}

impl Hash for FontSourceIdentity {
    fn hash<H: Hasher>(&self, state: &mut H) {
        ptr::hash(Arc::as_ptr(&self.0), state);
    }
}

/// Exact cache-compatibility identity of one font-source snapshot.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct FontSourceSnapshot {
    identity: FontSourceIdentity,
    revision: FontSourceRevision,
}

impl FontSourceSnapshot {
    pub(crate) fn new(identity: FontSourceIdentity, revision: FontSourceRevision) -> Self {
        Self { identity, revision }
    }

    /// Returns the opaque identity of the font-source universe.
    #[must_use]
    pub const fn identity(&self) -> &FontSourceIdentity {
        &self.identity
    }

    /// Returns the monotonic revision within that font-source universe.
    #[must_use]
    pub const fn revision(&self) -> FontSourceRevision {
        self.revision
    }
}
