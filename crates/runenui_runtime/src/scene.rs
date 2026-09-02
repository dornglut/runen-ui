//! Canonical immutable renderer-neutral paint and displayed-hit products.

use core::num::NonZeroU64;
use std::sync::Arc;

use runenui_core::{
    LogicalPoint, LogicalSize, LogicalTransform, MountedNodeId, PaintPrimitive, PointerPolicy,
    ResourceRef, SceneLayer, SceneOpacity, SceneShape, SurfaceId, SurfaceInputContext,
};
use runenui_text::{ShapedTextResource, TextArtifact, TextLine, TextRun};

use crate::surface::RasterScale;

/// One self-contained conjunctive scene clip in surface-logical coordinates.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SceneClip {
    shape: SceneShape,
    clip_to_surface: LogicalTransform,
}

impl SceneClip {
    pub(crate) const fn new(shape: SceneShape, clip_to_surface: LogicalTransform) -> Self {
        Self {
            shape,
            clip_to_surface,
        }
    }

    /// Returns the exact logical clip shape.
    #[must_use]
    pub const fn shape(self) -> SceneShape {
        self.shape
    }

    /// Returns the exact clip-local to surface-logical transform.
    #[must_use]
    pub const fn clip_to_surface(self) -> LogicalTransform {
        self.clip_to_surface
    }

    /// Evaluates one surface-logical point against this clip.
    ///
    /// A non-invertible clip transform excludes coverage rather than making the
    /// clip disappear or falling back to untransformed geometry.
    #[must_use]
    pub fn contains_surface_point(self, point: LogicalPoint) -> bool {
        self.clip_to_surface
            .inverse()
            .and_then(|surface_to_clip| surface_to_clip.transform_point(point))
            .is_some_and(|clip_point| self.shape.contains(clip_point))
    }
}

/// One self-contained renderer-neutral paint item in stable scene order.
#[derive(Clone, Debug, PartialEq)]
pub struct PaintSceneItem {
    primitive: PaintPrimitive,
    local_to_surface: LogicalTransform,
    clips: Vec<SceneClip>,
    opacity: SceneOpacity,
    layer: SceneLayer,
}

impl PaintSceneItem {
    pub(crate) const fn new(
        primitive: PaintPrimitive,
        local_to_surface: LogicalTransform,
        clips: Vec<SceneClip>,
        opacity: SceneOpacity,
        layer: SceneLayer,
    ) -> Self {
        Self {
            primitive,
            local_to_surface,
            clips,
            opacity,
            layer,
        }
    }

    /// Returns the neutral paint primitive authored by the widget.
    #[must_use]
    pub const fn primitive(&self) -> &PaintPrimitive {
        &self.primitive
    }

    /// Returns the exact primitive-local to surface-logical transform.
    #[must_use]
    pub const fn local_to_surface(&self) -> LogicalTransform {
        self.local_to_surface
    }

    /// Returns conjunctive clips in authored order.
    #[must_use]
    pub const fn clips(&self) -> &[SceneClip] {
        self.clips.as_slice()
    }

    /// Returns validated explicit item opacity.
    #[must_use]
    pub const fn opacity(&self) -> SceneOpacity {
        self.opacity
    }

    /// Returns the snapshot-local ordering layer.
    #[must_use]
    pub const fn layer(&self) -> SceneLayer {
        self.layer
    }
}

/// Immutable canonical renderer scene content.
///
/// Logical text artifacts are retained privately only to keep every shaped
/// `ResourceRef` in this exact scene bound to its immutable logical payload for
/// retained-publication renderer retry. They are not separate paint authority:
/// visible scene identity remains the ordered paint items.
#[derive(Clone, Debug, Default)]
pub struct PaintScene {
    items: Arc<Vec<PaintSceneItem>>,
    text_artifacts: Arc<Vec<TextArtifact>>,
}

impl PartialEq for PaintScene {
    fn eq(&self, other: &Self) -> bool {
        self.items == other.items
    }
}

