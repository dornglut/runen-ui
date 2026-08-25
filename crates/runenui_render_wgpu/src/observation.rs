use std::sync::Arc;

use runenui_core::{LogicalSize, ResourceKind, ResourceRef, SurfaceId};
use runenui_runtime::{PaintDamage, PaintPublication, PaintRevision, RasterScale};

use crate::{
    PublicationUpdateMode, ResourceRequest,
    backend::{OffscreenExtent, RendererDiagnostics},
};

/// Renderer-owned result of one logical-resource lookup/realization decision.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResourceCacheOutcome {
    /// The exact complete resource identity and request were already realized.
    Reused,
    /// The provider payload was loaded and a new renderer realization was retained.
    Realized,
    /// The provider returned valid empty coverage, so no GPU texture was required.
    EmptyCoverage,
}

/// Immutable correlation record for one resource-backed scene item.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResourceObservation {
    item_index: usize,
    resource: ResourceRef,
    request: ResourceRequest,
    cache_outcome: ResourceCacheOutcome,
}

impl ResourceObservation {
    pub(crate) const fn new(
        item_index: usize,
        resource: ResourceRef,
        request: ResourceRequest,
        cache_outcome: ResourceCacheOutcome,
    ) -> Self {
        Self {
            item_index,
            resource,
            request,
            cache_outcome,
        }
    }

    #[must_use]
    pub const fn item_index(&self) -> usize {
        self.item_index
    }

    #[must_use]
    pub const fn resource(&self) -> &ResourceRef {
        &self.resource
    }

    #[must_use]
    pub const fn request(&self) -> ResourceRequest {
        self.request
    }

    #[must_use]
    pub const fn cache_outcome(&self) -> ResourceCacheOutcome {
        self.cache_outcome
    }
}

/// Immutable renderer-edge observation derived from one public paint publication.
///
/// This value observes public publication facts and renderer-owned work only; it
/// never mutates runtime state or allocates `RunenUI` identities.
#[derive(Clone, Debug, PartialEq)]
pub struct PublicationObservation {
    surface_id: SurfaceId,
    revision: PaintRevision,
    base_revision: Option<PaintRevision>,
    update_mode: PublicationUpdateMode,
    damage: PaintDamage,
    logical_size: LogicalSize,
    raster_scale: RasterScale,
    required_resource_kinds: Vec<ResourceKind>,
    physical_extent: Option<OffscreenExtent>,
    target_generation: Option<u64>,
    target_format: Option<wgpu::TextureFormat>,
    adapter_name: Option<Arc<str>>,
    backend: Option<wgpu::Backend>,
    resource_observations: Vec<ResourceObservation>,
    render_succeeded: bool,
    readback_succeeded: bool,
    presented: bool,
}

impl PublicationObservation {
    /// Captures immutable public publication facts for one renderer classification.
    #[must_use]
    pub fn new(publication: &PaintPublication, update_mode: PublicationUpdateMode) -> Self {
        let requirements = publication.scene().requirements();
        Self {
            surface_id: publication.surface_id().clone(),
            revision: publication.revision(),
            base_revision: publication.base_revision(),
            update_mode,
            damage: publication.damage(),
            logical_size: publication.logical_size(),
            raster_scale: publication.raster_scale(),
            required_resource_kinds: requirements.resource_kinds().to_vec(),
            physical_extent: None,
            target_generation: None,
            target_format: None,
            adapter_name: None,
            backend: None,
            resource_observations: Vec::new(),
            render_succeeded: false,
            readback_succeeded: false,
            presented: false,
        }
    }

    pub(crate) fn completed(
        publication: &PaintPublication,
        update_mode: PublicationUpdateMode,
        extent: OffscreenExtent,
        target_generation: u64,
        diagnostics: &RendererDiagnostics,
        resource_observations: Vec<ResourceObservation>,
    ) -> Self {
        let mut observation = Self::new(publication, update_mode);
        observation.physical_extent = Some(extent);
        observation.target_generation = Some(target_generation);
        observation.target_format = Some(diagnostics.offscreen_format());
        observation.adapter_name = Some(diagnostics.adapter_info().name.clone().into());
        observation.backend = Some(diagnostics.adapter_info().backend);
        observation.resource_observations = resource_observations;
        observation.render_succeeded = true;
        observation.readback_succeeded = true;
        observation
    }

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
    pub const fn update_mode(&self) -> PublicationUpdateMode {
        self.update_mode
    }

    #[must_use]
    pub const fn damage(&self) -> PaintDamage {
        self.damage
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
    pub const fn required_resource_kinds(&self) -> &[ResourceKind] {
        self.required_resource_kinds.as_slice()
    }

    #[must_use]
    pub const fn physical_extent(&self) -> Option<OffscreenExtent> {
        self.physical_extent
    }

    #[must_use]
    pub const fn target_generation(&self) -> Option<u64> {
        self.target_generation
    }

    #[must_use]
    pub const fn target_format(&self) -> Option<wgpu::TextureFormat> {
        self.target_format
    }

    #[must_use]
    pub fn adapter_name(&self) -> Option<&str> {
        self.adapter_name.as_deref()
    }

    #[must_use]
    pub const fn backend(&self) -> Option<wgpu::Backend> {
        self.backend
    }

    #[must_use]
    pub const fn resource_observations(&self) -> &[ResourceObservation] {
        self.resource_observations.as_slice()
    }

    #[must_use]
    pub const fn render_succeeded(&self) -> bool {
        self.render_succeeded
    }

    #[must_use]
    pub const fn readback_succeeded(&self) -> bool {
        self.readback_succeeded
    }

    /// Offscreen rendering does not present a native surface; a future window
    /// path will set this fact only after the real present succeeds.
    #[must_use]
    pub const fn presented(&self) -> bool {
        self.presented
    }
}
