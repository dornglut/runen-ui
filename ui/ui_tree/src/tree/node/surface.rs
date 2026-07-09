//! Surface and leaf node contracts.

use ui_layout::SizePolicy;
use ui_math::{Axis, UiRect, UiSize};
use ui_render_data::{
    ProductSurfaceAlphaMode, ProductSurfaceTextureBindingSource, UiDrawKey, UiPaint,
    ViewportSurfaceEmbedSlotId,
};
use ui_theme::UiColor;

#[derive(Debug, Clone, PartialEq)]
pub struct SpacerNode {
    pub min_size: UiSize,
}

impl SpacerNode {
    pub const fn new(min_size: UiSize) -> Self {
        Self { min_size }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct DividerNode {
    pub axis: Axis,
    pub thickness: f32,
    pub length_policy: SizePolicy,
    pub color: UiColor,
}

impl DividerNode {
    pub fn new(axis: Axis, thickness: f32, length_policy: SizePolicy, color: UiColor) -> Self {
        Self {
            axis,
            thickness: thickness.max(0.0),
            length_policy,
            color,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ImageNode {
    pub draw_key: UiDrawKey,
    pub uv_rect: UiRect,
    pub tint: UiPaint,
    pub min_size: UiSize,
}

impl ImageNode {
    pub const fn new(
        draw_key: UiDrawKey,
        uv_rect: UiRect,
        tint: UiPaint,
        min_size: UiSize,
    ) -> Self {
        Self {
            draw_key,
            uv_rect,
            tint,
            min_size,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProductSurfaceNode {
    pub source: ProductSurfaceTextureBindingSource,
    pub uv_rect: UiRect,
    pub tint: UiPaint,
    pub alpha_mode: ProductSurfaceAlphaMode,
    pub min_size: UiSize,
}

impl ProductSurfaceNode {
    pub const fn new(
        source: ProductSurfaceTextureBindingSource,
        uv_rect: UiRect,
        tint: UiPaint,
        alpha_mode: ProductSurfaceAlphaMode,
        min_size: UiSize,
    ) -> Self {
        Self {
            source,
            uv_rect,
            tint,
            alpha_mode,
            min_size,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ViewportSurfaceEmbedNode {
    pub viewport_id: u64,
    pub slot: ViewportSurfaceEmbedSlotId,
    pub min_size: UiSize,
}

impl ViewportSurfaceEmbedNode {
    pub fn new(viewport_id: u64, slot: ViewportSurfaceEmbedSlotId) -> Self {
        Self {
            viewport_id,
            slot,
            min_size: UiSize::new(64.0, 64.0),
        }
    }
}
