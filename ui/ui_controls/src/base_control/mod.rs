//! File: domain/ui/ui_controls/src/base_control/mod.rs
//! Crate: ui_controls
//! Purpose: UI-local Phase 11 base-control contribution and lowering model.

mod compiler;
mod contribution;
mod def;
mod field_group;
mod lowering;
mod plugin;
mod preset;
mod projection;
mod theme_group;

pub use compiler::*;
pub use contribution::*;
pub use def::*;
pub use field_group::*;
pub use plugin::*;
pub use preset::*;
pub use projection::*;
pub use theme_group::*;

use crate::{
    ACTION_PROMPT_CONTROL_KIND_ID, BUTTON_CONTROL_KIND_ID, COLOR_PICKER_CONTROL_KIND_ID,
    ControlCatalogIndex, INSPECTOR_FIELD_CONTROL_KIND_ID, LABEL_CONTROL_KIND_ID,
    LIST_VIEW_CONTROL_KIND_ID, SURFACE2D_CONTROL_KIND_ID, TABLE_VIEW_CONTROL_KIND_ID,
    TREE_VIEW_CONTROL_KIND_ID,
};

pub const BASE_CONTROL_TARGET_KIND_IDS: [&str; 9] = [
    LABEL_CONTROL_KIND_ID,
    BUTTON_CONTROL_KIND_ID,
    INSPECTOR_FIELD_CONTROL_KIND_ID,
    COLOR_PICKER_CONTROL_KIND_ID,
    ACTION_PROMPT_CONTROL_KIND_ID,
    LIST_VIEW_CONTROL_KIND_ID,
    TREE_VIEW_CONTROL_KIND_ID,
    TABLE_VIEW_CONTROL_KIND_ID,
    SURFACE2D_CONTROL_KIND_ID,
];

pub type ControlCatalog = ControlCatalogIndex;
