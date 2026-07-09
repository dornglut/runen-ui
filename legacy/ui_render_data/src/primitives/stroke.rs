//! File: domain/ui/ui_render_data/src/primitives/stroke.rs
//! Purpose: Immediate screen-space stroke primitive.

use ui_math::{UiPoint, UiRect};

use crate::{UiDrawKey, UiPaint, UiSortKey};

#[derive(Debug, Clone, PartialEq)]
pub struct StrokePrimitive {
    pub points: Vec<UiPoint>,
    pub width: f32,
    pub paint: UiPaint,
    pub draw_key: UiDrawKey,
    pub sort_key: UiSortKey,
    pub clip: Option<UiRect>,
}

impl StrokePrimitive {
    pub fn new(
        points: impl IntoIterator<Item = UiPoint>,
        width: f32,
        paint: UiPaint,
        draw_key: UiDrawKey,
        sort_key: UiSortKey,
    ) -> Self {
        Self {
            points: points.into_iter().collect(),
            width,
            paint,
            draw_key,
            sort_key,
            clip: None,
        }
    }

    pub fn with_clip(mut self, clip: UiRect) -> Self {
        self.clip = Some(clip);
        self
    }
}
