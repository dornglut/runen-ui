//! File: domain/ui/ui_runtime/src/output/build_ui_frame.rs
//! Purpose: Convert retained tree + computed layout into UiFrame.

use crate::{ComputedLayoutMap, UiTree};

#[cfg(test)]
pub(crate) use super::emit::containers::scrollbar_geometry;
pub(crate) use super::emit::containers::{
    ScrollbarGeometry, scrollbar_geometries, scrollbar_geometry_for_axis,
};
pub use super::interaction_visual::InteractionVisualState;
#[cfg(test)]
pub(super) use super::layer::POPUP_LAYER_ORDER;
use super::{layer::BASE_LAYER_ORDER, traversal::emit_node};
use ui_math::UiSize;
use ui_render_data::{UiFrame, UiLayer, UiLayerId, UiSurface, UiSurfaceId};
use ui_text::{AtlasTextLayouter, FontAtlasSource};

pub fn build_ui_frame(
    tree: &UiTree,
    layouts: &ComputedLayoutMap,
    surface_size: UiSize,
    interaction_state: InteractionVisualState,
    atlas_source: &dyn FontAtlasSource,
) -> UiFrame {
    let mut layer = UiLayer::new(UiLayerId(0));
    let mut primitive_order = 0u32;
    let layouter = AtlasTextLayouter;

    emit_node(
        tree,
        &tree.root,
        layouts,
        &mut layer,
        surface_size,
        atlas_source,
        &layouter,
        interaction_state,
        BASE_LAYER_ORDER,
        &mut primitive_order,
    );

    UiFrame::with_surfaces(vec![UiSurface::with_layers(
        UiSurfaceId(0),
        surface_size,
        vec![layer],
    )])
}

#[cfg(test)]
mod tests;
