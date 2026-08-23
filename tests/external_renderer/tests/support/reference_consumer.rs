use runenui_core::{
    LogicalSize, LogicalTransform, MountedNodeId, PaintPrimitive, PointerPolicy, ResourceKind,
    SceneLayer, SceneOpacity, SceneShape, SurfaceId, SurfaceInputContext,
};
use runenui_runtime::{
    HitTestScene, PaintDamage, PaintPublication, PaintRevision, RasterScale, SceneCapabilities,
    SceneClip,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum ReferenceUpdateMode {
    FullResync,
    ExactBaseMatch,
    AlreadyCurrent,
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct ReferencePaintRecord {
    primitive: PaintPrimitive,
    local_to_surface: LogicalTransform,
    clips: Vec<SceneClip>,
    opacity: SceneOpacity,
    layer: SceneLayer,
}

impl ReferencePaintRecord {
    #[must_use]
    pub(super) const fn primitive(&self) -> &PaintPrimitive {
        &self.primitive
    }

    #[must_use]
    pub(super) const fn local_to_surface(&self) -> LogicalTransform {
        self.local_to_surface
    }

    #[must_use]
    pub(super) const fn clips(&self) -> &[SceneClip] {
        self.clips.as_slice()
    }

    #[must_use]
    pub(super) const fn opacity(&self) -> SceneOpacity {
        self.opacity
    }

    #[must_use]
    pub(super) const fn layer(&self) -> SceneLayer {
        self.layer
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct ReferenceHitRecord {
    target: MountedNodeId,
    shape: SceneShape,
    local_to_surface: LogicalTransform,
    clips: Vec<SceneClip>,
    layer: SceneLayer,
    pointer_policy: PointerPolicy,
}

impl ReferenceHitRecord {
    #[must_use]
    pub(super) const fn target(&self) -> &MountedNodeId {
        &self.target
    }

    #[must_use]
    pub(super) const fn shape(&self) -> SceneShape {
        self.shape
    }

    #[must_use]
    pub(super) const fn local_to_surface(&self) -> LogicalTransform {
        self.local_to_surface
    }

    #[must_use]
    pub(super) const fn clips(&self) -> &[SceneClip] {
        self.clips.as_slice()
    }

    #[must_use]
    pub(super) const fn layer(&self) -> SceneLayer {
        self.layer
    }

    #[must_use]
    pub(super) const fn pointer_policy(&self) -> PointerPolicy {
        self.pointer_policy
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct ReferenceSnapshot {
    surface_id: SurfaceId,
    revision: PaintRevision,
    base_revision: Option<PaintRevision>,
    logical_size: LogicalSize,
    raster_scale: RasterScale,
    damage: PaintDamage,
    input_context: SurfaceInputContext,
    required_resource_kinds: Vec<ResourceKind>,
    paint_items: Vec<ReferencePaintRecord>,
    hit_regions: Vec<ReferenceHitRecord>,
    mounted_targets: Vec<MountedNodeId>,
}

impl ReferenceSnapshot {
    #[must_use]
    pub(super) const fn surface_id(&self) -> &SurfaceId {
        &self.surface_id
    }

    #[must_use]
    pub(super) const fn revision(&self) -> PaintRevision {
        self.revision
    }

    #[must_use]
    pub(super) const fn base_revision(&self) -> Option<PaintRevision> {
        self.base_revision
    }

    #[must_use]
    pub(super) const fn logical_size(&self) -> LogicalSize {
        self.logical_size
    }

    #[must_use]
    pub(super) const fn raster_scale(&self) -> RasterScale {
        self.raster_scale
    }

    #[must_use]
    pub(super) const fn damage(&self) -> PaintDamage {
        self.damage
    }

    #[must_use]
    pub(super) const fn input_context(&self) -> &SurfaceInputContext {
        &self.input_context
    }

    #[must_use]
    pub(super) const fn required_resource_kinds(&self) -> &[ResourceKind] {
        self.required_resource_kinds.as_slice()
    }

    #[must_use]
    pub(super) const fn paint_items(&self) -> &[ReferencePaintRecord] {
        self.paint_items.as_slice()
    }

    #[must_use]
    pub(super) const fn hit_regions(&self) -> &[ReferenceHitRecord] {
        self.hit_regions.as_slice()
    }

    #[must_use]
    pub(super) const fn mounted_targets(&self) -> &[MountedNodeId] {
        self.mounted_targets.as_slice()
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(super) struct ReferenceConsumption {
    mode: ReferenceUpdateMode,
    snapshot: ReferenceSnapshot,
}

impl ReferenceConsumption {
    #[must_use]
    pub(super) const fn mode(&self) -> ReferenceUpdateMode {
        self.mode
    }

    #[must_use]
    pub(super) const fn snapshot(&self) -> &ReferenceSnapshot {
        &self.snapshot
    }
}

#[derive(Clone, Debug)]
pub(super) struct ReferenceConsumer {
    capabilities: SceneCapabilities,
    realized: Option<(SurfaceId, PaintRevision)>,
}

impl ReferenceConsumer {
    #[must_use]
    pub(super) const fn new(capabilities: SceneCapabilities) -> Self {
        Self {
            capabilities,
            realized: None,
        }
    }

    pub(super) fn reset(&mut self) {
        self.realized = None;
    }

    pub(super) fn consume(
        &mut self,
        publication: &PaintPublication,
        hit_scene: &HitTestScene,
    ) -> Result<ReferenceConsumption, ResourceKind> {
        let requirements = publication.scene().requirements();
        for &kind in requirements.resource_kinds() {
            if !self.capabilities.supports_resource_kind(kind) {
                return Err(kind);
            }
        }

        let mode = match self.realized.as_ref() {
            Some((surface_id, revision))
                if surface_id == publication.surface_id()
                    && *revision == publication.revision() =>
            {
                ReferenceUpdateMode::AlreadyCurrent
            }
            Some((surface_id, revision))
                if surface_id == publication.surface_id()
                    && publication.base_revision() == Some(*revision) =>
            {
                ReferenceUpdateMode::ExactBaseMatch
            }
            _ => ReferenceUpdateMode::FullResync,
        };

        let paint_items = publication
            .scene()
            .items()
            .iter()
            .map(|item| ReferencePaintRecord {
                primitive: item.primitive().clone(),
                local_to_surface: item.local_to_surface(),
                clips: item.clips().to_vec(),
                opacity: item.opacity(),
                layer: item.layer(),
            })
            .collect();
        let hit_regions = hit_scene
            .regions()
            .iter()
            .map(|region| ReferenceHitRecord {
                target: region.target().clone(),
                shape: region.shape(),
                local_to_surface: region.local_to_surface(),
                clips: region.clips().to_vec(),
                layer: region.layer(),
                pointer_policy: region.pointer_policy(),
            })
            .collect();

        let snapshot = ReferenceSnapshot {
            surface_id: publication.surface_id().clone(),
            revision: publication.revision(),
            base_revision: publication.base_revision(),
            logical_size: publication.logical_size(),
            raster_scale: publication.raster_scale(),
            damage: publication.damage(),
            input_context: hit_scene.input_context().clone(),
            required_resource_kinds: requirements.resource_kinds().to_vec(),
            paint_items,
            hit_regions,
            mounted_targets: hit_scene.mounted_targets().to_vec(),
        };

        self.realized = Some((publication.surface_id().clone(), publication.revision()));
        Ok(ReferenceConsumption { mode, snapshot })
    }
}
