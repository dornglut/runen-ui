#![allow(clippy::redundant_pub_crate)]

mod arena;
mod capabilities;
mod capability_cache;
mod diagnostics;
mod inspection;
mod interaction;
mod invalidation;
mod lifecycle;
mod namespace;
mod node;
mod reconcile;
mod routing;
mod tree;

pub(crate) use capability_cache::{CachedCapability, CapabilityCaches};
pub use diagnostics::{DuplicateIdentityKind, IdentityDiagnostic};
pub(crate) use interaction::InteractionState;
pub use interaction::InteractionStateRef;
pub(crate) use invalidation::{DirtyPhases, apply_invalidation, publication_is_dirty};
pub use node::{MountedNodeRef, MountedTreeIndex};
pub(crate) use routing::RouteBuildError;
pub use runenui_core::{MountedNodeId, SemanticNodeId};
pub(crate) use tree::{MountedIdentityExhausted, MountedTree, ReconcileStats, TargetStatus};
