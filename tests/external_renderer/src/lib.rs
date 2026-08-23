//! Genuine downstream renderer-neutral scene consumer used by M6D conformance.
//!
//! This crate intentionally depends only on ordinary public core/runtime contracts.
//! It is a deterministic scene interpreter/recorder, not a production renderer.

#![forbid(unsafe_code)]

use runenui_core::{
    LogicalPoint, LogicalSize, LogicalTransform, MountedNodeId, PaintPrimitive, PointerPolicy,
    ResourceKind, SceneLayer, SceneOpacity, SceneShape, SurfaceId, SurfaceInputContext,
};
use runenui_runtime::{
    HitTestScene, PaintDamage, PaintPublication, PaintRevision, RasterScale, SceneCapabilities,
    SceneClip, UnsupportedSceneRequirement,
};

/// Whether one supplied publication has an exactly realized predecessor.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum UpdateMode {
    /// The consumer has no exact `(SurfaceId, PaintRevision)` match for the base.
    FullResync,
    /// The publication's base is exactly the consumer's last realized revision.
    ExactBaseMatch,
}

/// One paint item copied into consumer-owned deterministic state.
#[derive(Clone, Debug, PartialEq)]
pub struct PaintRecord {
    primitive: PaintPrimitive,
    local_to_surface: LogicalTransform,
    clips: Vec<SceneClip>,
    opacity: SceneOpacity,
    layer: SceneLayer,
}

impl PaintRecord {
    /// Returns the exact neutral primitive.
    #[must_use]
    pub const fn primitive(&self) -> &PaintPrimitive {
        &self.primitive
    }

    /// Returns the exact primitive-local to surface-logical transform.
    #[must_use]
    pub const fn local_to_surface(&self) -> LogicalTransform {
        self.local_to_surface
    }

    /// Returns conjunctive clips in canonical scene order.
    #[must_use]
    pub const fn clips(&self) -> &[SceneClip] {
        self.clips.as_slice()
    }

    /// Returns explicit item opacity.
    #[must_use]
    pub const fn opacity(&self) -> SceneOpacity {
        self.opacity
    }

    /// Returns the snapshot-local layer.
    #[must_use]
    pub const fn layer(&self) -> SceneLayer {
        self.layer
    }
}

/// One hit region copied into consumer-owned deterministic state.
#[derive(Clone, Debug, PartialEq)]
pub struct HitRecord {
    target: MountedNodeId,
    shape: SceneShape,
    local_to_surface: LogicalTransform,
    clips: Vec<SceneClip>,
    layer: SceneLayer,
    pointer_policy: PointerPolicy,
}

impl HitRecord {
    /// Returns the runtime-issued mounted owner published by the scene.
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

    /// Returns conjunctive clips in canonical scene order.
    #[must_use]
    pub const fn clips(&self) -> &[SceneClip] {
        self.clips.as_slice()
    }

    /// Returns the snapshot-local layer.
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

/// Complete consumer-owned interpretation of one public paint/hit snapshot.
#[derive(Clone, Debug, PartialEq)]
pub struct ConsumerSnapshot {
    surface_id: SurfaceId,
    revision: PaintRevision,
    base_revision: Option<PaintRevision>,
    logical_size: LogicalSize,
    raster_scale: RasterScale,
    damage: PaintDamage,
    input_context: SurfaceInputContext,
    required_resource_kinds: Vec<ResourceKind>,
    paint_items: Vec<PaintRecord>,
    hit_regions: Vec<HitRecord>,
    mounted_targets: Vec<MountedNodeId>,
}

impl ConsumerSnapshot {
    #[must_use]
    pub const fn surface_id(&self) -> &SurfaceId {
        &self.surface_id
    }

    #[must_use]
    pub const fn revision(&self) -> PaintRevision {
        self.revision
    }

    #[must_use]
    pub const fn base_revision(&self) -> Option<PaintRevision> {
        self.base_revision
    }

    #[must_use]
    pub const fn logical_size(&self) -> LogicalSize {
        self.logical_size
    }

    #[must_use]
    pub const fn raster_scale(&self) -> RasterScale {
        self.raster_scale
    }

    #[must_use]
    pub const fn damage(&self) -> PaintDamage {
        self.damage
    }

    #[must_use]
    pub const fn input_context(&self) -> &SurfaceInputContext {
        &self.input_context
    }

    #[must_use]
    pub const fn required_resource_kinds(&self) -> &[ResourceKind] {
        self.required_resource_kinds.as_slice()
    }

    #[must_use]
    pub const fn paint_items(&self) -> &[PaintRecord] {
        self.paint_items.as_slice()
    }

    #[must_use]
    pub const fn hit_regions(&self) -> &[HitRecord] {
        self.hit_regions.as_slice()
    }

