//! File: domain/ui/ui_schema/src/value.rs
//! Crate: ui_schema

use std::collections::BTreeMap;
use std::fmt;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum UiSchemaValue {
    Null,
    Bool(bool),
    Integer(i64),
    UnsignedInteger(u64),
    Number(f64),
    String(String),
    StableIdRef(UiStableIdRef),
    RouteRef(UiRouteRef),
    OpaqueHostRef(UiOpaqueHostRef),
    List(Vec<UiSchemaValue>),
    Object(BTreeMap<String, UiSchemaValue>),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum UiSchemaValueKind {
    Null,
    Bool,
    Integer,
    UnsignedInteger,
    Number,
    String,
    StableIdRef,
    RouteRef,
    OpaqueHostRef,
    List,
    Object,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UiStableIdRef(String);

impl UiStableIdRef {
    pub fn new(value: impl Into<String>) -> Self {
        Self::try_new(value).expect("UI stable id references must be namespaced")
    }

    pub fn try_new(value: impl Into<String>) -> Result<Self, UiSchemaValueError> {
        let value = value.into();
        validate_stable_ref(&value, "stable_id")?;
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UiRouteRef(String);

impl UiRouteRef {
    pub fn new(value: impl Into<String>) -> Self {
        Self::try_new(value).expect("UI route references must be namespaced")
    }

    pub fn try_new(value: impl Into<String>) -> Result<Self, UiSchemaValueError> {
        let value = value.into();
        validate_stable_ref(&value, "route")?;
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UiOpaqueHostRef(String);

impl UiOpaqueHostRef {
    pub fn new(value: impl Into<String>) -> Self {
        Self::try_new(value).expect("opaque host references must be namespaced")
    }

    pub fn try_new(value: impl Into<String>) -> Result<Self, UiSchemaValueError> {
        let value = value.into();
        validate_stable_ref(&value, "opaque_host")?;
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum UiSchemaValueError {
    NonFiniteNumber { value: f64 },
    DuplicateObjectField { field: String },
    EmptyStableReference { kind: &'static str },
    UnnamespacedStableReference { kind: &'static str, value: String },
    InvalidStableReferenceCharacter { kind: &'static str, value: String },
}

impl fmt::Display for UiSchemaValueError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFiniteNumber { value } => {
                write!(formatter, "non-finite UI schema number {value}")
            }
            Self::DuplicateObjectField { field } => {
                write!(formatter, "duplicate UI schema object field {field}")
            }
            Self::EmptyStableReference { kind } => {
                write!(formatter, "empty UI schema {kind} reference")
            }
            Self::UnnamespacedStableReference { kind, value } => {
                write!(
                    formatter,
                    "UI schema {kind} reference {value} is not namespaced"
                )
            }
            Self::InvalidStableReferenceCharacter { kind, value } => {
                write!(
                    formatter,
                    "UI schema {kind} reference {value} contains an invalid character"
                )
            }
        }
    }
}

impl std::error::Error for UiSchemaValueError {}

impl UiSchemaValue {
    pub const fn null() -> Self {
        Self::Null
    }

    pub const fn bool(value: bool) -> Self {
        Self::Bool(value)
    }

    pub const fn integer(value: i64) -> Self {
        Self::Integer(value)
    }

    pub const fn unsigned_integer(value: u64) -> Self {
        Self::UnsignedInteger(value)
    }

    pub fn number(value: f64) -> Self {
        Self::try_number(value).expect("UI schema numbers must be finite")
    }

    pub fn try_number(value: f64) -> Result<Self, UiSchemaValueError> {
        if value.is_finite() {
            Ok(Self::Number(value))
        } else {
            Err(UiSchemaValueError::NonFiniteNumber { value })
        }
    }

    pub fn string(value: impl Into<String>) -> Self {
        Self::String(value.into())
    }

    pub fn stable_id_ref(value: impl Into<String>) -> Self {
        Self::StableIdRef(UiStableIdRef::new(value))
    }

    pub fn route_ref(value: impl Into<String>) -> Self {
        Self::RouteRef(UiRouteRef::new(value))
    }

    pub fn opaque_host_ref(value: impl Into<String>) -> Self {
        Self::OpaqueHostRef(UiOpaqueHostRef::new(value))
    }

    pub fn list(values: impl IntoIterator<Item = UiSchemaValue>) -> Self {
        Self::List(values.into_iter().collect())
    }

    pub fn object<const N: usize>(entries: [(&str, UiSchemaValue); N]) -> Self {
        Self::try_object(entries).expect("UI schema object fields must be unique")
    }

    pub fn try_object<const N: usize>(
        entries: [(&str, UiSchemaValue); N],
    ) -> Result<Self, UiSchemaValueError> {
        let mut values = BTreeMap::new();
        for (key, value) in entries {
            if values.insert(key.to_owned(), value).is_some() {
                return Err(UiSchemaValueError::DuplicateObjectField {
                    field: key.to_owned(),
                });
            }
        }
        Ok(Self::Object(values))
    }

    pub fn kind(&self) -> UiSchemaValueKind {
        match self {
            Self::Null => UiSchemaValueKind::Null,
            Self::Bool(_) => UiSchemaValueKind::Bool,
            Self::Integer(_) => UiSchemaValueKind::Integer,
            Self::UnsignedInteger(_) => UiSchemaValueKind::UnsignedInteger,
            Self::Number(_) => UiSchemaValueKind::Number,
            Self::String(_) => UiSchemaValueKind::String,
            Self::StableIdRef(_) => UiSchemaValueKind::StableIdRef,
            Self::RouteRef(_) => UiSchemaValueKind::RouteRef,
            Self::OpaqueHostRef(_) => UiSchemaValueKind::OpaqueHostRef,
            Self::List(_) => UiSchemaValueKind::List,
            Self::Object(_) => UiSchemaValueKind::Object,
        }
    }

    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Self::Bool(value) => Some(*value),
            _ => None,
        }
    }

    pub fn as_integer(&self) -> Option<i64> {
        match self {
            Self::Integer(value) => Some(*value),
            _ => None,
        }
    }

    pub fn as_unsigned_integer(&self) -> Option<u64> {
        match self {
            Self::UnsignedInteger(value) => Some(*value),
            _ => None,
        }
    }

    pub fn as_number(&self) -> Option<f64> {
        match self {
            Self::Number(value) => Some(*value),
            _ => None,
        }
    }

    pub fn as_str(&self) -> Option<&str> {
        match self {
            Self::String(value) => Some(value),
            _ => None,
        }
    }

    pub fn as_stable_id_ref(&self) -> Option<&UiStableIdRef> {
        match self {
            Self::StableIdRef(value) => Some(value),
            _ => None,
        }
    }

    pub fn as_route_ref(&self) -> Option<&UiRouteRef> {
        match self {
            Self::RouteRef(value) => Some(value),
            _ => None,
        }
    }

    pub fn as_opaque_host_ref(&self) -> Option<&UiOpaqueHostRef> {
        match self {
            Self::OpaqueHostRef(value) => Some(value),
            _ => None,
        }
    }

    pub fn as_list(&self) -> Option<&[UiSchemaValue]> {
        match self {
            Self::List(values) => Some(values),
            _ => None,
        }
    }

    pub fn as_object(&self) -> Option<&BTreeMap<String, UiSchemaValue>> {
        match self {
            Self::Object(values) => Some(values),
            _ => None,
        }
    }

    pub fn get(&self, key: &str) -> Option<&UiSchemaValue> {
        match self {
            Self::Object(values) => values.get(key),
            _ => None,
        }
    }

    pub fn get_index(&self, index: usize) -> Option<&UiSchemaValue> {
        match self {
            Self::List(values) => values.get(index),
            _ => None,
        }
    }

    pub fn get_path(&self, path: &[&str]) -> Option<&UiSchemaValue> {
        let mut current = self;
        for segment in path {
            current = current.get(segment)?;
        }
        Some(current)
    }
}

fn validate_stable_ref(value: &str, kind: &'static str) -> Result<(), UiSchemaValueError> {
    if value.is_empty() {
        return Err(UiSchemaValueError::EmptyStableReference { kind });
    }
    if !value.contains('.') {
        return Err(UiSchemaValueError::UnnamespacedStableReference {
            kind,
            value: value.to_owned(),
        });
    }
    if !value
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || matches!(character, '.' | '_' | '-'))
    {
        return Err(UiSchemaValueError::InvalidStableReferenceCharacter {
            kind,
            value: value.to_owned(),
        });
    }
    Ok(())
}
