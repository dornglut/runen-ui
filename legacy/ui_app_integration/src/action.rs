//! App-action descriptors for the proof bridge.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use ui_program::{RouteCapability, RouteId, RouteSchemaVersion};
use ui_schema::UiSchemaRef;

use crate::ids::UiAppActionId;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiAppActionCapabilityRequirement {
    pub capability: RouteCapability,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiAppActionDescriptor {
    pub action_id: UiAppActionId,
    pub route: RouteId,
    pub schema_version: RouteSchemaVersion,
    pub payload_schema: UiSchemaRef,
    pub capability: UiAppActionCapabilityRequirement,
}

impl UiAppActionDescriptor {
    pub fn new(
        action_id: UiAppActionId,
        route: RouteId,
        schema_version: RouteSchemaVersion,
        payload_schema: UiSchemaRef,
        capability: RouteCapability,
    ) -> Self {
        Self {
            action_id,
            route,
            schema_version,
            payload_schema,
            capability: UiAppActionCapabilityRequirement { capability },
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiAppActionRegistry {
    actions: BTreeMap<String, UiAppActionDescriptor>,
    routes: BTreeMap<String, UiAppActionId>,
}

impl UiAppActionRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(
        &mut self,
        descriptor: UiAppActionDescriptor,
    ) -> Result<(), UiAppActionRegistryError> {
        let action_key = descriptor.action_id.as_str().to_owned();
        let route_key = descriptor.route.as_str().to_owned();
        if self.actions.contains_key(&action_key) {
            return Err(UiAppActionRegistryError::DuplicateAction {
                action_id: action_key,
            });
        }
        if self.routes.contains_key(&route_key) {
            return Err(UiAppActionRegistryError::DuplicateRoute { route: route_key });
        }
        self.routes.insert(route_key, descriptor.action_id.clone());
        self.actions.insert(action_key, descriptor);
        Ok(())
    }

    pub fn descriptor(&self, action_id: &UiAppActionId) -> Option<&UiAppActionDescriptor> {
        self.actions.get(action_id.as_str())
    }

    pub fn descriptor_for_route(&self, route: &RouteId) -> Option<&UiAppActionDescriptor> {
        self.routes
            .get(route.as_str())
            .and_then(|action_id| self.descriptor(action_id))
    }

    pub fn len(&self) -> usize {
        self.actions.len()
    }

    pub fn is_empty(&self) -> bool {
        self.actions.is_empty()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum UiAppActionRegistryError {
    DuplicateAction { action_id: String },
    DuplicateRoute { route: String },
}
