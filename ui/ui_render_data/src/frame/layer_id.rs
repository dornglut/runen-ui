//! File: domain/ui/ui_render_data/src/frame/layer_id.rs
//! Purpose: Stable identifier for a layer within a UI surface.

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UiLayerId(pub u64);
