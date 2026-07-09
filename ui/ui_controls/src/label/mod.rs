//! File: domain/ui/ui_controls/src/label/mod.rs
//! Crate: ui_controls

use ui_program::RouteCapability;
use ui_schema::UiSchemaShape;

use crate::{
    ControlCompiler, ControlContribution, ControlDef, ControlField, ControlFieldGroup,
    ControlModuleDescriptor, ControlPreset, ControlThemeGroup,
};

pub const LABEL_CONTROL_KIND_ID: &str = "runenwerk.ui.controls.label";

pub fn control_contribution() -> ControlContribution {
    ControlDef::builder(
        "label",
        "Label",
        ControlPreset::Label,
        RouteCapability::new("runenwerk.ui.controls.read"),
    )
    .with_field_group(ControlFieldGroup::properties([
        ControlField::required("text", UiSchemaShape::String),
        ControlField::optional("style_slot", UiSchemaShape::String),
    ]))
    .with_field_group(ControlFieldGroup::state([
        ControlField::optional("measured_width", UiSchemaShape::Number),
        ControlField::optional("measured_height", UiSchemaShape::Number),
    ]))
    .with_field_group(ControlFieldGroup::event_payload([ControlField::optional(
        "route",
        UiSchemaShape::RouteRef,
    )]))
    .with_theme_group(ControlThemeGroup::base("label"))
    .build_contribution()
}

pub fn control_module() -> ControlModuleDescriptor {
    ControlCompiler::new().compile_module(&control_contribution())
}
