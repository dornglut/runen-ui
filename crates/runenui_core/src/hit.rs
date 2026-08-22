//! Owner-local physical hit contribution vocabulary.

use crate::{LogicalRect, LogicalSize};

/// Read-only facts supplied while one mounted widget contributes physical hit geometry.
///
/// The context exposes no mounted target, surface origin, semantic state, focus
/// eligibility, renderer object, or displayed-generation identity.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HitContributionContext {
    local_size: LogicalSize,
}

impl HitContributionContext {
    /// Returns the owner's final local logical size.
    #[must_use]
    pub const fn local_size(self) -> LogicalSize {
        self.local_size
    }

    #[doc(hidden)]
    #[must_use]
    pub const fn __runtime_new(local_size: LogicalSize) -> Self {
        Self { local_size }
    }
}

/// Ordered immutable physical hit fragment authored in one widget's local space.
///
/// Empty is the canonical pass-through representation. Mounted target identity,
/// displayed-snapshot membership, and surface placement are injected only by the
/// runtime when the public hit-test scene is composed.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct HitContribution {
    regions: Vec<HitRegion>,
}

impl HitContribution {
    /// Canonical pass-through contribution.
    #[must_use]
    pub const fn empty() -> Self {
        Self {
            regions: Vec::new(),
        }
    }

    /// Creates a contribution from validated owner-local regions in authored order.
    #[must_use]
    pub fn new(regions: Vec<HitRegion>) -> Self {
        Self { regions }
    }

    /// Creates one targetable owner-local rectangle contribution.
    #[must_use]
    pub fn single_rect(rect: LogicalRect) -> Self {
        Self {
            regions: vec![HitRegion::rect(rect)],
        }
    }

    /// Returns owner-local regions in exact authored order.
    #[must_use]
    pub const fn regions(&self) -> &[HitRegion] {
        self.regions.as_slice()
    }

    /// Returns whether this owner contributes no physical hit region.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.regions.is_empty()
    }
}

/// One owner-local targetable physical hit region.
///
/// M6B intentionally starts with the rectangle shape required for the canonical
/// displayed-hit cutover. Rounded shapes, explicit clips, arbitrary authored
/// transforms, layering, and blocking policy extend this same value in M6C.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HitRegion {
    rect: LogicalRect,
}

impl HitRegion {
    /// Creates one owner-local logical rectangle region.
    #[must_use]
    pub const fn rect(rect: LogicalRect) -> Self {
        Self { rect }
    }

    /// Returns the owner-local logical rectangle.
    #[must_use]
    pub const fn logical_rect(self) -> LogicalRect {
        self.rect
    }
}

#[cfg(test)]
mod tests {
    use super::{HitContribution, HitRegion};
    use crate::LogicalRect;

    #[test]
    fn empty_is_the_only_default_hit_contribution() {
        assert!(HitContribution::default().is_empty());
        assert!(HitContribution::empty().is_empty());
    }

    #[test]
    fn region_preserves_owner_local_geometry() {
        let rect = LogicalRect::try_new(1.0, 2.0, 3.0, 4.0)
            .unwrap_or_else(|_| unreachable!("test rectangle is valid"));
        let contribution = HitContribution::new(vec![HitRegion::rect(rect)]);
        assert_eq!(contribution.regions(), &[HitRegion::rect(rect)]);
    }
}
