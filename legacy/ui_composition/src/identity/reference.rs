use core::fmt;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NamespacedReferenceError {
    Empty,
    MissingNamespace,
    EmptySegment,
    InvalidCharacter(char),
    InvalidSegmentBoundary,
}

impl fmt::Display for NamespacedReferenceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => formatter.write_str("reference must not be empty"),
            Self::MissingNamespace => formatter.write_str("reference must contain a namespace"),
            Self::EmptySegment => {
                formatter.write_str("reference contains an empty namespace segment")
            }
            Self::InvalidCharacter(value) => {
                write!(formatter, "reference contains invalid character {value:?}")
            }
            Self::InvalidSegmentBoundary => formatter.write_str(
                "reference segments must begin and end with an ASCII alphanumeric character",
            ),
        }
    }
}

impl std::error::Error for NamespacedReferenceError {}

fn validate(value: &str) -> Result<(), NamespacedReferenceError> {
    if value.is_empty() {
        return Err(NamespacedReferenceError::Empty);
    }
    if !value.contains('.') {
        return Err(NamespacedReferenceError::MissingNamespace);
    }
    if value.split('.').any(str::is_empty) {
        return Err(NamespacedReferenceError::EmptySegment);
    }
    if value.split('.').any(|segment| {
        !segment
            .as_bytes()
            .first()
            .is_some_and(u8::is_ascii_alphanumeric)
            || !segment
                .as_bytes()
                .last()
                .is_some_and(u8::is_ascii_alphanumeric)
    }) {
        return Err(NamespacedReferenceError::InvalidSegmentBoundary);
    }
    if let Some(character) = value.chars().find(|character| {
        !character.is_ascii_alphanumeric() && !matches!(character, '.' | '_' | '-')
    }) {
        return Err(NamespacedReferenceError::InvalidCharacter(character));
    }
    Ok(())
}

macro_rules! namespaced_reference {
    ($name:ident) => {
        #[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        #[serde(try_from = "String", into = "String")]
        pub struct $name(String);

        impl $name {
            pub fn new(value: impl Into<String>) -> Result<Self, NamespacedReferenceError> {
                let value = value.into();
                validate(&value)?;
                Ok(Self(value))
            }

            pub fn try_new(value: impl Into<String>) -> Result<Self, NamespacedReferenceError> {
                Self::new(value)
            }

            pub fn as_str(&self) -> &str {
                self.0.as_str()
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter.write_str(self.as_str())
            }
        }

        impl TryFrom<String> for $name {
            type Error = NamespacedReferenceError;

            fn try_from(value: String) -> Result<Self, Self::Error> {
                Self::new(value)
            }
        }

        impl From<$name> for String {
            fn from(value: $name) -> Self {
                value.0
            }
        }
    };
}

namespaced_reference!(HostProfileId);
namespaced_reference!(TargetProfileId);
namespaced_reference!(RegionProfileId);
namespaced_reference!(ContentOwnerId);
namespaced_reference!(ContentProfileId);
namespaced_reference!(ContentInstanceRef);
namespaced_reference!(CapabilityId);
namespaced_reference!(AdaptiveProposalExpectationId);
namespaced_reference!(AppProfileId);
namespaced_reference!(ExtensionProfileId);
