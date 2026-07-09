//! File: domain/ui/ui_render_data/src/batching/sort_key.rs
//! Purpose: Stable render ordering key for UI primitives.

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UiSortKey {
    pub surface_order: u32,
    pub layer_order: u32,
    pub primitive_order: u32,
}

impl UiSortKey {
    pub const fn new(surface_order: u32, layer_order: u32, primitive_order: u32) -> Self {
        Self {
            surface_order,
            layer_order,
            primitive_order,
        }
    }
}
