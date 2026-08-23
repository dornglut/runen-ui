//! Reusable renderer edge over ordinary public `RunenUI` paint publications.
//!
//! This crate is intentionally outside `runenui_core` and `runenui_runtime`
//! authority. Native event-loop and accessibility integration are separate M7
//! slices; renderer-side realization state must remain disposable.

#![forbid(unsafe_code)]

mod backend;
mod lineage;
mod observation;
mod resource;

pub use backend::{
    AdapterPowerPreference, BackendSelection, OffscreenExtent, OffscreenReadback,
    OffscreenRenderError, Renderer, RendererDiagnostics, RendererInitError, RendererOptions,
};
pub use lineage::{PublicationLineage, PublicationUpdateMode, PublicationUpdatePlan};
pub use observation::PublicationObservation;
pub use resource::{
    ImagePayload, PayloadValidationError, ResourcePayload, ResourceProvider, ResourceProviderError,
    ResourceProviderErrorKind, ResourceRequest, ResourceResolveError, ShapedRunRaster,
    resolve_resource,
};
