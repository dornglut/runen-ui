//! Opaque runtime-local protocol identity and logical values.

use core::{
    fmt,
    hash::{Hash, Hasher},
    num::NonZeroU64,
};
use std::{error::Error, sync::Arc, time::Duration};

#[derive(Debug)]
struct RuntimeNamespaceMarker;

/// Opaque namespace shared by protocol identities issued by one runtime.
#[doc(hidden)]
#[derive(Clone)]
pub struct RuntimeNamespace {
    marker: Arc<RuntimeNamespaceMarker>,
}

impl RuntimeNamespace {
    /// Creates a new unrelated namespace for the runtime bridge.
    #[doc(hidden)]
    #[must_use]
    pub fn __runtime_new() -> Self {
        Self {
            marker: Arc::new(RuntimeNamespaceMarker),
        }
    }

    fn same_as(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.marker, &other.marker)
    }

    #[doc(hidden)]
    #[must_use]
    pub fn __runtime_mounted_id(&self, slot: u32, generation: u64) -> MountedNodeId {
        MountedNodeId {
            namespace: self.clone(),
            slot,
            generation,
        }
    }

    #[doc(hidden)]
    #[must_use]
    pub fn __runtime_semantic_id(&self, slot: u32, generation: u64) -> SemanticNodeId {
        SemanticNodeId {
            namespace: self.clone(),
            slot,
            generation,
        }
    }

    #[doc(hidden)]
    #[must_use]
    pub fn __runtime_surface_id(&self, slot: u32, generation: u64) -> SurfaceId {
        SurfaceId {
            namespace: self.clone(),
            slot,
            generation,
        }
    }

    #[doc(hidden)]
    #[must_use]
    pub fn __runtime_surface_context(
        &self,
        surface: SurfaceId,
        coordinate_revision: u64,
        hit_test_generation: u64,
    ) -> Option<SurfaceInputContext> {
        surface
            .namespace
            .same_as(self)
            .then(|| SurfaceInputContext {
                namespace: self.clone(),
                surface,
                coordinate_revision,
                hit_test_generation,
            })
    }

    #[doc(hidden)]
    #[must_use]
    pub fn __runtime_mounted_parts(&self, id: &MountedNodeId) -> Option<(u32, u64)> {
        id.namespace
            .same_as(self)
            .then_some((id.slot, id.generation))
    }

    #[doc(hidden)]
    #[must_use]
    pub fn __runtime_surface_parts(&self, id: &SurfaceId) -> Option<(u32, u64)> {
        id.namespace
            .same_as(self)
            .then_some((id.slot, id.generation))
    }

    #[doc(hidden)]
    #[must_use]
    pub fn __runtime_surface_context_parts(
        &self,
        context: &SurfaceInputContext,
    ) -> Option<(SurfaceId, u64, u64)> {
        context.namespace.same_as(self).then(|| {
            (
                context.surface.clone(),
                context.coordinate_revision,
                context.hit_test_generation,
            )
        })
    }
}

impl fmt::Debug for RuntimeNamespace {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("RuntimeNamespace { .. }")
    }
}

macro_rules! runtime_id {
    ($name:ident, $description:literal) => {
        #[doc = $description]
        #[derive(Clone)]
        pub struct $name {
            namespace: RuntimeNamespace,
            slot: u32,
            generation: u64,
        }

        impl PartialEq for $name {
            fn eq(&self, other: &Self) -> bool {
                self.namespace.same_as(&other.namespace)
                    && self.slot == other.slot
                    && self.generation == other.generation
            }
        }

        impl Eq for $name {}

        impl Hash for $name {
            fn hash<H: Hasher>(&self, state: &mut H) {
                Arc::as_ptr(&self.namespace.marker).hash(state);
                self.slot.hash(state);
                self.generation.hash(state);
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter
                    .debug_struct(stringify!($name))
                    .finish_non_exhaustive()
            }
        }
    };
}

runtime_id!(
    MountedNodeId,
    "Opaque runtime-local identity for one exact mounted widget lifetime."
);
runtime_id!(
    SemanticNodeId,
    "Distinct mounted-lifetime semantic identity sharing the runtime namespace."
);
runtime_id!(
    SurfaceId,
    "Opaque runtime-local identity for one logical surface lifetime."
);

/// Opaque runtime-issued context for one exact displayed hit-test generation.
#[derive(Clone)]
pub struct SurfaceInputContext {
    namespace: RuntimeNamespace,
    surface: SurfaceId,
    coordinate_revision: u64,
    hit_test_generation: u64,
}

impl SurfaceInputContext {
    /// Returns the logical surface identity carried by this context.
    #[must_use]
    pub const fn surface_id(&self) -> &SurfaceId {
        &self.surface
    }

    /// Returns the coordinate-space revision associated with this snapshot.
    #[must_use]
    pub const fn coordinate_revision(&self) -> u64 {
        self.coordinate_revision
    }

    /// Returns the exact displayed hit-test generation.
    #[must_use]
    pub const fn hit_test_generation(&self) -> u64 {
        self.hit_test_generation
    }
}

