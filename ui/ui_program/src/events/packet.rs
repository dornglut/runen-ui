//! File: domain/ui/ui_program/src/events/packet.rs
//! Crate: ui_program

use serde::{Deserialize, Serialize};
use ui_schema::{UiSchema, UiSchemaRef, UiSchemaValue};

use crate::diagnostics::UiProgramDiagnostic;
use crate::events::payload::UiEventPayload;
use crate::events::phase::UiEventPhase;
use crate::events::route::{RouteCapability, RouteId, RouteSchemaVersion};
use crate::source_map::UiProgramSourceMapEntry;

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UiEventSourceControlId(String);

impl UiEventSourceControlId {
    pub fn new(value: impl Into<String>) -> Self {
        let value = value.into();
        assert!(
            !value.is_empty(),
            "event source control IDs must not be empty"
        );
        Self(value)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct UiEventPacket {
    pub route: RouteId,
    pub schema_version: RouteSchemaVersion,
    #[serde(default)]
    pub source_control: Option<UiEventSourceControlId>,
    #[serde(default)]
    pub phase: UiEventPhase,
    pub payload: UiEventPayload,
    #[serde(default)]
    pub capabilities: Vec<RouteCapability>,
    #[serde(default)]
    pub source_map: Vec<UiProgramSourceMapEntry>,
    #[serde(default)]
    pub diagnostics: Vec<UiProgramDiagnostic>,
}

impl UiEventPacket {
    pub fn new(
        route: RouteId,
        schema_version: RouteSchemaVersion,
        payload_schema: UiSchemaRef,
        payload: UiSchemaValue,
    ) -> Self {
        Self {
            route,
            schema_version,
            source_control: None,
            phase: UiEventPhase::Activate,
            payload: UiEventPayload::new(payload_schema, payload),
            capabilities: Vec::new(),
            source_map: Vec::new(),
            diagnostics: Vec::new(),
        }
    }

    pub fn payload_schema(&self) -> &UiSchemaRef {
        &self.payload.schema
    }

    pub fn with_capability(mut self, capability: RouteCapability) -> Self {
        self.capabilities.push(capability);
        self
    }

    pub fn with_source_control(mut self, source_control: UiEventSourceControlId) -> Self {
        self.source_control = Some(source_control);
        self
    }

    pub fn with_phase(mut self, phase: UiEventPhase) -> Self {
        self.phase = phase;
        self
    }

    pub fn with_source_map_entry(mut self, source_map: UiProgramSourceMapEntry) -> Self {
        self.source_map.push(source_map);
        self
    }

    pub fn with_diagnostic(mut self, diagnostic: UiProgramDiagnostic) -> Self {
        self.diagnostics.push(diagnostic);
        self
    }

    pub fn with_payload_validation(mut self, schema: &UiSchema) -> Self {
        self.payload = self.payload.with_validation(schema);
        self
    }

    pub fn requires_capability(&self, capability: &RouteCapability) -> bool {
        self.capabilities
            .iter()
            .any(|existing| existing == capability)
    }
}
