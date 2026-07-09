//! File: domain/ui/ui_layout/src/measure.rs
//! Purpose: Measurement results.

use ui_math::UiSize;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct MeasuredSize {
    pub size: UiSize,
}

impl MeasuredSize {
    pub fn new(size: UiSize) -> Self {
        Self { size }
    }
}