impl PartialEq for SurfaceInputContext {
    fn eq(&self, other: &Self) -> bool {
        self.namespace.same_as(&other.namespace)
            && self.surface == other.surface
            && self.coordinate_revision == other.coordinate_revision
            && self.hit_test_generation == other.hit_test_generation
    }
}

impl Eq for SurfaceInputContext {}

impl Hash for SurfaceInputContext {
    fn hash<H: Hasher>(&self, state: &mut H) {
        Arc::as_ptr(&self.namespace.marker).hash(state);
        self.surface.hash(state);
        self.coordinate_revision.hash(state);
        self.hit_test_generation.hash(state);
    }
}

impl fmt::Debug for SurfaceInputContext {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SurfaceInputContext")
            .field("surface", &self.surface)
            .field("coordinate_revision", &self.coordinate_revision)
            .field("hit_test_generation", &self.hit_test_generation)
            .finish_non_exhaustive()
    }
}

/// Runtime-relative monotonic nanosecond instant.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct MonotonicInstant(u64);

impl MonotonicInstant {
    pub const ZERO: Self = Self(0);

    #[must_use]
    pub const fn as_nanos(self) -> u64 {
        self.0
    }

    #[doc(hidden)]
    #[must_use]
    pub const fn __runtime_from_nanos(nanos: u64) -> Self {
        Self(nanos)
    }

    /// Adds a duration without wrapping.
    ///
    /// # Errors
    ///
    /// Returns [`MonotonicTimeError::Overflow`] when the duration or result
    /// cannot be represented.
    pub fn checked_add(self, duration: Duration) -> Result<Self, MonotonicTimeError> {
        let nanos = u64::try_from(duration.as_nanos()).map_err(|_| MonotonicTimeError::Overflow)?;
        self.0
            .checked_add(nanos)
            .map(Self)
            .ok_or(MonotonicTimeError::Overflow)
    }
}

/// Failure to represent a checked monotonic time operation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MonotonicTimeError {
    Overflow,
}

impl fmt::Display for MonotonicTimeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("monotonic time overflow")
    }
}

impl Error for MonotonicTimeError {}

/// Non-wrapping sequence assigned by the runtime to accepted canonical work.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct WorkSequence(NonZeroU64);

impl WorkSequence {
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0.get()
    }

    #[doc(hidden)]
    #[must_use]
    pub const fn __runtime_new(value: NonZeroU64) -> Self {
        Self(value)
    }
}

#[cfg(test)]
mod tests {
    use std::{
        collections::hash_map::DefaultHasher,
        hash::{Hash, Hasher},
    };

    use super::{MountedNodeId, RuntimeNamespace, SurfaceInputContext};

    fn hash(value: &MountedNodeId) -> u64 {
        let mut hasher = DefaultHasher::new();
        value.hash(&mut hasher);
        hasher.finish()
    }

    #[test]
    fn mounted_identity_includes_namespace_slot_and_generation() {
        let a = RuntimeNamespace::__runtime_new();
        let b = RuntimeNamespace::__runtime_new();
        let id = a.__runtime_mounted_id(1, 2);
        assert_eq!(id, a.__runtime_mounted_id(1, 2));
        assert_ne!(id, b.__runtime_mounted_id(1, 2));
        assert_ne!(id, a.__runtime_mounted_id(2, 2));
        assert_ne!(id, a.__runtime_mounted_id(1, 3));
        assert_eq!(hash(&id), hash(&a.__runtime_mounted_id(1, 2)));
        assert_eq!(format!("{id:?}"), "MountedNodeId { .. }");
    }

    #[test]
    fn surface_context_reuses_one_checked_namespace() {
        let a = RuntimeNamespace::__runtime_new();
        let b = RuntimeNamespace::__runtime_new();
        let surface = a.__runtime_surface_id(0, 1);
        let context = a
            .__runtime_surface_context(surface.clone(), 4, 7)
            .unwrap_or_else(|| unreachable!("matching surface namespace is accepted"));
        assert_eq!(context.surface_id(), &surface);
        assert_eq!(context.coordinate_revision(), 4);
        assert_eq!(context.hit_test_generation(), 7);
        assert_eq!(
            a.__runtime_surface_context_parts(&context),
            Some((surface, 4, 7))
        );
        assert!(b.__runtime_surface_context_parts(&context).is_none());
        assert_eq!(
            format!("{context:?}"),
            "SurfaceInputContext { surface: SurfaceId { .. }, coordinate_revision: 4, hit_test_generation: 7, .. }"
        );
    }

    #[test]
    fn surface_context_rejects_a_surface_from_another_namespace() {
        let a = RuntimeNamespace::__runtime_new();
        let b = RuntimeNamespace::__runtime_new();
        let foreign = b.__runtime_surface_id(0, 1);
        let context: Option<SurfaceInputContext> = a.__runtime_surface_context(foreign, 1, 1);
        assert!(context.is_none());
    }
}
