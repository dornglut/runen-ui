//! File: domain/ui/ui_render_data/src/frame/ui_frame.rs
//! Purpose: Renderer-facing UI frame payload.

use crate::frame::UiSurface;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct UiFrame {
    pub surfaces: Vec<UiSurface>,
}

impl UiFrame {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_surfaces(surfaces: Vec<UiSurface>) -> Self {
        Self { surfaces }
    }

    pub fn push_surface(&mut self, surface: UiSurface) {
        self.surfaces.push(surface);
    }

    pub fn is_empty(&self) -> bool {
        self.surfaces.iter().all(UiSurface::is_empty)
    }
}
