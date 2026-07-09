//! File: domain/ui/ui_math/src/pointer
//! Purpose: Shared 2D point type.

use crate::UiVector;

#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct UiPoint {
    pub x: f32,
    pub y: f32,
}

impl UiPoint {
    pub const ZERO: Self = Self { x: 0.0, y: 0.0 };

    pub const fn new(x: f32, y: f32) -> Self {
        Self { x, y }
    }
}

impl core::ops::Add<UiVector> for UiPoint {
    type Output = UiPoint;

    fn add(self, rhs: UiVector) -> Self::Output {
        UiPoint::new(self.x + rhs.x, self.y + rhs.y)
    }
}

impl core::ops::Sub for UiPoint {
    type Output = UiVector;

    fn sub(self, rhs: Self) -> Self::Output {
        UiVector::new(self.x - rhs.x, self.y - rhs.y)
    }
}
