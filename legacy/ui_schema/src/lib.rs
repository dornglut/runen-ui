//! File: domain/ui/ui_schema/src/lib.rs
//! Crate: ui_schema

pub mod schema;
pub mod value;

pub use schema::*;
pub use value::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_value_preserves_typed_object_shape() {
        let value = UiSchemaValue::object([
            ("route", UiSchemaValue::string("editor.color.apply")),
            ("version", UiSchemaValue::integer(1)),
            (
                "payload",
                UiSchemaValue::object([
                    ("r", UiSchemaValue::number(0.25)),
                    ("g", UiSchemaValue::number(0.5)),
                    ("b", UiSchemaValue::number(0.75)),
                    ("a", UiSchemaValue::number(1.0)),
                ]),
            ),
        ]);

        assert_eq!(value.kind(), UiSchemaValueKind::Object);
        assert_eq!(
            value.get("route"),
            Some(&UiSchemaValue::string("editor.color.apply"))
        );
        assert_eq!(
            value.get_path(&["payload", "g"]),
            Some(&UiSchemaValue::number(0.5))
        );
    }

    #[test]
    fn schema_value_validates_route_refs_and_unknown_fields() {
        let schema = UiSchema::object("ui.event.route_payload", 1)
            .with_required_field("route", UiSchemaShape::RouteRef)
            .with_optional_field("label", UiSchemaShape::String);
        let valid = UiSchemaValue::object([
            ("route", UiSchemaValue::route_ref("editor.color.apply")),
            ("label", UiSchemaValue::string("Apply")),
        ]);

        assert!(schema.validate(&valid).is_valid());

        let invalid = UiSchemaValue::object([
            ("route", UiSchemaValue::string("editor.color.apply")),
            ("unknown", UiSchemaValue::bool(true)),
        ]);
        let report = schema.validate(&invalid);

        assert!(!report.is_valid());
        assert_eq!(
            report.diagnostics[0].diagnostic_id.as_str(),
            "ui.schema.field_kind_mismatch"
        );
        assert_eq!(
            report.diagnostics[1].diagnostic_id.as_str(),
            "ui.schema.unknown_field"
        );
    }

    #[test]
    fn schema_value_rejects_non_finite_numbers_and_duplicate_fields() {
        assert!(UiSchemaValue::try_number(f64::INFINITY).is_err());
        assert!(
            UiSchemaValue::try_object([
                ("field", UiSchemaValue::integer(1)),
                ("field", UiSchemaValue::integer(2)),
            ])
            .is_err()
        );
    }

    #[test]
    fn schema_value_validates_string_enum_fields() {
        let schema = UiSchema::object("ui.button.properties", 1).with_required_field(
            "density",
            UiSchemaShape::string_enum(["compact", "normal", "spacious"]),
        );

        let valid = UiSchemaValue::object([("density", UiSchemaValue::string("compact"))]);

        assert!(schema.validate(&valid).is_valid());

        let invalid = UiSchemaValue::object([("density", UiSchemaValue::string("tiny"))]);

        let report = schema.validate(&invalid);

        assert!(!report.is_valid());
        assert_eq!(
            report.diagnostics[0].diagnostic_id.as_str(),
            "ui.schema.string_value_not_allowed"
        );
        assert_eq!(report.diagnostics[0].field_path, ["density"]);
    }
}
