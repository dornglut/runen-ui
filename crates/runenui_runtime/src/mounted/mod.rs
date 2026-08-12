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
mod semantic_publication_plan;
mod surface_publication_plan;
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
pub(crate) use semantic::SemanticReconcileError;
pub(crate) use semantic_publication_plan::{FinalizedSemanticPublication, SemanticMountedCommit};
pub(crate) use surface_publication_plan::SurfaceCapabilityPlan;
pub use tree::AutomationMatchDiagnostic;
pub(crate) use tree::{
    AutomationResolution, MountedIdentityExhausted, MountedTree, ReconcileStats, TargetStatus,
};
