use std::sync::Arc;

use runenui_core::{LogicalSize, ResourceKind, ResourceRef, SurfaceId};
use runenui_runtime::{PaintDamage, PaintPublication, PaintRevision, RasterScale};

use crate::PublicationUpdateMode;
use crate::backend::{OffscreenExtent, RendererDiagnostics};

/// Renderer-owned category of realization work for one resource-backed scene item.
///
/// This deliberately describes renderer work rather than caller-provider vocabulary.
/// External image lookup may still use [`ResourceRequest`] internally, while shaped-text
/// realization is free to move from provider coverage to renderer-owned MSDF without changing
/// the observation contract.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResourceRealizationKind {
    /// Renderer realization of an externally supplied image resource.
    Image,
    /// Renderer realization of one immutable logical shaped-text resource.
    ShapedText,
}

/// Renderer-owned result of one logical-resource realization decision.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ResourceCacheOutcome {
    /// The exact complete resource identity and renderer realization were already available.
    Reused,
    /// A new renderer realization was retained.
    Realized,
    /// The logical resource produced valid empty coverage, so no GPU texture was required.
    EmptyCoverage,
    /// Resource resolution or renderer realization failed before a usable realization existed.
    Failed,
}

/// Truthful result of one renderer publication stage.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PublicationStageResult {
    /// The stage was not reached by the attempted publication.
    NotAttempted,
    /// The stage completed successfully.
    Succeeded,
    /// The stage was reached and failed.
    Failed,
}

/// Immutable correlation record for one resource-backed scene item.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResourceObservation {
    item_index: usize,
    resource: ResourceRef,
    realization_kind: ResourceRealizationKind,
    cache_outcome: ResourceCacheOutcome,
}

impl ResourceObservation {
    pub(crate) const fn new(
        item_index: usize,
        resource: ResourceRef,
        realization_kind: ResourceRealizationKind,
        cache_outcome: ResourceCacheOutcome,
    ) -> Self {
        Self {
            item_index,
            resource,
            realization_kind,
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

    /// Returns the renderer-owned category of work represented by this observation.
    #[must_use]
    pub const fn realization_kind(&self) -> ResourceRealizationKind {
        self.realization_kind
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
    render_result: PublicationStageResult,
    readback_result: PublicationStageResult,
    present_result: PublicationStageResult,
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
            render_result: PublicationStageResult::NotAttempted,
            readback_result: PublicationStageResult::NotAttempted,
            present_result: PublicationStageResult::NotAttempted,
        }
    }

    pub(crate) fn set_target_facts(
        &mut self,
        extent: OffscreenExtent,
        target_generation: Option<u64>,
        diagnostics: &RendererDiagnostics,
    ) {
        self.set_target_facts_with_format(
            extent,
            target_generation,
            diagnostics.offscreen_format(),
            diagnostics,
        );
    }

    pub(crate) fn set_target_facts_with_format(
        &mut self,
        extent: OffscreenExtent,
        target_generation: Option<u64>,
        target_format: wgpu::TextureFormat,
        diagnostics: &RendererDiagnostics,
    ) {
        self.physical_extent = Some(extent);
        self.target_generation = target_generation;
        self.target_format = Some(target_format);
        self.adapter_name = Some(diagnostics.adapter_info().name.clone().into());
        self.backend = Some(diagnostics.adapter_info().backend);
    }

    pub(crate) fn set_resource_observations(&mut self, observations: Vec<ResourceObservation>) {
        self.resource_observations = observations;
    }

    pub(crate) const fn mark_render_succeeded(&mut self) {
        self.render_result = PublicationStageResult::Succeeded;
    }

    pub(crate) const fn mark_readback_succeeded(&mut self) {
        self.readback_result = PublicationStageResult::Succeeded;
    }

    pub(crate) const fn mark_readback_failed(&mut self) {
        self.readback_result = PublicationStageResult::Failed;
    }

    pub(crate) const fn mark_present_succeeded(&mut self) {
        self.present_result = PublicationStageResult::Succeeded;
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
    pub const fn render_result(&self) -> PublicationStageResult {
        self.render_result
    }

    #[must_use]
    pub const fn readback_result(&self) -> PublicationStageResult {
        self.readback_result
    }

    /// Returns the result of native presentation for this publication attempt.
    /// Offscreen rendering leaves this stage [`PublicationStageResult::NotAttempted`].
    #[must_use]
    pub const fn present_result(&self) -> PublicationStageResult {
        self.present_result
    }

    /// Compatibility convenience for callers that only need the successful stage fact.
    #[must_use]
    pub const fn render_succeeded(&self) -> bool {
        matches!(self.render_result, PublicationStageResult::Succeeded)
    }

    /// Compatibility convenience for callers that only need the successful stage fact.
    #[must_use]
    pub const fn readback_succeeded(&self) -> bool {
        matches!(self.readback_result, PublicationStageResult::Succeeded)
    }

    /// Returns whether the publication was successfully presented to a native surface.
    #[must_use]
    pub const fn presented(&self) -> bool {
        matches!(self.present_result, PublicationStageResult::Succeeded)
    }
}