impl PaintScene {
    pub(crate) fn with_text_artifacts(
        items: Vec<PaintSceneItem>,
        text_artifacts: Vec<TextArtifact>,
    ) -> Self {
        Self {
            items: Arc::new(items),
            text_artifacts: Arc::new(text_artifacts),
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

    /// Resolves one runtime-backed shaped-text reference to the exact immutable
    /// logical resource retained by this scene.
    ///
    /// Renderer scale, quality, atlas placement, and device state are deliberately
    /// absent. Resource references authored by other caller-owned resource domains
    /// simply return `None` and remain the caller's responsibility.
    #[must_use]
    pub fn shaped_text_resource(&self, resource: &ResourceRef) -> Option<&ShapedTextResource> {
        self.text_artifacts
            .iter()
            .flat_map(TextArtifact::lines)
            .flat_map(TextLine::runs)
            .find(|run| run.resource_ref() == resource)
            .map(TextRun::shaped_resource)
    }

    #[cfg(test)]
    pub(crate) fn shares_storage_with(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.items, &other.items)
            && Arc::ptr_eq(&self.text_artifacts, &other.text_artifacts)
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

/// Conservative renderer damage for one changed paint publication.
///
/// M6C intentionally permits full-surface damage for every changed renderer
/// tuple. The exact damaged logical extent is the publication's `logical_size`;
/// damage never participates in [`PaintScene`] content identity.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PaintDamage {
    /// Reprocess the complete logical surface extent.
    FullSurface,
}

/// Immutable renderer update for one logical surface.
///
/// Scene content remains history-independent. Revision lineage, exact logical
/// extent, raster scale, and damage are publication metadata only.
#[derive(Clone, Debug, PartialEq)]
pub struct PaintPublication {
    surface_id: SurfaceId,
    revision: PaintRevision,
    base_revision: Option<PaintRevision>,
    logical_size: LogicalSize,
    raster_scale: RasterScale,
    damage: PaintDamage,
    scene: PaintScene,
}

impl PaintPublication {
    pub(crate) const fn new(
        surface_id: SurfaceId,
        revision: PaintRevision,
        base_revision: Option<PaintRevision>,
        logical_size: LogicalSize,
        raster_scale: RasterScale,
        scene: PaintScene,
    ) -> Self {
        Self {
            surface_id,
            revision,
            base_revision,
            logical_size,
            raster_scale,
            damage: PaintDamage::FullSurface,
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

    /// Returns the immediately previous accepted paint revision used as this
    /// update's damage base, or `None` for the first publication.
    #[must_use]
    pub const fn base_revision(&self) -> Option<PaintRevision> {
        self.base_revision
    }

    /// Returns the exact logical renderer target extent.
    #[must_use]
    pub const fn logical_size(&self) -> LogicalSize {
        self.logical_size
    }

    /// Returns the exact validated renderer raster scale.
    #[must_use]
    pub const fn raster_scale(&self) -> RasterScale {
        self.raster_scale
    }

    /// Returns deterministic damage relative to [`Self::base_revision`].
    #[must_use]
    pub const fn damage(&self) -> PaintDamage {
        self.damage
    }

    /// Returns the complete immutable renderer scene for this revision.
    #[must_use]
    pub const fn scene(&self) -> &PaintScene {
        &self.scene
    }
}

/// One runtime-targeted physical hit region in stable hit-scene order.
#[derive(Clone, Debug, PartialEq)]
pub struct HitTestRegion {
    target: MountedNodeId,
    shape: SceneShape,
    local_to_surface: LogicalTransform,
    clips: Vec<SceneClip>,
    layer: SceneLayer,
    pointer_policy: PointerPolicy,
}

impl HitTestRegion {
    pub(crate) const fn new(
        target: MountedNodeId,
        shape: SceneShape,
        local_to_surface: LogicalTransform,
        clips: Vec<SceneClip>,
        layer: SceneLayer,
        pointer_policy: PointerPolicy,
    ) -> Self {
        Self {
            target,
            shape,
            local_to_surface,
            clips,
            layer,
            pointer_policy,
        }
    }

    /// Returns the exact runtime-injected mounted owner.
    #[must_use]
    pub const fn target(&self) -> &MountedNodeId {
        &self.target
    }

    /// Returns the exact logical region shape.
    #[must_use]
    pub const fn shape(&self) -> SceneShape {
        self.shape
    }

    /// Returns the exact region-local to surface-logical transform.
    #[must_use]
    pub const fn local_to_surface(&self) -> LogicalTransform {
        self.local_to_surface
    }

    /// Returns conjunctive clips in exact authored order.
    #[must_use]
    pub const fn clips(&self) -> &[SceneClip] {
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

    /// Evaluates one surface-logical point against the exact transformed shape
    /// and every conjunctive clip.
    ///
    /// Non-invertible region or clip transforms produce no eligible coverage.
    #[must_use]
    pub fn contains_surface_point(&self, point: LogicalPoint) -> bool {
        let Some(region_point) = self
            .local_to_surface
            .inverse()
            .and_then(|surface_to_local| surface_to_local.transform_point(point))
        else {
            return false;
        };
        self.shape.contains(region_point)
            && self
                .clips
                .iter()
                .all(|clip| clip.contains_surface_point(point))
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct HitTestSceneContent {
    data: Arc<HitTestSceneContentData>,
}

#[derive(Clone, Debug, Default, PartialEq)]
struct HitTestSceneContentData {
    regions: Vec<HitTestRegion>,
    membership: Vec<MountedNodeId>,
}

impl HitTestSceneContent {
    pub(super) fn new(regions: Vec<HitTestRegion>, membership: Vec<MountedNodeId>) -> Self {
        Self {
            data: Arc::new(HitTestSceneContentData {
                regions,
                membership,
            }),
        }
    }

    pub(super) fn regions(&self) -> &[HitTestRegion] {
        self.data.regions.as_slice()
    }

    pub(super) fn membership(&self) -> &[MountedNodeId] {
        self.data.membership.as_slice()
    }

    #[cfg(test)]
    pub(super) fn shares_storage_with(&self, other: &Self) -> bool {
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
    pub(crate) const fn new(
        input_context: SurfaceInputContext,
        scene_content: HitTestSceneContent,
    ) -> Self {
        Self {
            context: input_context,
            content: scene_content,
        }
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

    /// Resolves the first containing region in topmost order.
    ///
    /// `Target` returns the runtime-injected mounted owner. `Block` terminates
    /// resolution with no target. Omitted regions remain the sole pass-through
    /// representation.
    #[must_use]
    pub fn target_at(&self, point: LogicalPoint) -> Option<&MountedNodeId> {
        for region in self.regions().iter().rev() {
            if !region.contains_surface_point(point) {
                continue;
            }
            return match region.pointer_policy() {
                PointerPolicy::Target => Some(region.target()),
                PointerPolicy::Block => None,
            };
        }
        None
    }

    /// Returns whether the target belonged to this exact displayed snapshot.
    #[must_use]
    pub fn contains_mounted_target(&self, target: &MountedNodeId) -> bool {
        self.mounted_targets().iter().any(|member| member == target)
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
