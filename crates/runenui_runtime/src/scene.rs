//! Canonical immutable renderer-neutral paint and displayed-hit products.

use core::num::NonZeroU64;
use std::sync::Arc;

use runenui_core::{
    LogicalPoint, LogicalRect, LogicalSize, LogicalTransform, MountedNodeId, PaintPrimitive,
    SurfaceId, SurfaceInputContext,
};

/// One self-contained renderer-neutral paint item in stable scene order.
#[derive(Clone, Debug, PartialEq)]
pub struct PaintSceneItem {
    primitive: PaintPrimitive,
    local_to_surface: LogicalTransform,
}

impl PaintSceneItem {
    pub(crate) const fn new(primitive: PaintPrimitive, local_to_surface: LogicalTransform) -> Self {
        Self {
            primitive,
            local_to_surface,
        }
    }

    /// Returns the neutral paint primitive authored by the widget.
    #[must_use]
    pub const fn primitive(&self) -> &PaintPrimitive {
        &self.primitive
    }

    /// Returns the exact owner-local to surface-logical placement transform.
    #[must_use]
    pub const fn local_to_surface(&self) -> LogicalTransform {
        self.local_to_surface
    }
}

/// Immutable canonical renderer scene content.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct PaintScene {
    items: Arc<Vec<PaintSceneItem>>,
}

impl PaintScene {
    pub(crate) fn new(items: Vec<PaintSceneItem>) -> Self {
        Self {
            items: Arc::new(items),
        }
    }

    /// Returns paint items in exact deterministic scene order.
    #[must_use]
    pub fn items(&self) -> &[PaintSceneItem] {
        self.items.as_slice()
    }

    /// Returns whether the scene contains no paint items.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    #[cfg(test)]
    pub(crate) fn shares_storage_with(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.items, &other.items)
    }
}

/// Surface-scoped non-zero renderer update identity.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct PaintRevision(NonZeroU64);

impl PaintRevision {
    pub(crate) fn new(value: u64) -> Option<Self> {
        NonZeroU64::new(value).map(Self)
    }

    /// Returns the non-zero revision value.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0.get()
    }
}

/// Immutable renderer update for one logical surface.
///
/// M6B owns scene content, exact logical extent, and renderer revision identity.
/// M6C extends this same value with scale/base/damage metadata rather than
/// introducing another renderer publication path.
#[derive(Clone, Debug, PartialEq)]
pub struct PaintPublication {
    surface_id: SurfaceId,
    revision: PaintRevision,
    logical_size: LogicalSize,
    scene: PaintScene,
}

impl PaintPublication {
    pub(crate) const fn new(
        surface_id: SurfaceId,
        revision: PaintRevision,
        logical_size: LogicalSize,
        scene: PaintScene,
    ) -> Self {
        Self {
            surface_id,
            revision,
            logical_size,
            scene,
        }
    }

    /// Returns the exact logical surface identity.
    #[must_use]
    pub const fn surface_id(&self) -> &SurfaceId {
        &self.surface_id
    }

    /// Returns the surface-scoped renderer update revision.
    #[must_use]
    pub const fn revision(&self) -> PaintRevision {
        self.revision
    }

    /// Returns the exact logical renderer target extent.
    #[must_use]
    pub const fn logical_size(&self) -> LogicalSize {
        self.logical_size
    }

    /// Returns the complete immutable renderer scene for this revision.
    #[must_use]
    pub const fn scene(&self) -> &PaintScene {
        &self.scene
    }
}

/// One runtime-targeted physical rectangle in stable hit-scene order.
#[derive(Clone, Debug, PartialEq)]
pub struct HitTestRegion {
    target: MountedNodeId,
    local_rect: LogicalRect,
    local_to_surface: LogicalTransform,
    surface_rect: LogicalRect,
}

impl HitTestRegion {
    pub(crate) const fn new(
        target: MountedNodeId,
        local_rect: LogicalRect,
        local_to_surface: LogicalTransform,
        surface_rect: LogicalRect,
    ) -> Self {
        Self {
            target,
            local_rect,
            local_to_surface,
            surface_rect,
        }
    }

