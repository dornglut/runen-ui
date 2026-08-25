//! Reusable renderer edge over ordinary public `RunenUI` paint publications.
//!
//! This crate is intentionally outside `runenui_core` and `runenui_runtime`
//! authority. Native event-loop and accessibility integration are separate M7
//! slices; renderer-side realization state must remain disposable.

#![forbid(unsafe_code)]

#[allow(
    clippy::redundant_pub_crate,
    reason = "the private backend module uses pub(crate) only for explicit crate-root sibling seams without widening the public API"
)]
mod backend;
mod lineage;
mod observation;
mod resource;
#[allow(
    clippy::redundant_pub_crate,
    reason = "the private scene-subset module exposes explicit crate-internal sibling seams without widening the public API"
)]
mod scene_subset;

pub use backend::clipped::{PublicationRenderError, ResourceRenderer as Renderer};
pub use backend::{
    AdapterPowerPreference, BackendSelection, OffscreenExtent, OffscreenPublicationReadback,
    OffscreenReadback, OffscreenRenderError, RendererDiagnostics, RendererInitError,
    RendererOptions,
};
pub use lineage::{PublicationUpdateMode, PublicationUpdatePlan};
pub use observation::PublicationObservation;
pub use resource::{
    ImagePayload, PayloadValidationError, ResourcePayload, ResourceProvider, ResourceProviderError,
    ResourceProviderErrorKind, ResourceRequest, ResourceResolveError, ShapedRunRaster,
    resolve_resource,
};
pub use wgpu_types::WgpuHasDisplayHandle;
