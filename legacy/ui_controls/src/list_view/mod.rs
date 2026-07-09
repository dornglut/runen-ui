//! File: domain/ui/ui_controls/src/list_view/mod.rs
//! Crate: ui_controls

use ui_program::RouteCapability;
use ui_schema::UiSchemaShape;

use crate::{
    ControlCompiler, ControlContribution, ControlDef, ControlField, ControlFieldGroup,
    ControlModuleDescriptor, ControlPreset, ControlStyleRole, ControlThemeGroup,
    ControlVisualState,
};

pub const LIST_VIEW_CONTROL_KIND_ID: &str = "runenwerk.ui.controls.list-view";

pub fn control_contribution() -> ControlContribution {
    ControlDef::builder(
        "list-view",
        "ListView",
        ControlPreset::ListView,
        RouteCapability::new("runenwerk.ui.controls.list.select"),
    )
    .with_field_group(ControlFieldGroup::properties([
        ControlField::required("items", UiSchemaShape::list(UiSchemaShape::Object)),
        ControlField::optional("selection_route", UiSchemaShape::RouteRef),
    ]))
    .with_field_group(ControlFieldGroup::state([
        ControlField::optional("selected_index", UiSchemaShape::UnsignedInteger),
        ControlField::optional("scroll_offset", UiSchemaShape::Number),
    ]))
    .with_field_group(ControlFieldGroup::event_payload([
        ControlField::required("route", UiSchemaShape::RouteRef),
        ControlField::required("selected_index", UiSchemaShape::UnsignedInteger),
    ]))
    .with_theme_group(
        ControlThemeGroup::surface("list-view")
            .with_style(ControlStyleRole::Background, "surface")
            .with_optional_visual_state(ControlVisualState::Focused)
            .with_optional_visual_state(ControlVisualState::Selected)
            .with_optional_visual_state(ControlVisualState::Loading),
    )
    .build_contribution()
}

pub fn control_module() -> ControlModuleDescriptor {
    ControlCompiler::new().compile_module(&control_contribution())
}
