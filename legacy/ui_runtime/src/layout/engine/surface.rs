//! File: domain/ui/ui_runtime/src/layout/engine/surface.rs
//! Purpose: Simple surface and embed primitive layout for retained UI nodes.

use ui_math::{UiRect, UiSize};

use crate::{
    ComputedLayout, ComputedLayoutMap, GraphCanvasNode, ImageNode, ProductSurfaceNode, UiNode,
    ViewportSurfaceEmbedNode,
};

pub(super) fn layout_image(
    node: &UiNode,
    image: &ImageNode,
    bounds: UiRect,
    out: &mut ComputedLayoutMap,
) -> UiSize {
    let measured_size = UiSize::new(
        bounds.width.max(image.min_size.width),
        bounds.height.max(image.min_size.height),
    );
    out.insert(node.id, ComputedLayout::new(bounds, bounds, measured_size));
    measured_size
}
pub(super) fn layout_product_surface(
    node: &UiNode,
    surface: &ProductSurfaceNode,
    bounds: UiRect,
    out: &mut ComputedLayoutMap,
) -> UiSize {
    let measured_size = UiSize::new(
        bounds.width.max(surface.min_size.width),
        bounds.height.max(surface.min_size.height),
    );
    out.insert(node.id, ComputedLayout::new(bounds, bounds, measured_size));
    measured_size
}
pub(super) fn layout_graph_canvas(
    node: &UiNode,
    graph_canvas: &GraphCanvasNode,
    bounds: UiRect,
    out: &mut ComputedLayoutMap,
) -> UiSize {
    let measured_size = UiSize::new(
        bounds.width.max(graph_canvas.min_size.width),
        bounds.height.max(graph_canvas.min_size.height),
    );
    out.insert(node.id, ComputedLayout::new(bounds, bounds, measured_size));
    measured_size
}
pub(super) fn layout_viewport_surface_embed(
    node: &UiNode,
    embed: &ViewportSurfaceEmbedNode,
    bounds: UiRect,
    out: &mut ComputedLayoutMap,
) -> UiSize {
    let measured_size = UiSize::new(
        bounds.width.max(embed.min_size.width),
        bounds.height.max(embed.min_size.height),
    );
    out.insert(node.id, ComputedLayout::new(bounds, bounds, measured_size));
    measured_size
}
