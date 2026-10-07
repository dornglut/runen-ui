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


/// Generic pointer policy for input outside one live presentation interaction family.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum PresentationOutsidePointerPolicy {
    #[default]
    Ignore,
    Block,
    DismissAndBlock,
}

/// Focus behavior when one exact presentation lifetime becomes active.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum PresentationFocusPolicy {
    #[default]
    Preserve,
    EnterAndRestore,
}

/// Authored entry preference for focus selection within one presentation lifetime.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum PresentationFocusEntry {
    #[default]
    Automatic,
    Preferred,
}

/// Bounded reason carried by one generic presentation-dismiss semantic request.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum PresentationDismissReason {
    OutsidePointer,
    CancelOrBack,
    AnchorUnavailable,
}

/// Non-empty ordered same-surface presentation placement request.
///
/// The first candidate is mandatory by construction. Fallbacks retain authored
/// order; runtime never remembers or reorders candidates.
#[derive(Clone, Debug, PartialEq)]
pub struct SurfacePresentation {
    anchor: SurfacePresentationAnchor,
    candidates: Vec<SurfacePresentationPlacement>,
    outside_pointer: PresentationOutsidePointerPolicy,
    modal: bool,
    dismiss_on_cancel_or_back: bool,
    focus: PresentationFocusPolicy,
}

impl SurfacePresentation {
    #[must_use]
    pub fn new(first: SurfacePresentationPlacement) -> Self {
        Self {
            anchor: SurfacePresentationAnchor::OwnerBounds,
            candidates: vec![first],
            outside_pointer: PresentationOutsidePointerPolicy::Ignore,
            modal: false,
            dismiss_on_cancel_or_back: false,
            focus: PresentationFocusPolicy::Preserve,
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
    pub const fn with_outside_pointer(mut self, policy: PresentationOutsidePointerPolicy) -> Self {
        self.outside_pointer = policy;
        self
    }

    #[must_use]
    pub const fn modal(mut self, modal: bool) -> Self {
        self.modal = modal;
        self
    }

    #[must_use]
    pub const fn dismiss_on_cancel_or_back(mut self, dismiss: bool) -> Self {
        self.dismiss_on_cancel_or_back = dismiss;
        self
    }

    #[must_use]
    pub const fn with_focus_policy(mut self, policy: PresentationFocusPolicy) -> Self {
        self.focus = policy;
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

    #[must_use]
    pub const fn outside_pointer(&self) -> PresentationOutsidePointerPolicy {
        self.outside_pointer
    }

    #[must_use]
    pub const fn is_modal(&self) -> bool {
        self.modal
    }

    #[must_use]
    pub const fn dismisses_on_cancel_or_back(&self) -> bool {
        self.dismiss_on_cancel_or_back
    }

    #[must_use]
    pub const fn focus_policy(&self) -> PresentationFocusPolicy {
        self.focus
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
