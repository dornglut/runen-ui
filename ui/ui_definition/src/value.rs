//! Generic value and collection slot values supplied during formation.

use crate::{
    UiAvailability,
    identity::{UiCollectionSlotId, UiSelectionSlotId, UiValueSlotId},
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum UiValue {
    Text(String),
    Bool(bool),
    Number(f64),
    Availability(UiAvailability),
}

impl UiValue {
    pub fn as_text(&self) -> String {
        match self {
            Self::Text(value) => value.clone(),
            Self::Bool(value) => value.to_string(),
            Self::Number(value) => value.to_string(),
            Self::Availability(value) => match value {
                UiAvailability::Available => "available".to_string(),
                UiAvailability::Disabled { reason } => format!("disabled: {reason}"),
                UiAvailability::Unavailable { reason } => format!("unavailable: {reason}"),
            },
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(value) => Some(*value),
            _ => None,
        }
    }

    pub fn as_number(&self) -> Option<f64> {
        match self {
            Self::Number(value) => Some(*value),
            _ => None,
        }
    }

    pub fn as_availability(&self) -> Option<UiAvailability> {
        match self {
            Self::Availability(value) => Some(value.clone()),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum UiValueBinding {
    Static(UiValue),
    Slot(UiValueSlotId),
}

impl UiValueBinding {
    pub fn static_text(value: impl Into<String>) -> Self {
        Self::Static(UiValue::Text(value.into()))
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UiCollectionItem {
    pub key: String,
    pub label: String,
    pub selected: bool,
    pub enabled: bool,
    #[serde(default)]
    pub values: BTreeMap<UiValueSlotId, UiValue>,
    #[serde(default)]
    pub collections: BTreeMap<UiCollectionSlotId, Vec<UiCollectionItem>>,
    #[serde(default)]
    pub selections: BTreeMap<UiSelectionSlotId, String>,
}

impl UiCollectionItem {
    pub fn new(key: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            label: label.into(),
            selected: false,
            enabled: true,
            values: BTreeMap::new(),
            collections: BTreeMap::new(),
            selections: BTreeMap::new(),
        }
    }

    pub fn with_value(mut self, slot: UiValueSlotId, value: UiValue) -> Self {
        self.values.insert(slot, value);
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiCollectionBinding {
    pub slot: UiCollectionSlotId,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiSelectionBinding {
    pub slot: UiSelectionSlotId,
}
