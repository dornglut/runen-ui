//! File: domain/ui/ui_render_data/src/frame/ui_surface.rs
//! Purpose: A UI render surface containing ordered layers.

use ui_math::UiSize;

use crate::frame::{UiLayer, UiSurfaceId};

#[derive(Debug, Clone, PartialEq)]
pub struct UiSurface {
    pub id: UiSurfaceId,
    pub size: UiSize,
    pub layers: Vec<UiLayer>,
}

impl UiSurface {
    pub fn new(id: UiSurfaceId, size: UiSize) -> Self {
        Self {
            id,
            size,
            layers: Vec::new(),
        }
    }

    pub fn with_layers(id: UiSurfaceId, size: UiSize, layers: Vec<UiLayer>) -> Self {
        Self { id, size, layers }
    }

    pub fn push_layer(&mut self, layer: UiLayer) {
        self.layers.push(layer);
    }

    pub fn is_empty(&self) -> bool {
        self.layers.iter().all(UiLayer::is_empty)
    }
}
