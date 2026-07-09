//! File: domain/ui/ui_controls/src/button/mod.rs
//! Crate: ui_controls

use ui_program::RouteCapability;
use ui_schema::UiSchemaShape;

use crate::{
    ControlCompiler, ControlContribution, ControlDef, ControlField, ControlFieldGroup,
    ControlModuleDescriptor, ControlPreset, ControlStyleRole, ControlThemeGroup,
    ControlThemeTokenKind, ControlThemeTokenRole, ControlVisualState,
};

pub const BUTTON_CONTROL_KIND_ID: &str = "runenwerk.ui.controls.button";

pub fn control_contribution() -> ControlContribution {
    ControlDef::builder(
        "button",
        "Button",
        ControlPreset::Button,
        RouteCapability::new("runenwerk.ui.controls.activate"),
    )
    .with_field_group(ControlFieldGroup::properties([
        ControlField::required("label", UiSchemaShape::String),
        ControlField::optional(
            "variant",
            UiSchemaShape::string_enum([
                "toolbar",
                "primary",
                "secondary",
                "ghost",
                "danger",
                "inspector",
                "icon",
            ]),
        ),
        ControlField::optional(
            "tone",
            UiSchemaShape::string_enum(["neutral", "accent", "success", "warning", "danger"]),
        ),
        ControlField::optional(
            "density",
            UiSchemaShape::string_enum(["compact", "normal", "spacious"]),
        ),
        ControlField::optional("size", UiSchemaShape::string_enum(["xs", "sm", "md", "lg"])),
        ControlField::optional("leading_icon", UiSchemaShape::String),
        ControlField::optional("trailing_icon", UiSchemaShape::String),
        ControlField::optional("show_label", UiSchemaShape::Bool),
        ControlField::optional("disabled", UiSchemaShape::Bool),
    ]))
    .with_field_group(ControlFieldGroup::state([
        ControlField::optional("selected", UiSchemaShape::Bool),
        ControlField::optional("pressed", UiSchemaShape::Bool),
        ControlField::optional("focused", UiSchemaShape::Bool),
    ]))
    .with_field_group(ControlFieldGroup::event_payload([
        ControlField::required("route", UiSchemaShape::RouteRef),
        ControlField::required("activated", UiSchemaShape::Bool),
    ]))
    .with_theme_group(
        ControlThemeGroup::surface("button")
            .with_token(
                "focus-ring",
                ControlThemeTokenKind::Color,
                ControlThemeTokenRole::FocusRing,
            )
            .with_style(ControlStyleRole::FocusRing, "focus-ring")
            .with_optional_visual_state(ControlVisualState::Hover)
            .with_optional_visual_state(ControlVisualState::Pressed)
            .with_optional_visual_state(ControlVisualState::Focused),
    )
    .build_contribution()
}

pub fn control_module() -> ControlModuleDescriptor {
    ControlCompiler::new().compile_module(&control_contribution())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ui_schema::UiSchemaValue;

    #[test]
    fn button_property_schema_accepts_final_shape_style_axes() {
        let schema = button_property_schema();

        let value = UiSchemaValue::object([
            ("label", UiSchemaValue::string("Press me")),
            ("variant", UiSchemaValue::string("primary")),
            ("tone", UiSchemaValue::string("accent")),
            ("density", UiSchemaValue::string("normal")),
            ("size", UiSchemaValue::string("md")),
            (
                "leading_icon",
                UiSchemaValue::string("runenwerk.icons.play"),
            ),
            (
                "trailing_icon",
                UiSchemaValue::string("runenwerk.icons.chevron_right"),
            ),
            ("show_label", UiSchemaValue::bool(true)),
        ]);

        let report = schema.validate(&value);

        assert!(report.is_valid(), "{:?}", report.diagnostics);
    }

    #[test]
    fn button_property_schema_rejects_invalid_density_and_size() {
        let schema = button_property_schema();

        let value = UiSchemaValue::object([
            ("label", UiSchemaValue::string("Broken")),
            ("density", UiSchemaValue::string("tiny")),
            ("size", UiSchemaValue::string("xxl")),
        ]);

        let report = schema.validate(&value);

        assert!(!report.is_valid());
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.field_path == ["density"]
                && diagnostic.diagnostic_id.as_str() == "ui.schema.string_value_not_allowed"
        }));
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.field_path == ["size"]
                && diagnostic.diagnostic_id.as_str() == "ui.schema.string_value_not_allowed"
        }));
    }

    #[test]
    fn button_property_schema_rejects_unknown_properties() {
        let schema = button_property_schema();

        let value = UiSchemaValue::object([
            ("label", UiSchemaValue::string("Broken")),
            ("background_color", UiSchemaValue::string("#ff00ff")),
        ]);

        let report = schema.validate(&value);

        assert!(!report.is_valid());
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.field_path == ["background_color"]
                && diagnostic.diagnostic_id.as_str() == "ui.schema.unknown_field"
        }));
    }

    #[test]
    fn button_property_schema_requires_label() {
        let schema = button_property_schema();

        let value = UiSchemaValue::object([
            ("variant", UiSchemaValue::string("primary")),
            ("tone", UiSchemaValue::string("accent")),
            ("density", UiSchemaValue::string("normal")),
            ("size", UiSchemaValue::string("md")),
        ]);

        let report = schema.validate(&value);

        assert!(!report.is_valid());
        assert!(report.diagnostics.iter().any(|diagnostic| {
            diagnostic.field_path == ["label"]
                && diagnostic.diagnostic_id.as_str() == "ui.schema.required_field_missing"
        }));
    }

    fn button_property_schema() -> ui_schema::UiSchema {
        control_module()
            .schemas
            .into_iter()
            .find(|schema| {
                schema.schema.schema_ref.id.as_str() == "runenwerk.ui.controls.button.properties"
            })
            .expect("button property schema should exist")
            .schema
    }
}
