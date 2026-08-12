#![allow(clippy::redundant_pub_crate)]

mod arena;
mod capabilities;
mod capability_cache;
mod diagnostics;
mod inspection;
mod interaction;
mod invalidation;
mod lifecycle;
#[cfg(test)]
mod m5a_tests;
mod namespace;
mod node;
mod reconcile;
mod routing;
mod semantic;
#[cfg(test)]
mod semantic_publication_plan;
mod tree;

pub(crate) use capability_cache::{CachedCapability, CachedSemanticContribution, CapabilityCaches};
pub use diagnostics::{DuplicateIdentityKind, IdentityDiagnostic};
pub(crate) use interaction::InteractionState;
pub use interaction::InteractionStateRef;
pub(crate) use invalidation::{DirtyPhases, apply_invalidation, publication_is_dirty};
pub use node::{MountedNodeRef, MountedTreeIndex};
pub(crate) use reconcile::{PlannedInvalidation, PlannedLifetimeReason};
pub(crate) use routing::RouteBuildError;
pub use runenui_core::{MountedNodeId, SemanticNodeId};
pub use tree::AutomationMatchDiagnostic;
pub(crate) use tree::{
    AutomationResolution, MountedIdentityExhausted, MountedTree, ReconcileStats, TargetStatus,
};
