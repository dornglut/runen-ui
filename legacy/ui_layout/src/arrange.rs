//! File: domain/ui/ui_layout/src/arrange.rs
//! Purpose: Arrangement results.

use ui_math::UiRect;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ArrangedRect {
    pub rect: UiRect,
}

impl ArrangedRect {
    pub fn new(rect: UiRect) -> Self {
        Self { rect }
    }
}
