//! File: domain/ui/ui_runtime/src/output/emit/surface.rs
//! Purpose: Leaf and external-surface primitive emission for UI frame output.

use crate::{DividerNode, ImageNode, ProductSurfaceNode, ViewportSurfaceEmbedNode};
use ui_math::{UiRect, UiSize};
use ui_render_data::{
    ImagePrimitive, ProductSurfacePrimitive, RectPrimitive, UiLayer, UiPaint, UiPrimitive,
    ViewportSurfaceEmbedPrimitive,
};

use super::super::primitives::{default_draw_key, paint_from_color, sort_key};

pub(crate) fn emit_viewport_surface_embed(
    embed: &ViewportSurfaceEmbedNode,
    bounds: UiRect,
    _surface_size: UiSize,
    layer: &mut UiLayer,
    depth: u32,
    primitive_order: &mut u32,
) {
    layer.push(UiPrimitive::ViewportSurfaceEmbed(
        ViewportSurfaceEmbedPrimitive::new(
            embed.viewport_id,
            embed.slot,
            bounds,
            UiRect::new(0.0, 0.0, 1.0, 1.0),
            UiPaint::rgba(1.0, 1.0, 1.0, 1.0),
            sort_key(depth, *primitive_order),
        ),
    ));
    *primitive_order += 1;
}

pub(crate) fn emit_divider(
    divider: &DividerNode,
    bounds: UiRect,
    layer: &mut UiLayer,
    depth: u32,
    primitive_order: &mut u32,
) {
    layer.push(UiPrimitive::Rect(RectPrimitive::new(
        bounds,
        0.0,
        paint_from_color(divider.color),
        default_draw_key(),
        sort_key(depth, *primitive_order),
    )));
    *primitive_order += 1;
}

pub(crate) fn emit_image(
    image: &ImageNode,
    bounds: UiRect,
    layer: &mut UiLayer,
    depth: u32,
    primitive_order: &mut u32,
) {
    layer.push(UiPrimitive::Image(ImagePrimitive::new(
        bounds,
        image.uv_rect,
        image.tint,
        image.draw_key,
        sort_key(depth, *primitive_order),
    )));
    *primitive_order += 1;
}

pub(crate) fn emit_product_surface(
    surface: &ProductSurfaceNode,
    bounds: UiRect,
    layer: &mut UiLayer,
    depth: u32,
    primitive_order: &mut u32,
) {
    layer.push(UiPrimitive::ProductSurface(ProductSurfacePrimitive::new(
        surface.source.clone(),
        bounds,
        surface.uv_rect,
        surface.tint,
        surface.alpha_mode,
        sort_key(depth, *primitive_order),
    )));
    *primitive_order += 1;
}
