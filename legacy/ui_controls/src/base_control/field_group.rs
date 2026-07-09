//! File: domain/ui/ui_controls/src/base_control/field_group.rs
//! Crate: ui_controls

use ui_schema::UiSchemaShape;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ControlFieldGroupRole {
    Properties,
    State,
    EventPayload,
}

impl ControlFieldGroupRole {
    pub(crate) const fn schema_suffix(self) -> &'static str {
        match self {
            Self::Properties => "properties",
            Self::State => "state",
            Self::EventPayload => "event",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ControlFieldGroup {
    pub role: ControlFieldGroupRole,
    pub fields: Vec<ControlField>,
}

impl ControlFieldGroup {
    pub fn properties(fields: impl IntoIterator<Item = ControlField>) -> Self {
        Self::new(ControlFieldGroupRole::Properties, fields)
    }

    pub fn state(fields: impl IntoIterator<Item = ControlField>) -> Self {
        Self::new(ControlFieldGroupRole::State, fields)
    }

    pub fn event_payload(fields: impl IntoIterator<Item = ControlField>) -> Self {
        Self::new(ControlFieldGroupRole::EventPayload, fields)
    }

    pub fn new(
        role: ControlFieldGroupRole,
        fields: impl IntoIterator<Item = ControlField>,
    ) -> Self {
        Self {
            role,
            fields: fields.into_iter().collect(),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ControlField {
    pub name: String,
    pub shape: UiSchemaShape,
    pub required: bool,
}

impl ControlField {
    pub fn required(name: impl Into<String>, shape: UiSchemaShape) -> Self {
        Self {
            name: name.into(),
            shape,
            required: true,
        }
    }

    pub fn optional(name: impl Into<String>, shape: UiSchemaShape) -> Self {
        Self {
            name: name.into(),
            shape,
            required: false,
        }
    }
}
