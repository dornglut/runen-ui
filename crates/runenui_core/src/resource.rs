//! Opaque renderer-resource identity values.

use core::{
    error::Error,
    fmt,
    hash::{Hash, Hasher},
    ptr,
};
use std::sync::Arc;

/// Neutral logical renderer-resource kind carried by one [`ResourceRef`].
///
/// Kinds describe protocol requirements only. They do not select a provider,
/// backend, decoder, shaper, cache, or realization path.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ResourceKind {
    /// Immutable logical image content.
    Image,
    /// Immutable shaped glyph/coverage geometry and non-paint run metadata.
    ShapedTextRun,
}

/// Opaque identity for one externally owned immutable logical renderer resource.
///
/// Every call to [`Self::new`] issues a fresh process-local identity. Resource
/// owners retain and clone that value for as long as the same logical content is
/// live; replacing logical content requires issuing a new reference. `RunenUI`
/// intentionally exposes no provider/domain key, payload, lookup handle, cache
/// handle, or backend identifier that consumers could split or reinterpret.
#[derive(Clone)]
pub struct ResourceRef {
    kind: ResourceKind,
    identity: Arc<ResourceIdentity>,
}

struct ResourceIdentity;

impl ResourceRef {
    /// Issues a fresh opaque reference for one logical resource of `kind`.
    ///
    /// The external resource owner is responsible for preserving the immutable
    /// logical-content binding while this reference remains live.
    #[must_use]
    pub fn new(kind: ResourceKind) -> Self {
        Self {
            kind,
            identity: Arc::new(ResourceIdentity),
        }
    }

    /// Returns the neutral resource kind.
    #[must_use]
    pub const fn kind(&self) -> ResourceKind {
        self.kind
    }
}

impl fmt::Debug for ResourceRef {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("ResourceRef")
            .field("kind", &self.kind)
            .field("identity", &"opaque")
            .finish()
    }
}

impl PartialEq for ResourceRef {
    fn eq(&self, other: &Self) -> bool {
        self.kind == other.kind && Arc::ptr_eq(&self.identity, &other.identity)
    }
}

impl Eq for ResourceRef {}

impl Hash for ResourceRef {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.kind.hash(state);
        ptr::hash(Arc::as_ptr(&self.identity), state);
    }
}

/// Error returned when a resource reference has the wrong neutral kind for one
/// paint primitive.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ResourceKindMismatch {
    expected: ResourceKind,
    actual: ResourceKind,
}

impl ResourceKindMismatch {
    pub(crate) const fn new(expected: ResourceKind, actual: ResourceKind) -> Self {
        Self { expected, actual }
    }

    /// Returns the primitive-required kind.
    #[must_use]
    pub const fn expected(self) -> ResourceKind {
        self.expected
    }

    /// Returns the supplied reference kind.
    #[must_use]
    pub const fn actual(self) -> ResourceKind {
        self.actual
    }
}

impl fmt::Display for ResourceKindMismatch {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "resource kind mismatch: expected {:?}, got {:?}",
            self.expected, self.actual
        )
    }
}

impl Error for ResourceKindMismatch {}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::{ResourceKind, ResourceRef};

    #[test]
    fn cloned_reference_preserves_identity_but_fresh_issuance_never_aliases_live_identity() {
        let image = ResourceRef::new(ResourceKind::Image);
        let cloned = image.clone();
        let replacement = ResourceRef::new(ResourceKind::Image);
        let shaped = ResourceRef::new(ResourceKind::ShapedTextRun);

        assert_eq!(image, cloned);
        assert_ne!(image, replacement);
        assert_ne!(image, shaped);
        assert_eq!(image.kind(), ResourceKind::Image);

        let mut payload_fixture = HashMap::new();
        payload_fixture.insert(image, "first");
        assert_eq!(payload_fixture.get(&cloned), Some(&"first"));
        assert_eq!(payload_fixture.get(&replacement), None);
    }
}
