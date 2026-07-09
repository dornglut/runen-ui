//! File: domain/ui/ui_render_data/src/batching/draw_key.rs
//! Purpose: Stable batching id for UI primitives.

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UiDrawKey {
    pub material_id: u64,
    pub texture_id: Option<u64>,
}

impl UiDrawKey {
    pub const fn new(material_id: u64, texture_id: Option<u64>) -> Self {
        Self {
            material_id,
            texture_id,
        }
    }
}
