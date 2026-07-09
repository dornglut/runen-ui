//! Drag and resize sessions over shared immutable projection storage.

use std::sync::Arc;

use ui_composition::{MountedUnitId, RegionId, SplitFraction};
use ui_input::{SemanticActionEvent, UiSemanticAction};
use ui_math::{UiPoint, UiRect};

use crate::{
    AdaptiveCompositionRejection, AdaptiveDiagnosticCode as Code,
    AdaptiveDiagnosticRecord as Record, AdaptiveDiagnosticStage as Stage,
    AdaptiveDiagnosticSubject as Subject, AdaptiveProjectionState, AdaptiveProposal,
    AdaptiveProposalId, AdaptiveProposalKind, DockZone, RegionHitIndex,
};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct DragFrameMetrics {
    pub full_graph_clones: usize,
    pub changed_regions: usize,
    pub bounded_allocation_units: usize,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PreviewProjection {
    pub region: RegionId,
    pub bounds: UiRect,
    pub zone: DockZone,
}

#[derive(Clone, Debug)]
pub struct DragSession {
    base: Arc<AdaptiveProjectionState>,
    hit_index: RegionHitIndex,
    unit: MountedUnitId,
    preview: Option<PreviewProjection>,
    proposal: Option<AdaptiveProposal>,
    metrics: DragFrameMetrics,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SessionSemanticOutcome {
    Ignored,
    ModeAccepted,
    CommitRequested,
    Cancelled,
}

impl DragSession {
    pub fn begin(base: Arc<AdaptiveProjectionState>, unit: MountedUnitId) -> Self {
        let hit_index = RegionHitIndex::new(base.shared_regions());
        Self {
            base,
            hit_index,
            unit,
            preview: None,
            proposal: None,
            metrics: DragFrameMetrics::default(),
        }
    }

    pub fn update_pointer(&mut self, point: UiPoint) -> Option<&PreviewProjection> {
        let region = self.hit_index.hit_test(point)?;
        let projected = self.base.region(region)?;
        let zone = dock_zone(projected.bounds, point);
        self.preview = Some(PreviewProjection {
            region,
            bounds: preview_bounds(projected.bounds, zone),
            zone,
        });
        self.proposal = Some(AdaptiveProposal::structural(
            AdaptiveProposalId(1),
            self.base.source_revision(),
            AdaptiveProposalKind::DockUnit {
                unit: self.unit,
                target_region: region,
                ordinal: 0,
                zone,
            },
        ));
        self.metrics = DragFrameMetrics {
            full_graph_clones: 0,
            changed_regions: 1,
            bounded_allocation_units: 1,
        };
        self.preview.as_ref()
    }

    pub fn commit(self) -> Option<AdaptiveProposal> {
        self.proposal
    }

    pub fn handle_semantic_action(&mut self, event: SemanticActionEvent) -> SessionSemanticOutcome {
        match event.action {
            UiSemanticAction::EnterMoveMode | UiSemanticAction::Activate => {
                SessionSemanticOutcome::ModeAccepted
            }
            UiSemanticAction::Commit if self.proposal.is_some() => {
                SessionSemanticOutcome::CommitRequested
            }
            UiSemanticAction::Cancel | UiSemanticAction::Rollback => {
                self.preview = None;
                self.proposal = None;
                SessionSemanticOutcome::Cancelled
            }
            UiSemanticAction::Focus(_)
            | UiSemanticAction::CycleTab(_)
            | UiSemanticAction::EnterResizeMode(_)
            | UiSemanticAction::Commit => SessionSemanticOutcome::Ignored,
        }
    }

    pub fn cancel(mut self) -> Option<AdaptiveProposal> {
        self.preview = None;
        self.proposal = None;
        None
    }

    pub const fn metrics(&self) -> DragFrameMetrics {
        self.metrics
    }

    pub fn shared_region_count(&self) -> usize {
        self.hit_index.shared_region_count()
    }
}

#[derive(Clone, Debug)]
pub struct ResizeSession {
    base: Arc<AdaptiveProjectionState>,
    split: RegionId,
    candidate: Option<SplitFraction>,
}

impl ResizeSession {
    pub fn begin(base: Arc<AdaptiveProjectionState>, split: RegionId) -> Self {
        Self {
            base,
            split,
            candidate: None,
        }
    }

    pub fn update_fraction(
        &mut self,
        basis_points: u16,
    ) -> Result<(), AdaptiveCompositionRejection> {
        self.candidate = Some(SplitFraction::try_new(basis_points).map_err(|_| {
            AdaptiveCompositionRejection::single(Record::error(
                Code::ProposalInvalid,
                Stage::Preview,
                Subject::Region(self.split),
                "Resize with a split fraction in 1..=9999 basis points.",
            ))
        })?);
        Ok(())
    }

    pub fn commit(self) -> Option<AdaptiveProposal> {
        let fraction = self.candidate?;
        Some(AdaptiveProposal::structural(
            AdaptiveProposalId(2),
            self.base.source_revision(),
            AdaptiveProposalKind::ResizeSplit {
                region: self.split,
                fraction_basis_points: fraction.basis_points(),
            },
        ))
    }

    pub fn cancel(self) -> Option<AdaptiveProposal> {
        None
    }
}

fn dock_zone(bounds: UiRect, point: UiPoint) -> DockZone {
    let x = if bounds.width > 0.0 {
        (point.x - bounds.x) / bounds.width
    } else {
        0.5
    };
    let y = if bounds.height > 0.0 {
        (point.y - bounds.y) / bounds.height
    } else {
        0.5
    };
    if x < 0.25 {
        DockZone::Left
    } else if x > 0.75 {
        DockZone::Right
    } else if y < 0.25 {
        DockZone::Top
    } else if y > 0.75 {
        DockZone::Bottom
    } else {
        DockZone::Center
    }
}

fn preview_bounds(bounds: UiRect, zone: DockZone) -> UiRect {
    match zone {
        DockZone::Center => bounds,
        DockZone::Left => UiRect::new(bounds.x, bounds.y, bounds.width * 0.5, bounds.height),
        DockZone::Right => UiRect::new(
            bounds.x + bounds.width * 0.5,
            bounds.y,
            bounds.width * 0.5,
            bounds.height,
        ),
        DockZone::Top => UiRect::new(bounds.x, bounds.y, bounds.width, bounds.height * 0.5),
        DockZone::Bottom => UiRect::new(
            bounds.x,
            bounds.y + bounds.height * 0.5,
            bounds.width,
            bounds.height * 0.5,
        ),
    }
}
