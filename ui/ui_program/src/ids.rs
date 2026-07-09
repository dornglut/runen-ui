//! Stable UiProgram identity contracts.

use std::fmt;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UiProgramId(String);

impl UiProgramId {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn try_new(value: impl Into<String>) -> Result<Self, UiProgramIdentityError> {
        let value = value.into();
        validate_namespaced_id(&value, "program")?;
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UiProgramSourceId(String);

impl UiProgramSourceId {
    pub fn new(value: impl Into<String>) -> Self {
        Self::try_new(value).expect("UI program source IDs must be namespaced")
    }

    pub fn try_new(value: impl Into<String>) -> Result<Self, UiProgramIdentityError> {
        let value = value.into();
        validate_namespaced_id(&value, "source")?;
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct UiProgramTargetId(String);

impl UiProgramTargetId {
    pub fn new(value: impl Into<String>) -> Self {
        Self::try_new(value).expect("UI program target IDs must be namespaced")
    }

    pub fn try_new(value: impl Into<String>) -> Result<Self, UiProgramIdentityError> {
        let value = value.into();
        validate_namespaced_id(&value, "target")?;
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UiProgramIdentityError {
    EmptyId { kind: &'static str },
    UnnamespacedId { kind: &'static str, value: String },
    InvalidIdCharacter { kind: &'static str, value: String },
}

impl fmt::Display for UiProgramIdentityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyId { kind } => write!(formatter, "empty UI program {kind} id"),
            Self::UnnamespacedId { kind, value } => {
                write!(formatter, "UI program {kind} id {value} is not namespaced")
            }
            Self::InvalidIdCharacter { kind, value } => write!(
                formatter,
                "UI program {kind} id {value} contains an invalid character"
            ),
        }
    }
}

impl std::error::Error for UiProgramIdentityError {}

fn validate_namespaced_id(value: &str, kind: &'static str) -> Result<(), UiProgramIdentityError> {
    if value.is_empty() {
        return Err(UiProgramIdentityError::EmptyId { kind });
    }
    if !value.contains('.') {
        return Err(UiProgramIdentityError::UnnamespacedId {
            kind,
            value: value.to_owned(),
        });
    }
    if !value
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || matches!(character, '.' | '_' | '-'))
    {
        return Err(UiProgramIdentityError::InvalidIdCharacter {
            kind,
            value: value.to_owned(),
        });
    }
    Ok(())
}