    /// Returns the exact runtime-injected mounted target.
    #[must_use]
    pub const fn target(&self) -> &MountedNodeId {
        &self.target
    }

    /// Returns the widget-authored owner-local rectangle.
    #[must_use]
    pub const fn local_rect(&self) -> LogicalRect {
        self.local_rect
    }

    /// Returns the runtime-composed owner-local to surface-logical placement.
    #[must_use]
    pub const fn local_to_surface(&self) -> LogicalTransform {
        self.local_to_surface
    }

    fn contains_surface_point(&self, point: LogicalPoint) -> bool {
        self.surface_rect.contains(point)
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct HitTestSceneContent {
    data: Arc<HitTestSceneContentData>,
}

#[derive(Clone, Debug, Default, PartialEq)]
struct HitTestSceneContentData {
    regions: Vec<HitTestRegion>,
    membership: Vec<MountedNodeId>,
}

impl HitTestSceneContent {
    pub(crate) fn new(regions: Vec<HitTestRegion>, membership: Vec<MountedNodeId>) -> Self {
        Self {
            data: Arc::new(HitTestSceneContentData {
                regions,
                membership,
            }),
        }
    }

    pub(crate) fn regions(&self) -> &[HitTestRegion] {
        self.data.regions.as_slice()
    }

    pub(crate) fn membership(&self) -> &[MountedNodeId] {
        self.data.membership.as_slice()
    }

    #[cfg(test)]
    pub(crate) fn shares_storage_with(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.data, &other.data)
    }
}

/// Exact immutable displayed hit-test scene for one `SurfaceInputContext`.
///
/// Pointer regions and mounted-target membership are independent facts. Region
/// absence means point pass-through only; it does not remove a mounted node from
/// the historical snapshot membership represented by this scene.
#[derive(Clone, Debug, PartialEq)]
pub struct HitTestScene {
    context: SurfaceInputContext,
    content: HitTestSceneContent,
}

impl HitTestScene {
    pub(crate) const fn new(context: SurfaceInputContext, content: HitTestSceneContent) -> Self {
        Self { context, content }
    }

    /// Returns the exact runtime-issued displayed input context owned by this scene.
    #[must_use]
    pub const fn input_context(&self) -> &SurfaceInputContext {
        &self.context
    }

    /// Returns ordered physical target regions.
    #[must_use]
    pub fn regions(&self) -> &[HitTestRegion] {
        self.content.regions()
    }

    /// Returns exact mounted membership for this displayed generation.
    #[must_use]
    pub fn mounted_targets(&self) -> &[MountedNodeId] {
        self.content.membership()
    }

    /// Resolves the topmost targetable M6B rectangle at one surface-logical point.
    #[must_use]
    pub fn target_at(&self, point: LogicalPoint) -> Option<&MountedNodeId> {
        self.regions()
            .iter()
            .rev()
            .find(|region| region.contains_surface_point(point))
            .map(HitTestRegion::target)
    }

    /// Returns whether the target belonged to this exact displayed snapshot.
    #[must_use]
    pub fn contains_mounted_target(&self, target: &MountedNodeId) -> bool {
        self.mounted_targets().iter().any(|member| member == target)
    }

    pub(crate) const fn content(&self) -> &HitTestSceneContent {
        &self.content
    }

    #[cfg(feature = "internal-test-seams")]
    pub(crate) fn replace_target_for_test(
        &mut self,
        original: &MountedNodeId,
        replacement: MountedNodeId,
    ) {
        let data = Arc::make_mut(&mut self.content.data);
        for region in &mut data.regions {
            if &region.target == original {
                region.target = replacement.clone();
            }
        }
        let member = data
            .membership
            .iter_mut()
            .find(|member| *member == original)
            .unwrap_or_else(|| unreachable!("test original target is retained"));
        *member = replacement;
    }
}
