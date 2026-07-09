//! File: domain/ui/ui_controls/src/package/ids.rs
//! Crate: ui_controls

use std::fmt;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ControlPackageId(String);
impl ControlPackageId {
    pub fn new(value: impl Into<String>) -> Self {
        Self::try_new(value).expect("control package IDs must be namespaced")
    }
    pub fn try_new(value: impl Into<String>) -> Result<Self, ControlPackageContractError> {
        let value = value.into();
        validate_control_id(&value, "package")?;
        Ok(Self(value))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ControlKindId(String);
impl ControlKindId {
    pub fn new(value: impl Into<String>) -> Self {
        Self::try_new(value).expect("control kind IDs must be namespaced")
    }
    pub fn try_new(value: impl Into<String>) -> Result<Self, ControlPackageContractError> {
        let value = value.into();
        validate_control_id(&value, "kind")?;
        Ok(Self(value))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ControlFixtureId(String);
impl ControlFixtureId {
    pub fn new(value: impl Into<String>) -> Self {
        Self::try_new(value).expect("control fixture IDs must be namespaced")
    }
    pub fn try_new(value: impl Into<String>) -> Result<Self, ControlPackageContractError> {
        let value = value.into();
        validate_control_id(&value, "fixture")?;
        Ok(Self(value))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ControlStoryId(String);
impl ControlStoryId {
    pub fn new(value: impl Into<String>) -> Self {
        Self::try_new(value).expect("control story IDs must be namespaced")
    }
    pub fn try_new(value: impl Into<String>) -> Result<Self, ControlPackageContractError> {
        let value = value.into();
        validate_control_id(&value, "story")?;
        Ok(Self(value))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ControlRenderEvidenceId(String);
impl ControlRenderEvidenceId {
    pub fn new(value: impl Into<String>) -> Self {
        Self::try_new(value).expect("control render evidence IDs must be namespaced")
    }
    pub fn try_new(value: impl Into<String>) -> Result<Self, ControlPackageContractError> {
        let value = value.into();
        validate_control_id(&value, "render evidence")?;
        Ok(Self(value))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ControlBudgetEvidenceId(String);
impl ControlBudgetEvidenceId {
    pub fn new(value: impl Into<String>) -> Self {
        Self::try_new(value).expect("control budget evidence IDs must be namespaced")
    }
    pub fn try_new(value: impl Into<String>) -> Result<Self, ControlPackageContractError> {
        let value = value.into();
        validate_control_id(&value, "budget evidence")?;
        Ok(Self(value))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ControlTargetProfileRef(String);
impl ControlTargetProfileRef {
    pub fn new(value: impl Into<String>) -> Self {
        Self::try_new(value).expect("control target profile refs must be namespaced")
    }
    pub fn try_new(value: impl Into<String>) -> Result<Self, ControlPackageContractError> {
        let value = value.into();
        validate_control_id(&value, "target profile")?;
        Ok(Self(value))
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ControlPackageVersion(u32);
impl ControlPackageVersion {
    pub const fn new(value: u32) -> Self {
        assert!(value > 0);
        Self(value)
    }
    pub const fn value(self) -> u32 {
        self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ControlPackageContractError {
    EmptyId { kind: &'static str },
    UnnamespacedId { kind: &'static str, value: String },
    InvalidIdCharacter { kind: &'static str, value: String },
}

impl fmt::Display for ControlPackageContractError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyId { kind } => write!(formatter, "empty control {kind} id"),
            Self::UnnamespacedId { kind, value } => {
                write!(formatter, "control {kind} id {value} is not namespaced")
            }
            Self::InvalidIdCharacter { kind, value } => write!(
                formatter,
                "control {kind} id {value} contains an invalid character"
            ),
        }
    }
}
impl std::error::Error for ControlPackageContractError {}

fn validate_control_id(value: &str, kind: &'static str) -> Result<(), ControlPackageContractError> {
    if value.is_empty() {
        return Err(ControlPackageContractError::EmptyId { kind });
    }
    if !value.contains('.') {
        return Err(ControlPackageContractError::UnnamespacedId {
            kind,
            value: value.to_owned(),
        });
    }
    if !value
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || matches!(character, '.' | '_' | '-'))
    {
        return Err(ControlPackageContractError::InvalidIdCharacter {
            kind,
            value: value.to_owned(),
        });
    }
    Ok(())
}
