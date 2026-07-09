//! File: domain/ui/ui_render_data/src/frame/ui_layer.rs
//! Purpose: Ordered group of UI primitives.

use crate::UiPrimitive;
use crate::frame::UiLayerId;

#[derive(Debug, Clone, PartialEq)]
pub struct UiLayer {
    pub id: UiLayerId,
    pub primitives: Vec<UiPrimitive>,
}

impl UiLayer {
    pub fn new(id: UiLayerId) -> Self {
        Self {
            id,
            primitives: Vec::new(),
        }
    }

    pub fn with_primitives(id: UiLayerId, primitives: Vec<UiPrimitive>) -> Self {
        Self { id, primitives }
    }

    pub fn push(&mut self, primitive: UiPrimitive) {
        self.primitives.push(primitive);
    }

    pub fn is_empty(&self) -> bool {
        self.primitives.is_empty()
    }
}
