#![allow(clippy::redundant_pub_crate)]

use core::{
    fmt,
    hash::{Hash, Hasher},
};
use std::sync::Arc;

#[derive(Debug)]
pub(crate) struct RuntimeInstanceMarker;

macro_rules! runtime_id {
    ($name:ident) => {
        #[derive(Clone)]
        pub struct $name {
            pub(crate) runtime: Arc<RuntimeInstanceMarker>,
            pub(crate) slot: usize,
            pub(crate) generation: u64,
        }

        impl $name {
            pub(crate) fn new(
                runtime: &Arc<RuntimeInstanceMarker>,
                slot: usize,
                generation: u64,
            ) -> Self {
                Self {
                    runtime: Arc::clone(runtime),
                    slot,
                    generation,
                }
            }

            #[allow(dead_code)]
            pub(crate) fn belongs_to(&self, runtime: &Arc<RuntimeInstanceMarker>) -> bool {
                Arc::ptr_eq(&self.runtime, runtime)
            }
        }

        impl PartialEq for $name {
            fn eq(&self, other: &Self) -> bool {
                Arc::ptr_eq(&self.runtime, &other.runtime)
                    && self.slot == other.slot
                    && self.generation == other.generation
            }
        }
        impl Eq for $name {}

        impl Hash for $name {
            fn hash<H: Hasher>(&self, state: &mut H) {
                Arc::as_ptr(&self.runtime).hash(state);
                self.slot.hash(state);
                self.generation.hash(state);
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter
                    .debug_struct(stringify!($name))
                    .field("runtime", &Arc::as_ptr(&self.runtime))
                    .field("slot", &self.slot)
                    .field("generation", &self.generation)
                    .finish()
            }
        }
    };
}

// Process-local, runtime-instance-local generational mounted identity.
runtime_id!(MountedNodeId);

// Distinct process-local semantic identity for one mounted lifetime.
runtime_id!(SemanticNodeId);

#[cfg(test)]
mod tests {
    use super::{MountedNodeId, RuntimeInstanceMarker};
    use std::{
        collections::hash_map::DefaultHasher,
        hash::{Hash, Hasher},
        sync::Arc,
    };

    fn hash(value: &MountedNodeId) -> u64 {
        let mut h = DefaultHasher::new();
        value.hash(&mut h);
        h.finish()
    }

    #[test]
    fn equality_and_hash_include_runtime_slot_and_generation() {
        let a = Arc::new(RuntimeInstanceMarker);
        let b = Arc::new(RuntimeInstanceMarker);
        let id = MountedNodeId::new(&a, 1, 2);
        assert_eq!(id, MountedNodeId::new(&a, 1, 2));
        assert_ne!(id, MountedNodeId::new(&b, 1, 2));
        assert_ne!(id, MountedNodeId::new(&a, 2, 2));
        assert_ne!(id, MountedNodeId::new(&a, 1, 3));
        assert_eq!(hash(&id), hash(&MountedNodeId::new(&a, 1, 2)));
    }
}
