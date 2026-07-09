//! File: domain/ui/ui_render_data/src/primitives/rect.rs
//! Purpose: Filled rectangle primitive.

use ui_math::UiRect;

use crate::{UiDrawKey, UiPaint, UiSortKey};

#[derive(Debug, Clone, PartialEq)]
pub struct RectPrimitive {
    pub rect: UiRect,
    pub radius: f32,
    pub paint: UiPaint,
    pub draw_key: UiDrawKey,
    pub sort_key: UiSortKey,
}

impl RectPrimitive {
    pub fn new(
        rect: UiRect,
        radius: f32,
        paint: UiPaint,
        draw_key: UiDrawKey,
        sort_key: UiSortKey,
    ) -> Self {
        Self {
            rect,
            radius,
            paint,
            draw_key,
            sort_key,
        }
    }
}
