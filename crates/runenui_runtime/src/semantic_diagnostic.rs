//! Surface-scoped semantic publication diagnostics.

use runenui_core::{ElementId, SemanticContributionError, SemanticKey, SurfaceId};

use crate::{SemanticNodeId, semantic_compositor::SemanticCompositionDiagnostic};

/// Deterministic semantic diagnostic product for one exact logical surface.
///
/// Diagnostics are publication-side observations rather than semantic revision
/// identity. A diagnostics-only change therefore does not require a new semantic
/// snapshot revision.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SemanticDiagnosticReport {
    surface: SurfaceId,
    diagnostics: Vec<SemanticDiagnostic>,
}

impl SemanticDiagnosticReport {
    pub(crate) const fn new(surface: SurfaceId, diagnostics: Vec<SemanticDiagnostic>) -> Self {
        Self {
            surface,
            diagnostics,
        }
    }

    /// Returns the exact logical surface owning these diagnostics.
    #[must_use]
    pub const fn surface_id(&self) -> &SurfaceId {
        &self.surface
    }

    /// Returns diagnostics in deterministic publication order.
    #[must_use]
    pub const fn diagnostics(&self) -> &[SemanticDiagnostic] {
        self.diagnostics.as_slice()
    }

    /// Returns whether this publication has no semantic diagnostics.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.diagnostics.is_empty()
    }
}

/// Deterministic renderer- and platform-neutral semantic publication diagnostic.
#[non_exhaustive]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SemanticDiagnostic {
    MissingOwnerBinding {
        key: SemanticKey,
    },
    MissingMountedOwner,
    MissingLocalRelationshipTarget {
        source: SemanticNodeId,
        key: SemanticKey,
    },
    MissingAuthoredRelationshipOwner {
        source: SemanticNodeId,
        element_id: ElementId,
    },
    AmbiguousAuthoredRelationshipOwner {
        source: SemanticNodeId,
        element_id: ElementId,
    },
    MissingAuthoredRelationshipTarget {
        source: SemanticNodeId,
        element_id: ElementId,
        key: SemanticKey,
    },
    FocusedOwnerMissingVisiblePrimary,
    OwnerWithdrawn {
        authored_id: Option<ElementId>,
        reason: SemanticOwnerWithdrawalReason,
    },
}

/// Exact fail-closed reason for withdrawing one mounted owner's semantic product.
#[non_exhaustive]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SemanticOwnerWithdrawalReason {
    InvalidContribution(SemanticContributionError),
    IdentityExhausted,
    IndexIntegrityFailure,
    StatePayloadMismatch,
}

impl From<SemanticCompositionDiagnostic> for SemanticDiagnostic {
    fn from(value: SemanticCompositionDiagnostic) -> Self {
        match value {
            SemanticCompositionDiagnostic::MissingOwnerBinding { key } => {
                Self::MissingOwnerBinding { key }
            }
            SemanticCompositionDiagnostic::MissingMountedOwner => Self::MissingMountedOwner,
            SemanticCompositionDiagnostic::MissingLocalRelationshipTarget { source, key } => {
                Self::MissingLocalRelationshipTarget { source, key }
            }
            SemanticCompositionDiagnostic::MissingAuthoredRelationshipOwner {
                source,
                element_id,
            } => Self::MissingAuthoredRelationshipOwner { source, element_id },
            SemanticCompositionDiagnostic::AmbiguousAuthoredRelationshipOwner {
                source,
                element_id,
            } => Self::AmbiguousAuthoredRelationshipOwner { source, element_id },
            SemanticCompositionDiagnostic::MissingAuthoredRelationshipTarget {
                source,
                element_id,
                key,
            } => Self::MissingAuthoredRelationshipTarget {
                source,
                element_id,
                key,
            },
            SemanticCompositionDiagnostic::FocusedOwnerMissingVisiblePrimary => {
                Self::FocusedOwnerMissingVisiblePrimary
            }
        }
    }
}
