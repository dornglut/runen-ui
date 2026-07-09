//! File: domain/ui/ui_render_data/src/primitives/border.rs
//! Purpose: Rectangle border primitive.

use ui_math::UiRect;

use crate::{UiDrawKey, UiPaint, UiSortKey};

#[derive(Debug, Clone, PartialEq)]
pub struct BorderPrimitive {
    pub rect: UiRect,
    pub radius: f32,
    pub width: f32,
    pub paint: UiPaint,
    pub draw_key: UiDrawKey,
    pub sort_key: UiSortKey,
}

impl BorderPrimitive {
    pub fn new(
        rect: UiRect,
        radius: f32,
        width: f32,
        paint: UiPaint,
        draw_key: UiDrawKey,
        sort_key: UiSortKey,
    ) -> Self {
        Self {
            rect,
            radius,
            width,
            paint,
            draw_key,
            sort_key,
        }
    }
}
