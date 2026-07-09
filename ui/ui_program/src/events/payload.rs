//! File: domain/ui/ui_program/src/events/payload.rs
//! Crate: ui_program

use serde::{Deserialize, Serialize};
use ui_schema::{
    UiSchema, UiSchemaDiagnosticId, UiSchemaRef, UiSchemaValidationDiagnostic,
    UiSchemaValidationReport, UiSchemaValue,
};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct UiEventPayload {
    pub schema: UiSchemaRef,
    pub value: UiSchemaValue,
    #[serde(default)]
    pub diagnostics: Vec<UiSchemaValidationDiagnostic>,
}

impl UiEventPayload {
    pub fn new(schema: UiSchemaRef, value: UiSchemaValue) -> Self {
        Self {
            schema,
            value,
            diagnostics: Vec::new(),
        }
    }

    pub fn validation_report(&self, schema: &UiSchema) -> UiSchemaValidationReport {
        let mut report = schema.validate(&self.value);
        if schema.schema_ref != self.schema {
            report.diagnostics.push(UiSchemaValidationDiagnostic::new(
                self.schema.clone(),
                UiSchemaDiagnosticId::new("ui.event.payload_schema_mismatch"),
                "event payload schema reference does not match validation schema",
            ));
        }
        report
    }

    pub fn with_validation(mut self, schema: &UiSchema) -> Self {
        self.diagnostics = self.validation_report(schema).diagnostics;
        self
    }

    pub fn is_valid_for(&self, schema: &UiSchema) -> bool {
        self.validation_report(schema).is_valid()
    }
}
