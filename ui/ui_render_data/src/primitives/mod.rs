//! File: domain/ui/ui_render_data/src/primitives/mod.rs
//! Purpose: Renderer-facing UI primitive contracts.

pub mod border;
pub mod clip;
pub mod glyph_run;
pub mod graph_canvas;
pub mod image;
pub mod product_surface;
pub mod rect;
pub mod stroke;
pub mod ui_primitive;
pub mod viewport_surface_embed;

pub use border::BorderPrimitive;
pub use clip::ClipPrimitive;
pub use glyph_run::GlyphRunPrimitive;
pub use graph_canvas::{
    GraphCanvasPrimitiveBatch, GraphCanvasPrimitiveRole, GraphCanvasRenderPrimitive,
};
pub use image::ImagePrimitive;
pub use product_surface::{
    ProductSurfaceAlphaMode, ProductSurfacePrimitive, ProductSurfaceTextureBindingSource,
};
pub use rect::RectPrimitive;
pub use stroke::StrokePrimitive;
pub use ui_primitive::UiPrimitive;
pub use viewport_surface_embed::{
    ViewportSurfaceBinding, ViewportSurfaceBindingRegistry, ViewportSurfaceBindingSource,
    ViewportSurfaceEmbedPrimitive, ViewportSurfaceEmbedSlotId,
};
