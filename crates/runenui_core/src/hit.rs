//! Owner-local physical hit contribution vocabulary.

use crate::{
    ContributionClip, LogicalRect, LogicalSize, LogicalTransform, Radius, SceneLayer, SceneShape,
};

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
    pub const fn new(regions: Vec<HitRegion>) -> Self {
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

/// Physical policy applied by the first containing region in topmost hit order.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub enum PointerPolicy {
    /// Resolve the region to its runtime-injected mounted owner.
    #[default]
    Target,
    /// Terminate physical hit testing without a mounted target.
    Block,
}

/// One self-contained owner-local physical hit region.
///
/// Shape, transform, conjunctive clips, ordering layer, and pointer policy are
/// explicit values. Omission is the only pass-through representation; a present
/// [`PointerPolicy::Block`] region is intentionally occluding.
#[derive(Clone, Debug, PartialEq)]
pub struct HitRegion {
    shape: SceneShape,
    local_transform: LogicalTransform,
    clips: Vec<ContributionClip>,
    layer: SceneLayer,
    pointer_policy: PointerPolicy,
}

impl HitRegion {
    const fn from_shape(shape: SceneShape) -> Self {
        Self {
            shape,
            local_transform: LogicalTransform::IDENTITY,
            clips: Vec::new(),
            layer: SceneLayer::ZERO,
            pointer_policy: PointerPolicy::Target,
        }
    }

    /// Creates one owner-local logical rectangle region.
    #[must_use]
    pub const fn rect(rect: LogicalRect) -> Self {
        Self::from_shape(SceneShape::rect(rect))
    }

    /// Creates one owner-local rounded-rectangle region.
    #[must_use]
    pub const fn rounded_rect(rect: LogicalRect, radius: Radius) -> Self {
        Self::from_shape(SceneShape::rounded_rect(rect, radius))
    }

    /// Replaces the region-local to owner-local transform.
    #[must_use]
    pub const fn with_transform(mut self, transform: LogicalTransform) -> Self {
        self.local_transform = transform;
        self
    }

    /// Appends one conjunctive owner-local clip.
    #[must_use]
    pub fn with_clip(mut self, clip: ContributionClip) -> Self {
        self.clips.push(clip);
        self
    }

    /// Replaces the snapshot-local ordering layer.
    #[must_use]
    pub const fn with_layer(mut self, layer: SceneLayer) -> Self {
        self.layer = layer;
        self
    }

    /// Replaces the first-containing pointer policy.
    #[must_use]
    pub const fn with_pointer_policy(mut self, pointer_policy: PointerPolicy) -> Self {
        self.pointer_policy = pointer_policy;
        self
    }

    /// Returns the owner-local logical shape.
    #[must_use]
    pub const fn shape(&self) -> SceneShape {
        self.shape
    }

    /// Returns region-local to owner-local transform.
    #[must_use]
    pub const fn local_transform(&self) -> LogicalTransform {
        self.local_transform
    }

    /// Returns conjunctive clips in authored order.
    #[must_use]
    pub const fn clips(&self) -> &[ContributionClip] {
        self.clips.as_slice()
    }

    /// Returns the snapshot-local ordering layer.
    #[must_use]
    pub const fn layer(&self) -> SceneLayer {
        self.layer
    }

    /// Returns the first-containing pointer policy.
    #[must_use]
    pub const fn pointer_policy(&self) -> PointerPolicy {
        self.pointer_policy
    }
}

#[cfg(test)]
mod tests {
    use super::{HitContribution, HitRegion, PointerPolicy};
    use crate::{
        ContributionClip, LogicalLength, LogicalRect, LogicalTransform, Radius, SceneLayer,
        SceneShape,
    };

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
        assert_eq!(contribution.regions()[0].shape(), SceneShape::rect(rect));
    }

    #[test]
    fn region_composition_defaults_and_explicit_values_are_self_contained() {
        let rect = LogicalRect::try_new(0.0, 0.0, 10.0, 12.0)
            .unwrap_or_else(|_| unreachable!("test rectangle is valid"));
        let default_region = HitRegion::rect(rect);
        assert_eq!(default_region.shape(), SceneShape::rect(rect));
        assert_eq!(default_region.local_transform(), LogicalTransform::IDENTITY);
        assert!(default_region.clips().is_empty());
        assert_eq!(default_region.layer(), SceneLayer::ZERO);
        assert_eq!(default_region.pointer_policy(), PointerPolicy::Target);

        let transform = LogicalTransform::translation(2.0, 3.0)
            .unwrap_or_else(|_| unreachable!("test transform is valid"));
        let clip = ContributionClip::identity(SceneShape::rect(rect));
        let radius = Radius::all(
            LogicalLength::new(2.0).unwrap_or_else(|_| unreachable!("test radius is valid")),
        );
        let region = HitRegion::rounded_rect(rect, radius)
            .with_transform(transform)
            .with_clip(clip)
            .with_layer(SceneLayer::new(-4))
            .with_pointer_policy(PointerPolicy::Block);

        assert_eq!(region.shape(), SceneShape::rounded_rect(rect, radius));
        assert_eq!(region.local_transform(), transform);
        assert_eq!(region.clips(), &[clip]);
        assert_eq!(region.layer(), SceneLayer::new(-4));
        assert_eq!(region.pointer_policy(), PointerPolicy::Block);
    }
}
