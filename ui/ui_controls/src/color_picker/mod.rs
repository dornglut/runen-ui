//! File: domain/ui/ui_controls/src/color_picker/mod.rs
//! Crate: ui_controls

use ui_program::RouteCapability;
use ui_schema::UiSchemaShape;

use crate::{
    ControlCompiler, ControlContribution, ControlDef, ControlField, ControlFieldGroup,
    ControlModuleDescriptor, ControlPreset, ControlStyleRole, ControlThemeGroup,
    ControlThemeTokenKind, ControlThemeTokenRole, ControlVisualState,
};

pub const COLOR_PICKER_CONTROL_KIND_ID: &str = "runenwerk.ui.controls.color-picker";

pub fn control_contribution() -> ControlContribution {
    ControlDef::builder(
        "color-picker",
        "ColorPicker",
        ControlPreset::ColorPicker,
        RouteCapability::new("runenwerk.ui.controls.color.write"),
    )
    .with_field_group(ControlFieldGroup::properties([
        ControlField::required("committed_rgba", UiSchemaShape::Object),
        ControlField::optional("allow_alpha", UiSchemaShape::Bool),
    ]))
    .with_field_group(ControlFieldGroup::state([
        ControlField::required("hue_degrees", UiSchemaShape::Number),
        ControlField::required("triangle_saturation", UiSchemaShape::Number),
        ControlField::required("triangle_value", UiSchemaShape::Number),
        ControlField::required("preview_rgba", UiSchemaShape::Object),
    ]))
    .with_field_group(ControlFieldGroup::event_payload([
        ControlField::required("route", UiSchemaShape::RouteRef),
        ControlField::required("rgba", UiSchemaShape::Object),
        ControlField::required("preview", UiSchemaShape::Bool),
        ControlField::required("committed", UiSchemaShape::Bool),
    ]))
    .with_theme_group(
        ControlThemeGroup::surface("color-picker")
            .with_token(
                "accent",
                ControlThemeTokenKind::Color,
                ControlThemeTokenRole::Accent,
            )
            .with_style(ControlStyleRole::Accent, "accent")
            .with_optional_visual_state(ControlVisualState::Focused)
            .with_optional_visual_state(ControlVisualState::Active),
    )
    .build_contribution()
}

pub fn control_module() -> ControlModuleDescriptor {
    ControlCompiler::new().compile_module(&control_contribution())
}
