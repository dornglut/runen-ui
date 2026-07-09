//! File: domain/ui/ui_program/src/events/route.rs
//! Crate: ui_program

use std::fmt;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RouteId(String);

impl RouteId {
    pub fn new(value: impl Into<String>) -> Self {
        Self::try_new(value).expect("route IDs must be stable namespaced IDs")
    }

    pub fn try_new(value: impl Into<String>) -> Result<Self, RouteContractError> {
        let value = value.into();
        validate_route_ref(&value, "route")?;
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct RouteSchemaVersion(u32);

impl RouteSchemaVersion {
    pub const fn new(value: u32) -> Self {
        assert!(value > 0);
        Self(value)
    }

    pub fn try_new(value: u32) -> Result<Self, RouteContractError> {
        if value == 0 {
            Err(RouteContractError::ZeroSchemaVersion)
        } else {
            Ok(Self(value))
        }
    }

    pub const fn value(self) -> u32 {
        self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RouteCapability(String);

impl RouteCapability {
    pub fn new(value: impl Into<String>) -> Self {
        Self::try_new(value).expect("route capabilities must be stable namespaced IDs")
    }

    pub fn try_new(value: impl Into<String>) -> Result<Self, RouteContractError> {
        let value = value.into();
        validate_route_ref(&value, "capability")?;
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RouteContractError {
    EmptyId { kind: &'static str },
    UnnamespacedId { kind: &'static str, value: String },
    InvalidIdCharacter { kind: &'static str, value: String },
    ZeroSchemaVersion,
}

impl fmt::Display for RouteContractError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyId { kind } => write!(formatter, "empty route {kind} id"),
            Self::UnnamespacedId { kind, value } => {
                write!(formatter, "route {kind} id {value} is not namespaced")
            }
            Self::InvalidIdCharacter { kind, value } => {
                write!(
                    formatter,
                    "route {kind} id {value} contains an invalid character"
                )
            }
            Self::ZeroSchemaVersion => write!(formatter, "route schema version must be non-zero"),
        }
    }
}

impl std::error::Error for RouteContractError {}

fn validate_route_ref(value: &str, kind: &'static str) -> Result<(), RouteContractError> {
    if value.is_empty() {
        return Err(RouteContractError::EmptyId { kind });
    }
    if !value.contains('.') {
        return Err(RouteContractError::UnnamespacedId {
            kind,
            value: value.to_owned(),
        });
    }
    if !value
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || matches!(character, '.' | '_' | '-'))
    {
        return Err(RouteContractError::InvalidIdCharacter {
            kind,
            value: value.to_owned(),
        });
    }
    Ok(())
}
