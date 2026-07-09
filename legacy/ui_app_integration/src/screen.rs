//! Screen descriptors for the proof bridge.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use ui_definition::{UiNodeDefinition, UiRouteSlotRef};
use ui_program::RouteId;

use crate::ids::UiAppScreenId;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiAppScreenRoute {
    pub slot: UiRouteSlotRef,
    pub route: RouteId,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct UiAppScreenDescriptor {
    pub screen_id: UiAppScreenId,
    pub root: UiNodeDefinition,
    #[serde(default)]
    pub routes: Vec<UiAppScreenRoute>,
}

impl UiAppScreenDescriptor {
    pub fn new(screen_id: UiAppScreenId, root: UiNodeDefinition) -> Self {
        Self {
            screen_id,
            root,
            routes: Vec::new(),
        }
    }

    pub fn with_route(mut self, route: UiAppScreenRoute) -> Self {
        self.routes.push(route);
        self
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiAppScreenSelection {
    pub active_screen: UiAppScreenId,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct UiAppScreenRegistry {
    screens: BTreeMap<String, UiAppScreenDescriptor>,
}

impl UiAppScreenRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register(
        &mut self,
        descriptor: UiAppScreenDescriptor,
    ) -> Result<(), UiAppScreenRegistryError> {
        let key = descriptor.screen_id.as_str().to_owned();
        if self.screens.contains_key(&key) {
            return Err(UiAppScreenRegistryError::DuplicateScreen { screen_id: key });
        }
        self.screens.insert(key, descriptor);
        Ok(())
    }

    pub fn screen(&self, screen_id: &UiAppScreenId) -> Option<&UiAppScreenDescriptor> {
        self.screens.get(screen_id.as_str())
    }

    pub fn len(&self) -> usize {
        self.screens.len()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum UiAppScreenRegistryError {
    DuplicateScreen { screen_id: String },
}
