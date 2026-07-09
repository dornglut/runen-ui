use super::super::{ControlDef, ControlPreset};
use crate::{ControlKindId, ControlOverlayDescriptor};

pub(crate) fn lower_layering_support(
    def: &ControlDef,
    kind_id: ControlKindId,
) -> ControlOverlayDescriptor {
    match def.preset() {
        ControlPreset::Label => ControlOverlayDescriptor::tooltip_on_hover(
            kind_id,
            "anchor.label.tooltip",
            "tooltip.label",
        ),
        ControlPreset::Button => {
            ControlOverlayDescriptor::popup_on_press(kind_id, "anchor.button.popup", "popup.button")
        }
        ControlPreset::ActionPrompt => ControlOverlayDescriptor::menu_on_press(
            kind_id,
            "anchor.action-prompt.menu",
            "menu.action-prompt",
        ),
        ControlPreset::InspectorField => {
            ControlOverlayDescriptor::focus_containing_overlay_on_press(
                kind_id,
                "anchor.inspector-field.focus",
                "focus.inspector-field",
            )
        }
        ControlPreset::ColorPicker => ControlOverlayDescriptor::picker_popup_on_press(
            kind_id,
            "anchor.color-picker.picker",
            "picker.color",
        ),
        ControlPreset::ListView => ControlOverlayDescriptor::dropdown_on_press(
            kind_id,
            "anchor.list-view.dropdown",
            "dropdown.list-view.options",
        ),
        ControlPreset::TreeView => ControlOverlayDescriptor::menu_on_press(
            kind_id,
            "anchor.tree-view.menu",
            "menu.tree-view",
        ),
        ControlPreset::TableView => ControlOverlayDescriptor::menu_on_press(
            kind_id,
            "anchor.table-view.menu",
            "menu.table-view",
        ),
        ControlPreset::Surface2D => ControlOverlayDescriptor::tooltip_on_hover(
            kind_id,
            "anchor.surface2d.diagnostic",
            "tooltip.surface2d.diagnostic",
        ),
    }
}
