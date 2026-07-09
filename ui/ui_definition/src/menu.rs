//! Generic menu definitions.

use crate::{
    availability::UiAvailabilityBinding,
    identity::UiNodeId,
    interaction::{UiMenuSizingDefinition, UiMenuStackScopeDefinition},
    slot::UiRouteSlotRef,
    value::UiValueBinding,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UiMenuItemDefinition {
    pub id: UiNodeId,
    pub label: UiValueBinding,
    pub route: Option<UiRouteSlotRef>,
    pub availability: Option<UiAvailabilityBinding>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UiMenuDefinition {
    pub id: String,
    #[serde(default)]
    pub scope: Option<UiMenuStackScopeDefinition>,
    #[serde(default)]
    pub sizing: Option<UiMenuSizingDefinition>,
    pub items: Vec<UiMenuItemDefinition>,
}
