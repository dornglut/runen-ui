//! File: domain/ui/ui_controls/src/action_prompt/mod.rs
//! Crate: ui_controls

use ui_program::RouteCapability;
use ui_schema::UiSchemaShape;

use crate::{
    ControlCompiler, ControlContribution, ControlDef, ControlField, ControlFieldGroup,
    ControlModuleDescriptor, ControlPreset, ControlStyleRole, ControlThemeGroup,
    ControlVisualState,
};

pub const ACTION_PROMPT_CONTROL_KIND_ID: &str = "runenwerk.ui.controls.action-prompt";

pub fn control_contribution() -> ControlContribution {
    ControlDef::builder(
        "action-prompt",
        "ActionPrompt",
        ControlPreset::ActionPrompt,
        RouteCapability::new("runenwerk.ui.controls.prompt.answer"),
    )
    .with_field_group(ControlFieldGroup::properties([
        ControlField::required("title", UiSchemaShape::String),
        ControlField::required("primary_route", UiSchemaShape::RouteRef),
    ]))
    .with_field_group(ControlFieldGroup::state([
        ControlField::optional("open", UiSchemaShape::Bool),
        ControlField::optional("focused_action", UiSchemaShape::String),
    ]))
    .with_field_group(ControlFieldGroup::event_payload([
        ControlField::required("route", UiSchemaShape::RouteRef),
        ControlField::required("accepted", UiSchemaShape::Bool),
    ]))
    .with_theme_group(
        ControlThemeGroup::surface("action-prompt")
            .with_style(ControlStyleRole::Container, "surface")
            .with_optional_visual_state(ControlVisualState::Focused),
    )
    .build_contribution()
}

pub fn control_module() -> ControlModuleDescriptor {
    ControlCompiler::new().compile_module(&control_contribution())
}
