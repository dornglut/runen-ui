//! File: domain/ui/ui_theme/src/color.rs
//! Purpose: Theme color tokens.

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct UiColor {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

impl UiColor {
    pub const fn new(r: f32, g: f32, b: f32, a: f32) -> Self {
        Self { r, g, b, a }
    }
}
