//! File: domain/ui/ui_math/src/vector.rs
//! Purpose: Shared 2D vector/delta type.

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct UiVector {
    pub x: f32,
    pub y: f32,
}

impl UiVector {
    pub const ZERO: Self = Self { x: 0.0, y: 0.0 };

    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}
