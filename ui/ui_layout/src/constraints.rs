//! File: domain/ui/ui_layout/src/constraints.rs
//! Purpose: Shared layout constraint model.

use ui_math::UiSize;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LayoutConstraints {
    pub min: UiSize,
    pub max: UiSize,
}

impl LayoutConstraints {
    pub fn new(min: UiSize, max: UiSize) -> Self {
        Self { min, max }
    }

    pub fn tight(size: UiSize) -> Self {
        Self {
            min: size,
            max: size,
        }
    }

    pub fn loose(max: UiSize) -> Self {
        Self {
            min: UiSize::ZERO,
            max,
        }
    }

    pub fn constrain(&self, size: UiSize) -> UiSize {
        size.clamp(self.min, self.max)
    }
}
