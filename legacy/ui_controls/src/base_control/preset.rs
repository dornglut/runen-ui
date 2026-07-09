//! File: domain/ui/ui_controls/src/base_control/preset.rs
//! Crate: ui_controls

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ControlPreset {
    Label,
    Button,
    InspectorField,
    ColorPicker,
    ActionPrompt,
    ListView,
    TreeView,
    TableView,
    Surface2D,
}

impl ControlPreset {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Label => "label",
            Self::Button => "button",
            Self::InspectorField => "inspector-field",
            Self::ColorPicker => "color-picker",
            Self::ActionPrompt => "action-prompt",
            Self::ListView => "list-view",
            Self::TreeView => "tree-view",
            Self::TableView => "table-view",
            Self::Surface2D => "surface2d",
        }
    }
}