    #[must_use]
    pub const fn mounted_targets(&self) -> &[MountedNodeId] {
        self.mounted_targets.as_slice()
    }

    /// Resolves one point independently from copied public hit-region fields.
    #[must_use]
    pub fn target_at(&self, point: LogicalPoint) -> Option<&MountedNodeId> {
        for region in self.hit_regions.iter().rev() {
            if !region_contains_surface_point(region, point) {
                continue;
            }
            return match region.pointer_policy {
                PointerPolicy::Target => Some(&region.target),
                PointerPolicy::Block => None,
            };
        }
        None
    }
}

/// Result of consuming one complete public snapshot.
#[derive(Clone, Debug, PartialEq)]
pub struct Consumption {
    mode: UpdateMode,
    snapshot: ConsumerSnapshot,
}

impl Consumption {
    #[must_use]
    pub const fn mode(&self) -> UpdateMode {
        self.mode
    }

    #[must_use]
    pub const fn snapshot(&self) -> &ConsumerSnapshot {
        &self.snapshot
    }

    #[must_use]
    pub fn into_snapshot(self) -> ConsumerSnapshot {
        self.snapshot
    }
}

#[derive(Clone, Debug, PartialEq)]
struct RealizedRevision {
    surface_id: SurfaceId,
    revision: PaintRevision,
}

/// Deterministic downstream consumer with only predecessor identity as retained state.
#[derive(Clone, Debug)]
pub struct SceneConsumer {
    capabilities: SceneCapabilities,
    realized: Option<RealizedRevision>,
}

impl SceneConsumer {
    /// Creates a consumer with explicit renderer-neutral resource capabilities.
    #[must_use]
    pub fn new(capabilities: SceneCapabilities) -> Self {
        Self {
            capabilities,
            realized: None,
        }
    }

    /// Drops all consumer-owned realization history.
    pub fn reset(&mut self) {
        self.realized = None;
    }

    /// Consumes a complete ordinary public paint/hit snapshot.
    ///
    /// The scene is always rebuilt from the supplied products. `base_revision`
    /// controls only whether predecessor-relative damage is eligible; it is never
    /// used as hidden scene reconstruction state.
    ///
    /// # Errors
    ///
    /// Returns the canonical unsupported-requirement error when this consumer's
    /// declared resource capabilities cannot process the scene.
    pub fn consume(
        &mut self,
        publication: &PaintPublication,
        hit_scene: &HitTestScene,
    ) -> Result<Consumption, UnsupportedSceneRequirement> {
        let requirements = publication.scene().requirements();
        self.capabilities.check_requirements(&requirements)?;

        let mode = self
            .realized
            .as_ref()
            .map_or(UpdateMode::FullResync, |realized| {
                if &realized.surface_id == publication.surface_id()
                    && publication.base_revision() == Some(realized.revision)
                {
                    UpdateMode::ExactBaseMatch
                } else {
                    UpdateMode::FullResync
                }
            });

        let snapshot = ConsumerSnapshot {
            surface_id: publication.surface_id().clone(),
            revision: publication.revision(),
            base_revision: publication.base_revision(),
            logical_size: publication.logical_size(),
            raster_scale: publication.raster_scale(),
            damage: publication.damage(),
            input_context: hit_scene.input_context().clone(),
            required_resource_kinds: requirements.resource_kinds().to_vec(),
            paint_items: publication
                .scene()
                .items()
                .iter()
                .map(|item| PaintRecord {
                    primitive: item.primitive().clone(),
                    local_to_surface: item.local_to_surface(),
                    clips: item.clips().to_vec(),
                    opacity: item.opacity(),
                    layer: item.layer(),
                })
                .collect(),
            hit_regions: hit_scene
                .regions()
                .iter()
                .map(|region| HitRecord {
                    target: region.target().clone(),
                    shape: region.shape(),
                    local_to_surface: region.local_to_surface(),
                    clips: region.clips().to_vec(),
                    layer: region.layer(),
                    pointer_policy: region.pointer_policy(),
                })
                .collect(),
            mounted_targets: hit_scene.mounted_targets().to_vec(),
        };

        self.realized = Some(RealizedRevision {
            surface_id: publication.surface_id().clone(),
            revision: publication.revision(),
        });

        Ok(Consumption { mode, snapshot })
    }
}

fn region_contains_surface_point(region: &HitRecord, point: LogicalPoint) -> bool {
    let Some(region_point) = region
        .local_to_surface
        .inverse()
        .and_then(|surface_to_local| surface_to_local.transform_point(point))
    else {
        return false;
    };
    if !region.shape.contains(region_point) {
        return false;
    }
    region.clips.iter().all(|clip| {
        clip.clip_to_surface()
            .inverse()
            .and_then(|surface_to_clip| surface_to_clip.transform_point(point))
            .is_some_and(|clip_point| clip.shape().contains(clip_point))
    })
}
