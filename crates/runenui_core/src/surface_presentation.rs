//! Same-surface presentation authoring shared by popup-like M11 consumers.
//!
//! These values describe attachment and deterministic placement only. Mounted lifetime,
//! layout, stacking, clipping, hit testing, focus geometry, semantics, and renderer
//! publication remain runtime-owned.
use crate::{LogicalDelta, LogicalLength, LogicalPoint, LogicalRect};

/// Surface-level anchor for one projected mounted presentation subtree.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SurfacePresentationAnchor {
    /// Attach to the current final bounds of the logical mounted owner.
    OwnerBounds,
    /// Attach to one finite rectangle expressed in the logical owner's local coordinates.
    OwnerRect(LogicalRect),
    /// Attach to an exact surface-logical point.
    SurfacePoint(LogicalPoint),
    /// Attach to the complete logical surface viewport.
    SurfaceViewport,
}

/// Primary side used by one presentation placement candidate.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum SurfacePresentationSide {
    Top,
    Right,
    Bottom,
    Left,
    Center,
}

/// Cross-axis alignment for one placement candidate.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum SurfacePresentationAlignment {
    Start,
    Center,
    End,
}

/// One deterministic presentation placement candidate.
///
/// `gap` separates the presentation from the selected anchor side. `offset` is
/// then applied in surface-logical x/y coordinates. Center placement ignores
/// cross-axis alignment but retains offset.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SurfacePresentationPlacement {
    side: SurfacePresentationSide,
    alignment: SurfacePresentationAlignment,
    gap: LogicalLength,
    offset: LogicalDelta,
}

impl SurfacePresentationPlacement {
    #[must_use]
    pub const fn new(side: SurfacePresentationSide) -> Self {
        Self {
            side,
            alignment: SurfacePresentationAlignment::Center,
            gap: LogicalLength::ZERO,
            offset: LogicalDelta::ZERO,
        }
    }

    #[must_use]
    pub const fn with_alignment(mut self, alignment: SurfacePresentationAlignment) -> Self {
        self.alignment = alignment;
        self
    }

    #[must_use]
    pub const fn with_gap(mut self, gap: LogicalLength) -> Self {
        self.gap = gap;
        self
    }

    #[must_use]
    pub const fn with_offset(mut self, offset: LogicalDelta) -> Self {
        self.offset = offset;
        self
    }

    #[must_use]
    pub const fn side(self) -> SurfacePresentationSide {
        self.side
    }

    #[must_use]
    pub const fn alignment(self) -> SurfacePresentationAlignment {
        self.alignment
    }

    #[must_use]
    pub const fn gap(self) -> LogicalLength {
        self.gap
    }

    #[must_use]
    pub const fn offset(self) -> LogicalDelta {
        self.offset
    }
}

/// Non-empty ordered same-surface presentation placement request.
///
/// The first candidate is mandatory by construction. Fallbacks retain authored
/// order; runtime never remembers or reorders candidates.
#[derive(Clone, Debug, PartialEq)]
pub struct SurfacePresentation {
    anchor: SurfacePresentationAnchor,
    candidates: Vec<SurfacePresentationPlacement>,
}

impl SurfacePresentation {
    #[must_use]
    pub fn new(first: SurfacePresentationPlacement) -> Self {
        Self {
            anchor: SurfacePresentationAnchor::OwnerBounds,
            candidates: vec![first],
        }
    }

    /// Replaces the default owner-bounds anchor.
    #[must_use]
    pub const fn with_anchor(mut self, anchor: SurfacePresentationAnchor) -> Self {
        self.anchor = anchor;
        self
    }

    #[must_use]
    pub fn with_fallback(mut self, candidate: SurfacePresentationPlacement) -> Self {
        self.candidates.push(candidate);
        self
    }

    #[must_use]
    pub const fn anchor(&self) -> SurfacePresentationAnchor {
        self.anchor
    }

    #[must_use]
    pub const fn candidates(&self) -> &[SurfacePresentationPlacement] {
        self.candidates.as_slice()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn authoring_keeps_nonempty_candidate_order_and_finite_geometry() {
        let gap = LogicalLength::from(4_u8);
        let offset = LogicalDelta::new(2.0, -3.0)
            .unwrap_or_else(|_| unreachable!("fixture offset is finite"));
        let first = SurfacePresentationPlacement::new(SurfacePresentationSide::Bottom)
            .with_alignment(SurfacePresentationAlignment::Start)
            .with_gap(gap);
        let fallback = SurfacePresentationPlacement::new(SurfacePresentationSide::Top)
            .with_alignment(SurfacePresentationAlignment::End)
            .with_offset(offset);
        let authored = SurfacePresentation::new(first).with_fallback(fallback);
        assert_eq!(authored.anchor(), SurfacePresentationAnchor::OwnerBounds);
        assert_eq!(authored.candidates(), [first, fallback]);
    }
}
