//! File: domain/ui/ui_controls/src/table_view/mod.rs
//! Crate: ui_controls

use ui_program::RouteCapability;
use ui_schema::UiSchemaShape;

use crate::{
    ControlCompiler, ControlContribution, ControlDef, ControlField, ControlFieldGroup,
    ControlModuleDescriptor, ControlPreset, ControlStyleRole, ControlThemeGroup,
    ControlVisualState,
};

pub const TABLE_VIEW_CONTROL_KIND_ID: &str = "runenwerk.ui.controls.table-view";

pub fn control_contribution() -> ControlContribution {
    ControlDef::builder(
        "table-view",
        "TableView",
        ControlPreset::TableView,
        RouteCapability::new("runenwerk.ui.controls.table.select"),
    )
    .with_field_group(ControlFieldGroup::properties([
        ControlField::required("columns", UiSchemaShape::list(UiSchemaShape::Object)),
        ControlField::required("rows", UiSchemaShape::list(UiSchemaShape::Object)),
    ]))
    .with_field_group(ControlFieldGroup::state([
        ControlField::optional("sort_column", UiSchemaShape::String),
        ControlField::optional("selected_row", UiSchemaShape::UnsignedInteger),
    ]))
    .with_field_group(ControlFieldGroup::event_payload([
        ControlField::required("route", UiSchemaShape::RouteRef),
        ControlField::required("row", UiSchemaShape::UnsignedInteger),
    ]))
    .with_theme_group(
        ControlThemeGroup::surface("table-view")
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
