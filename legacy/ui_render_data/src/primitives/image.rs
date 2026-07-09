//! File: domain/ui/ui_render_data/src/primitives/image.rs
//! Purpose: Textured rectangle primitive.

use ui_math::UiRect;

use crate::{UiDrawKey, UiPaint, UiSortKey};

#[derive(Debug, Clone, PartialEq)]
pub struct ImagePrimitive {
    pub rect: UiRect,
    pub uv_rect: UiRect,
    pub tint: UiPaint,
    pub draw_key: UiDrawKey,
    pub sort_key: UiSortKey,
}

impl ImagePrimitive {
    pub fn new(
        rect: UiRect,
        uv_rect: UiRect,
        tint: UiPaint,
        draw_key: UiDrawKey,
        sort_key: UiSortKey,
    ) -> Self {
        Self {
            rect,
            uv_rect,
            tint,
            draw_key,
            sort_key,
        }
    }
}
