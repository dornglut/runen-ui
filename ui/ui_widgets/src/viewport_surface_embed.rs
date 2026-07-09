//! File: domain/ui/ui_runtime/src/widgets/viewport_surface_embed.rs
//! Purpose: Viewport surface embed widget constructor.

use ui_render_data::ViewportSurfaceEmbedSlotId;

use crate::{UiNode, UiNodeKind, ViewportSurfaceEmbedNode, WidgetId};

pub fn viewport_surface_embed(
    id: WidgetId,
    viewport_id: u64,
    slot: ViewportSurfaceEmbedSlotId,
) -> UiNode {
    UiNode::new(
        id,
        UiNodeKind::ViewportSurfaceEmbed(ViewportSurfaceEmbedNode::new(viewport_id, slot)),
    )
}
