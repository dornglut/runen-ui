//! File: domain/ui/ui_layout/src/split.rs
//! Purpose: Basic split layout for editor panels.

use ui_math::{Axis, UiRect};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SplitLayout {
    pub axis: Axis,
    pub ratio: f32,
    pub gap: f32,
}

impl SplitLayout {
    pub fn new(axis: Axis, ratio: f32, gap: f32) -> Self {
        Self {
            axis,
            ratio: ratio.clamp(0.0, 1.0),
            gap: gap.max(0.0),
        }
    }

    pub fn arrange(&self, bounds: UiRect) -> (UiRect, UiRect) {
        match self.axis {
            Axis::Horizontal => {
                let total = (bounds.width - self.gap).max(0.0);
                let first_width = total * self.ratio;
                let second_width = total - first_width;

                (
                    UiRect::new(bounds.x, bounds.y, first_width, bounds.height),
                    UiRect::new(
                        bounds.x + first_width + self.gap,
                        bounds.y,
                        second_width,
                        bounds.height,
                    ),
                )
            }
            Axis::Vertical => {
                let total = (bounds.height - self.gap).max(0.0);
                let first_height = total * self.ratio;
                let second_height = total - first_height;

                (
                    UiRect::new(bounds.x, bounds.y, bounds.width, first_height),
                    UiRect::new(
                        bounds.x,
                        bounds.y + first_height + self.gap,
                        bounds.width,
                        second_height,
                    ),
                )
            }
        }
    }
}
