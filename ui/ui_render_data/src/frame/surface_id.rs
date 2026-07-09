//! File: domain/ui/ui_render_data/src/frame/surface_id.rs
//! Purpose: Stable identifier for a UI render surface.

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct UiSurfaceId(pub u64);
