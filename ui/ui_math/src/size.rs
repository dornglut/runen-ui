//! File: domain/ui/ui_math/src/size.rs
//! Purpose: Shared 2D size type.

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct UiSize {
    pub width: f32,
    pub height: f32,
}

impl UiSize {
    pub const ZERO: Self = Self {
        width: 0.0,
        height: 0.0,
    };

    pub const fn new(width: f32, height: f32) -> Self {
        Self { width, height }
    }

    pub fn clamp(self, min: Self, max: Self) -> Self {
        Self {
            width: self.width.clamp(min.width, max.width),
            height: self.height.clamp(min.height, max.height),
        }
    }
}
