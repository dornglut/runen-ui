//! File: domain/ui/ui_controls/src/lib.rs
//! Crate: ui_controls

pub mod accessibility;
pub mod action_prompt;
pub mod base_control;
pub mod button;
pub mod catalog;
pub mod color_picker;
pub mod diagnostics;
pub mod editable_text;
pub mod generic_text;
pub mod input;
pub mod inspector_field;
pub mod interaction;
pub mod kernel;
pub mod label;
pub mod layout;
pub mod list_view;
pub mod migration;
pub mod overlay;
pub mod package;
pub mod registry;
pub mod schema;
pub mod state;
pub mod surface2d;
pub mod table_view;
pub mod theme;
pub mod tree_view;

pub use accessibility::*;
pub use action_prompt::ACTION_PROMPT_CONTROL_KIND_ID;
pub use base_control::*;
pub use button::BUTTON_CONTROL_KIND_ID;
pub use catalog::*;
pub use color_picker::COLOR_PICKER_CONTROL_KIND_ID;
pub use diagnostics::*;
pub use editable_text::*;
pub use generic_text::*;
pub use inspector_field::INSPECTOR_FIELD_CONTROL_KIND_ID;
pub use interaction::*;
pub use kernel::*;
pub use label::LABEL_CONTROL_KIND_ID;
pub use list_view::LIST_VIEW_CONTROL_KIND_ID;
pub use migration::*;
pub use overlay::*;
pub use package::*;
pub use registry::*;
pub use schema::*;
pub use state::*;
pub use surface2d::*;
pub use table_view::TABLE_VIEW_CONTROL_KIND_ID;
pub use tree_view::TREE_VIEW_CONTROL_KIND_ID;

pub const RUNENWERK_CONTROL_PACKAGE_ID: &str = "runenwerk.ui.controls";
pub const RUNENWERK_CONTROL_TARGET_EDITOR: &str = "runenwerk.ui.target.editor";

pub fn runenwerk_control_package() -> ControlPackageDescriptor {
    BaseControlsPlugin::new().package()
}
