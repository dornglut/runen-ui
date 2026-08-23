use runenui_core::{LogicalSize, ResourceKind, SurfaceId};
use runenui_runtime::{PaintDamage, PaintPublication, PaintRevision, RasterScale};

use crate::PublicationUpdateMode;

/// Immutable renderer-edge observation derived from one public paint publication.
///
/// This records publication and classification facts only. Backend, resource,
/// readback, and present observations are added by the concrete renderer path;
/// this value never mutates runtime state or allocates `RunenUI` identities.
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
        }
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
}
