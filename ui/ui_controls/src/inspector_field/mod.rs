//! File: domain/ui/ui_controls/src/inspector_field/mod.rs
//! Crate: ui_controls

use ui_program::RouteCapability;
use ui_schema::UiSchemaShape;

use crate::{
    ControlCompiler, ControlContribution, ControlDef, ControlField, ControlFieldGroup,
    ControlModuleDescriptor, ControlPreset, ControlStyleRole, ControlThemeGroup,
    ControlThemeTokenKind, ControlThemeTokenRole, ControlVisualState,
};

pub const INSPECTOR_FIELD_CONTROL_KIND_ID: &str = "runenwerk.ui.controls.inspector-field";

pub fn control_contribution() -> ControlContribution {
    ControlDef::builder(
        "inspector-field",
        "InspectorField",
        ControlPreset::InspectorField,
        RouteCapability::new("runenwerk.ui.controls.inspect"),
    )
    .with_field_group(ControlFieldGroup::properties([
        ControlField::required("label", UiSchemaShape::String),
        ControlField::required("binding", UiSchemaShape::StableIdRef),
    ]))
    .with_field_group(ControlFieldGroup::state([
        ControlField::optional("preview_value", UiSchemaShape::Object),
        ControlField::optional("dirty", UiSchemaShape::Bool),
    ]))
    .with_field_group(ControlFieldGroup::event_payload([
        ControlField::required("route", UiSchemaShape::RouteRef),
        ControlField::required("value", UiSchemaShape::Object),
    ]))
    .with_theme_group(
        ControlThemeGroup::surface("inspector-field")
            .with_token(
                "value",
                ControlThemeTokenKind::Color,
                ControlThemeTokenRole::Text,
            )
            .with_style(ControlStyleRole::Value, "value")
            .with_optional_visual_state(ControlVisualState::Focused)
            .with_optional_visual_state(ControlVisualState::Error)
            .with_optional_visual_state(ControlVisualState::ReadOnly),
    )
    .build_contribution()
}

pub fn control_module() -> ControlModuleDescriptor {
    ControlCompiler::new().compile_module(&control_contribution())
}
