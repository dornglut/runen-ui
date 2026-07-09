//! File: domain/ui/ui_controls/src/schema.rs
//! Crate: ui_controls

use serde::{Deserialize, Serialize};
use ui_schema::{UiSchema, UiSchemaRef};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ControlSchemaRole {
    Properties,
    State,
    EventPayload,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ControlSchemaDescriptor {
    pub role: ControlSchemaRole,
    pub schema: UiSchema,
}

impl ControlSchemaDescriptor {
    pub fn new(role: ControlSchemaRole, schema: UiSchema) -> Self {
        Self { role, schema }
    }

    pub fn properties(schema: UiSchema) -> Self {
        Self::new(ControlSchemaRole::Properties, schema)
    }

    pub fn state(schema: UiSchema) -> Self {
        Self::new(ControlSchemaRole::State, schema)
    }

    pub fn event_payload(schema: UiSchema) -> Self {
        Self::new(ControlSchemaRole::EventPayload, schema)
    }

    pub fn schema_ref(&self) -> &UiSchemaRef {
        &self.schema.schema_ref
    }
}
