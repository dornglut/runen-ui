//! File: domain/ui/ui_widgets/src/image.rs
//! Purpose: Image widget constructor.

use crate::{ImageNode, UiNode, UiNodeKind, WidgetId};
use ui_math::{UiRect, UiSize};
use ui_render_data::{UiDrawKey, UiPaint};

pub fn image(
    id: WidgetId,
    draw_key: UiDrawKey,
    uv_rect: UiRect,
    tint: UiPaint,
    min_size: UiSize,
) -> UiNode {
    UiNode::new(
        id,
        UiNodeKind::Image(ImageNode::new(draw_key, uv_rect, tint, min_size)),
    )
}
