//! Generic route, collection, and selection slot references.

use crate::identity::{
    UiCollectionSlotId, UiMenuSlotId, UiRouteSlotId, UiSelectionSlotId, UiValueSlotId,
};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiRouteSlotRef {
    pub id: UiRouteSlotId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiValueSlotRef {
    pub id: UiValueSlotId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiCollectionSlotRef {
    pub id: UiCollectionSlotId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiSelectionSlotRef {
    pub id: UiSelectionSlotId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiMenuSlotRef {
    pub id: UiMenuSlotId,
}
