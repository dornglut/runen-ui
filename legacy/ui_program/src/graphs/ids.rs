//! Graph identity contracts.

use std::fmt;

use serde::{Deserialize, Serialize};

macro_rules! graph_id {
    ($name:ident, $kind:literal) => {
        #[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
        pub struct $name(String);

        impl $name {
            pub fn new(value: impl Into<String>) -> Self {
                Self::try_new(value).expect(concat!($kind, " IDs must be namespaced"))
            }

            pub fn try_new(value: impl Into<String>) -> Result<Self, GraphContractError> {
                let value = value.into();
                validate_graph_id(&value, $kind)?;
                Ok(Self(value))
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }
    };
}

graph_id!(ControlNodeId, "control node");
graph_id!(ControlPackageRef, "control package");
graph_id!(ControlKindRef, "control kind");
graph_id!(ControlKernelRef, "control kernel");
graph_id!(ControlPropertySnapshotId, "control property snapshot");
graph_id!(LayoutConstraintId, "layout constraint");
graph_id!(StateRequirementId, "state requirement");
graph_id!(StyleRuleId, "style rule");
graph_id!(StyleSlotId, "style slot");
graph_id!(InteractionHandlerId, "interaction handler");
graph_id!(BindingEdgeId, "binding edge");
graph_id!(BindingEndpointId, "binding endpoint");
graph_id!(VisualOperatorId, "visual operator");
graph_id!(AccessibilityNodeId, "accessibility node");
graph_id!(InspectionEntryId, "inspection entry");

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GraphContractError {
    EmptyId { kind: &'static str },
    UnnamespacedId { kind: &'static str, value: String },
    InvalidIdCharacter { kind: &'static str, value: String },
}

impl fmt::Display for GraphContractError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyId { kind } => write!(formatter, "empty UI graph {kind} id"),
            Self::UnnamespacedId { kind, value } => {
                write!(formatter, "UI graph {kind} id {value} is not namespaced")
            }
            Self::InvalidIdCharacter { kind, value } => write!(
                formatter,
                "UI graph {kind} id {value} contains an invalid character"
            ),
        }
    }
}

impl std::error::Error for GraphContractError {}

fn validate_graph_id(value: &str, kind: &'static str) -> Result<(), GraphContractError> {
    if value.is_empty() {
        return Err(GraphContractError::EmptyId { kind });
    }
    if !value.contains('.') {
        return Err(GraphContractError::UnnamespacedId {
            kind,
            value: value.to_owned(),
        });
    }
    if !value
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || matches!(character, '.' | '_' | '-'))
    {
        return Err(GraphContractError::InvalidIdCharacter {
            kind,
            value: value.to_owned(),
        });
    }
    Ok(())
}
