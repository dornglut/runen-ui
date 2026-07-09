//! Host-safe command intent contracts.

use serde::{Deserialize, Serialize};
use ui_schema::UiSchemaValue;

use crate::events::{RouteCapability, RouteId};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct UiCommandIntent {
    pub route: RouteId,
    pub payload: UiSchemaValue,
    #[serde(default)]
    pub required_capabilities: Vec<RouteCapability>,
}

impl UiCommandIntent {
    pub fn new(route: RouteId, payload: UiSchemaValue) -> Self {
        Self {
            route,
            payload,
            required_capabilities: Vec::new(),
        }
    }

    pub fn with_capability(mut self, capability: RouteCapability) -> Self {
        self.required_capabilities.push(capability);
        self
    }
}
