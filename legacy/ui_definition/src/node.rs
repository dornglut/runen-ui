//! Authored UI node definitions.

use crate::{
    availability::UiAvailabilityBinding,
    embed::UiEmbedSlotRef,
    identity::{UiNodeId, UiTemplateId},
    interaction::UiScrollOwnershipDefinition,
    slot::{UiCollectionSlotRef, UiMenuSlotRef, UiRouteSlotRef, UiSelectionSlotRef},
    value::UiValueBinding,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UiAxisDefinition {
    Horizontal,
    Vertical,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum UiScrollAxisDefinition {
    Horizontal,
    #[default]
    Vertical,
    Both,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UiScrollInputPolicyDefinition {
    WheelOnly,
    MiddleDragOnly,
    WheelAndMiddleDrag,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct UiScrollInputDefinition {
    pub horizontal: UiScrollInputPolicyDefinition,
    pub vertical: UiScrollInputPolicyDefinition,
}

impl Default for UiScrollInputDefinition {
    fn default() -> Self {
        Self {
            horizontal: UiScrollInputPolicyDefinition::WheelAndMiddleDrag,
            vertical: UiScrollInputPolicyDefinition::WheelOnly,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct UiTableColumnDefinition {
    pub label: String,
    pub width: f32,
    pub value: UiValueBinding,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AuthoredControlKindId(String);

impl AuthoredControlKindId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AuthoredRouteId(String);

impl AuthoredRouteId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct AuthoredBindingRef(String);

impl AuthoredBindingRef {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum AuthoredControlValue {
    Null,
    Bool(bool),
    Integer(i64),
    Number(f64),
    String(String),
    List(Vec<AuthoredControlValue>),
    Object(BTreeMap<String, AuthoredControlValue>),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AuthoredControlAccessibilityDefinition {
    pub role: String,
    #[serde(default)]
    pub label: Option<String>,
}
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum UiNodeDefinition {
    Panel {
        id: UiNodeId,
        children: Vec<UiNodeDefinition>,
        availability: Option<UiAvailabilityBinding>,
    },
    Row {
        id: UiNodeId,
        children: Vec<UiNodeDefinition>,
    },
    Column {
        id: UiNodeId,
        children: Vec<UiNodeDefinition>,
    },
    Stack {
        id: UiNodeId,
        axis: UiAxisDefinition,
        children: Vec<UiNodeDefinition>,
    },
    Scroll {
        id: UiNodeId,
        axis: UiScrollAxisDefinition,
        #[serde(default)]
        input: UiScrollInputDefinition,
        #[serde(default)]
        ownership: UiScrollOwnershipDefinition,
        children: Vec<UiNodeDefinition>,
    },
    Split {
        id: UiNodeId,
        axis: UiAxisDefinition,
        ratio: f32,
        children: Vec<UiNodeDefinition>,
    },
    Spacer {
        id: UiNodeId,
    },
    Separator {
        id: UiNodeId,
        #[serde(default)]
        axis: Option<UiAxisDefinition>,
        #[serde(default)]
        length: Option<f32>,
        #[serde(default)]
        thickness: Option<f32>,
    },
    Label {
        id: UiNodeId,
        label: UiValueBinding,
        #[serde(default)]
        availability: Option<UiAvailabilityBinding>,
    },
    Control {
        id: UiNodeId,
        kind: AuthoredControlKindId,
        #[serde(default)]
        properties: BTreeMap<String, AuthoredControlValue>,
        #[serde(default)]
        bindings: BTreeMap<String, AuthoredBindingRef>,
        #[serde(default)]
        route: Option<AuthoredRouteId>,
        #[serde(default)]
        accessibility: Option<AuthoredControlAccessibilityDefinition>,
        #[serde(default)]
        children: Vec<UiNodeDefinition>,
    },
    Button {
        id: UiNodeId,
        label: UiValueBinding,
        route: Option<UiRouteSlotRef>,
        availability: Option<UiAvailabilityBinding>,
        selected: Option<UiValueBinding>,
    },
    Toggle {
        id: UiNodeId,
        label: UiValueBinding,
        checked: UiValueBinding,
        route: Option<UiRouteSlotRef>,
        availability: Option<UiAvailabilityBinding>,
    },
    TextInput {
        id: UiNodeId,
        value: UiValueBinding,
        placeholder: Option<String>,
        route: Option<UiRouteSlotRef>,
        availability: Option<UiAvailabilityBinding>,
    },
    NumericInput {
        id: UiNodeId,
        value: UiValueBinding,
        route: Option<UiRouteSlotRef>,
        availability: Option<UiAvailabilityBinding>,
    },
    Select {
        id: UiNodeId,
        items: UiCollectionSlotRef,
        selected: Option<UiSelectionSlotRef>,
        route: Option<UiRouteSlotRef>,
        availability: Option<UiAvailabilityBinding>,
    },
    Tabs {
        id: UiNodeId,
        items: UiCollectionSlotRef,
        selected: Option<UiSelectionSlotRef>,
        route: Option<UiRouteSlotRef>,
    },
    Table {
        id: UiNodeId,
        rows: UiCollectionSlotRef,
        #[serde(default)]
        columns: Vec<UiTableColumnDefinition>,
        route: Option<UiRouteSlotRef>,
    },
    Tree {
        id: UiNodeId,
        rows: UiCollectionSlotRef,
        route: Option<UiRouteSlotRef>,
    },
    Repeat {
        id: UiNodeId,
        items: UiCollectionSlotRef,
        template: UiTemplateId,
        #[serde(default)]
        axis: Option<UiAxisDefinition>,
    },
    TemplateRef {
        id: UiNodeId,
        template: UiTemplateId,
    },
    MenuSlot {
        id: UiNodeId,
        menu: UiMenuSlotRef,
    },
    EmbedSlot {
        id: UiNodeId,
        slot: UiEmbedSlotRef,
    },
}

impl UiNodeDefinition {
    pub fn id(&self) -> &UiNodeId {
        match self {
            Self::Panel { id, .. }
            | Self::Row { id, .. }
            | Self::Column { id, .. }
            | Self::Stack { id, .. }
            | Self::Scroll { id, .. }
            | Self::Split { id, .. }
            | Self::Spacer { id }
            | Self::Separator { id, .. }
            | Self::Label { id, .. }
            | Self::Control { id, .. }
            | Self::Button { id, .. }
            | Self::Toggle { id, .. }
            | Self::TextInput { id, .. }
            | Self::NumericInput { id, .. }
            | Self::Select { id, .. }
            | Self::Tabs { id, .. }
            | Self::Table { id, .. }
            | Self::Tree { id, .. }
            | Self::Repeat { id, .. }
            | Self::TemplateRef { id, .. }
            | Self::MenuSlot { id, .. }
            | Self::EmbedSlot { id, .. } => id,
        }
    }

    pub fn children(&self) -> &[UiNodeDefinition] {
        match self {
            Self::Panel { children, .. }
            | Self::Row { children, .. }
            | Self::Column { children, .. }
            | Self::Stack { children, .. }
            | Self::Scroll { children, .. }
            | Self::Split { children, .. }
            | Self::Control { children, .. } => children,
            _ => &[],
        }
    }

    pub fn children_mut(&mut self) -> Option<&mut Vec<UiNodeDefinition>> {
        match self {
            Self::Panel { children, .. }
            | Self::Row { children, .. }
            | Self::Column { children, .. }
            | Self::Stack { children, .. }
            | Self::Scroll { children, .. }
            | Self::Split { children, .. }
            | Self::Control { children, .. } => Some(children),
            _ => None,
        }
    }
}
