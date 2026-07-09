//! Typed adaptive intent. Hosts own topology-aware transaction materialization.

use ui_composition::{MountedUnitId, RegionId, StateRevision};

use crate::AdaptivePresentationMode;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AdaptiveProposalId(pub u64);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AdaptiveEditClassification {
    TransientAdaptive,
    StructuralTransaction,
    PromotionCandidate,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DockZone {
    Center,
    Left,
    Right,
    Top,
    Bottom,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AdaptiveProposalKind {
    DockUnit {
        unit: MountedUnitId,
        target_region: RegionId,
        ordinal: usize,
        zone: DockZone,
    },
    ResizeSplit {
        region: RegionId,
        fraction_basis_points: u16,
    },
    Reflow {
        region: RegionId,
        mode: AdaptivePresentationMode,
    },
    SetDrawerOpen {
        region: RegionId,
        open: bool,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AdaptiveProposal {
    pub id: AdaptiveProposalId,
    pub source_revision: StateRevision,
    pub classification: AdaptiveEditClassification,
    pub kind: AdaptiveProposalKind,
}

impl AdaptiveProposal {
    pub fn transient(
        id: AdaptiveProposalId,
        source_revision: StateRevision,
        kind: AdaptiveProposalKind,
    ) -> Self {
        Self {
            id,
            source_revision,
            classification: AdaptiveEditClassification::TransientAdaptive,
            kind,
        }
    }

    pub fn structural(
        id: AdaptiveProposalId,
        source_revision: StateRevision,
        kind: AdaptiveProposalKind,
    ) -> Self {
        Self {
            id,
            source_revision,
            classification: AdaptiveEditClassification::StructuralTransaction,
            kind,
        }
    }

    pub const fn requires_host_transaction(&self) -> bool {
        matches!(
            self.classification,
            AdaptiveEditClassification::StructuralTransaction
        )
    }
}
