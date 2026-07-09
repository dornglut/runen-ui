use ui_program::RouteCapability;
use ui_schema::UiSchemaShape;

use crate::{
    ControlCompiler, ControlContribution, ControlDef, ControlField, ControlFieldGroup,
    ControlModuleDescriptor, ControlPreset, ControlStyleRole, ControlThemeGroup,
    ControlVisualState,
};

pub fn control_contribution() -> ControlContribution {
    ControlDef::builder(
        "surface2d",
        "Surface2D",
        ControlPreset::Surface2D,
        RouteCapability::new("runenwerk.ui.controls.surface2d.navigate"),
    )
    .with_description("Reusable renderer-neutral 2D coordinate and navigation surface descriptor with package, catalog, inspection, runtime proof, and static mount evidence.")
    .with_category("base-control")
    .with_tag("surface2d")
    .with_tag("coordinate-navigation")
    .with_field_group(ControlFieldGroup::properties([
        ControlField::required("surface_id", UiSchemaShape::StableIdRef),
        ControlField::required("content_width", UiSchemaShape::Number),
        ControlField::required("content_height", UiSchemaShape::Number),
        ControlField::required("viewport_width", UiSchemaShape::Number),
        ControlField::required("viewport_height", UiSchemaShape::Number),
    ]))
    .with_field_group(ControlFieldGroup::state([
        ControlField::optional("pan_x", UiSchemaShape::Number),
        ControlField::optional("pan_y", UiSchemaShape::Number),
        ControlField::optional("zoom", UiSchemaShape::Number),
        ControlField::optional("focused", UiSchemaShape::Bool),
    ]))
    .with_field_group(ControlFieldGroup::event_payload([
        ControlField::required("intent", UiSchemaShape::String),
        ControlField::optional("screen_x", UiSchemaShape::Number),
        ControlField::optional("screen_y", UiSchemaShape::Number),
        ControlField::optional("cancel", UiSchemaShape::Bool),
    ]))
    .with_theme_group(
        ControlThemeGroup::surface("surface2d")
            .with_style(ControlStyleRole::Container, "surface")
            .with_style(ControlStyleRole::FocusRing, "surface-focus-ring")
            .with_optional_visual_state(ControlVisualState::Focused),
    )
    .build_contribution()
}

pub fn control_module() -> ControlModuleDescriptor {
    ControlCompiler::new().compile_module(&control_contribution())
}
