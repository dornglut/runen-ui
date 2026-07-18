#![allow(clippy::redundant_pub_crate)]

mod arena;
mod capabilities;
mod capability_cache;
mod diagnostics;
mod id;
mod inspection;
mod interaction;
mod invalidation;
mod lifecycle;
mod node;
mod reconcile;
mod tree;

pub(crate) use capabilities::MountedActivationOutput;
pub(crate) use capability_cache::{CachedCapability, CapabilityCaches};
pub use diagnostics::{DuplicateIdentityKind, IdentityDiagnostic};
pub use id::{MountedNodeId, SemanticNodeId};
pub(crate) use interaction::InteractionState;
pub use interaction::InteractionStateRef;
pub(crate) use invalidation::{DirtyPhases, apply_invalidation, publication_is_dirty};
pub use node::{MountedNodeRef, MountedTreeIndex};
pub(crate) use tree::{MountedTree, TargetStatus};
