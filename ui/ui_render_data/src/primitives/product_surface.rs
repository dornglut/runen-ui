//! File: domain/ui/ui_render_data/src/primitives/product_surface.rs
//! Purpose: Backend-neutral product surface texture primitive.

use ui_math::UiRect;

use crate::{UiPaint, UiSortKey};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ProductSurfaceAlphaMode {
    Straight,
    Premultiplied,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum ProductSurfaceTextureBindingSource {
    DynamicTexture {
        namespace: String,
        target_id: String,
    },
}

impl ProductSurfaceTextureBindingSource {
    pub fn dynamic_texture(namespace: impl Into<String>, target_id: impl Into<String>) -> Self {
        Self::DynamicTexture {
            namespace: namespace.into(),
            target_id: target_id.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProductSurfacePrimitive {
    pub source: ProductSurfaceTextureBindingSource,
    pub rect: UiRect,
    pub uv_rect: UiRect,
    pub tint: UiPaint,
    pub alpha_mode: ProductSurfaceAlphaMode,
    pub sort_key: UiSortKey,
}

impl ProductSurfacePrimitive {
    pub fn new(
        source: ProductSurfaceTextureBindingSource,
        rect: UiRect,
        uv_rect: UiRect,
        tint: UiPaint,
        alpha_mode: ProductSurfaceAlphaMode,
        sort_key: UiSortKey,
    ) -> Self {
        Self {
            source,
            rect,
            uv_rect,
            tint,
            alpha_mode,
            sort_key,
        }
    }
}
