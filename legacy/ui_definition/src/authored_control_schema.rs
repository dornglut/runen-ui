// File: domain/ui/ui_definition/src/authored_control_schema.rs
// Functions: authored_control_value_to_schema_value, authored_control_properties_to_schema_value

use std::collections::BTreeMap;

use ui_schema::UiSchemaValue;

use crate::AuthoredControlValue;

pub fn authored_control_value_to_schema_value(value: &AuthoredControlValue) -> UiSchemaValue {
    match value {
        AuthoredControlValue::Null => UiSchemaValue::null(),
        AuthoredControlValue::Bool(value) => UiSchemaValue::bool(*value),
        AuthoredControlValue::Integer(value) => UiSchemaValue::integer(*value),
        AuthoredControlValue::Number(value) => UiSchemaValue::number(*value),
        AuthoredControlValue::String(value) => UiSchemaValue::string(value),
        AuthoredControlValue::List(values) => {
            UiSchemaValue::list(values.iter().map(authored_control_value_to_schema_value))
        }
        AuthoredControlValue::Object(values) => UiSchemaValue::Object(
            values
                .iter()
                .map(|(key, value)| (key.clone(), authored_control_value_to_schema_value(value)))
                .collect::<BTreeMap<_, _>>(),
        ),
    }
}

pub fn authored_control_properties_to_schema_value(
    properties: &BTreeMap<String, AuthoredControlValue>,
) -> UiSchemaValue {
    UiSchemaValue::Object(
        properties
            .iter()
            .map(|(key, value)| (key.clone(), authored_control_value_to_schema_value(value)))
            .collect::<BTreeMap<_, _>>(),
    )
}
