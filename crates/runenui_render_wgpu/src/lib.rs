//! Reusable concrete wgpu renderer edge over ordinary public `RunenUI` paint publications.
//!
//! The crate owns disposable GPU realization, offscreen/native target state,
//! external-image realization, retained shaped-text SDF/MSDF realization, readback,
//! presentation, and renderer observations. It remains outside `runenui_core` and
//! `runenui_runtime` authority and does not own widget behavior, semantics, mounted
//! state, text shaping/line breaking, logical layout, accessibility, or a native
//! event loop.

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
#[allow(
    clippy::redundant_pub_crate,
    reason = "the private tessellation module exposes only explicit sibling realization seams without widening the public API"
)]
mod tessellation;

pub use backend::clipped::{
    PublicationRenderError, ResourceRenderer as Renderer, UnsupportedShapedGlyphKind,
};
pub use backend::{
    AdapterPowerPreference, BackendSelection, OffscreenExtent, OffscreenPublicationReadback,
    OffscreenReadback, OffscreenRenderError, RendererDiagnostics, RendererInitError,
    RendererOptions,
};
pub use lineage::{PublicationUpdateMode, PublicationUpdatePlan};
pub use observation::{
    PublicationObservation, PublicationStageResult, ResourceCacheOutcome, ResourceObservation,
    ResourceRealizationKind,
};
pub use resource::{
    ImagePayload, PayloadValidationError, ResourcePayload, ResourceProvider, ResourceProviderError,
    ResourceProviderErrorKind, ResourceRequest, ResourceResolveError, resolve_resource,
};
pub use wgpu_types::WgpuHasDisplayHandle;
