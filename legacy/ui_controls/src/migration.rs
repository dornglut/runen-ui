//! File: domain/ui/ui_controls/src/migration.rs
//! Crate: ui_controls

use std::fmt;

use serde::{Deserialize, Serialize};

use crate::package::{ControlKindId, ControlPackageId, ControlPackageVersion};

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ControlMigrationId(String);

impl ControlMigrationId {
    pub fn new(value: impl Into<String>) -> Self {
        Self::try_new(value).expect("control migration IDs must be namespaced")
    }

    pub fn try_new(value: impl Into<String>) -> Result<Self, ControlMigrationContractError> {
        let value = value.into();
        validate_migration_id(&value, "migration")?;
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct ControlMigrationVersion(u32);

impl ControlMigrationVersion {
    pub const fn new(value: u32) -> Self {
        assert!(value > 0);
        Self(value)
    }

    pub const fn value(self) -> u32 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ControlMigrationGraphPolicy {
    PreservesGraph,
    RewritesGraph,
    RemovesCapabilities,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ControlBreakingChangePolicy {
    NonBreaking,
    BreakingRequiresDiagnostic,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlReplacementTarget {
    pub package_id: ControlPackageId,
    pub control_kind_id: Option<ControlKindId>,
}

impl ControlReplacementTarget {
    pub fn package(package_id: ControlPackageId) -> Self {
        Self {
            package_id,
            control_kind_id: None,
        }
    }

    pub fn kind(package_id: ControlPackageId, control_kind_id: ControlKindId) -> Self {
        Self {
            package_id,
            control_kind_id: Some(control_kind_id),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ControlDeprecationStatus {
    Active,
    Deprecated {
        reason: String,
        replacement: Option<ControlReplacementTarget>,
    },
    Removed {
        reason: String,
        replacement: Option<ControlReplacementTarget>,
    },
}

impl Default for ControlDeprecationStatus {
    fn default() -> Self {
        Self::Active
    }
}

impl ControlDeprecationStatus {
    pub fn replacement_package_id(&self) -> Option<&ControlPackageId> {
        match self {
            Self::Active => None,
            Self::Deprecated { replacement, .. } | Self::Removed { replacement, .. } => {
                replacement.as_ref().map(|target| &target.package_id)
            }
        }
    }

    pub fn replacement_control_kind_id(&self) -> Option<&ControlKindId> {
        match self {
            Self::Active => None,
            Self::Deprecated { replacement, .. } | Self::Removed { replacement, .. } => replacement
                .as_ref()
                .and_then(|target| target.control_kind_id.as_ref()),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlMigrationHook {
    pub migration_id: ControlMigrationId,
    pub migration_version: ControlMigrationVersion,
    pub from_package_version: Option<ControlPackageVersion>,
    pub to_package_version: ControlPackageVersion,
    pub graph_policy: ControlMigrationGraphPolicy,
    pub preserves_source_maps: bool,
    pub breaking_change_policy: ControlBreakingChangePolicy,
    pub replacement: Option<ControlReplacementTarget>,
    pub diagnostic_id: Option<String>,
}

impl ControlMigrationHook {
    pub fn initial(
        migration_id: ControlMigrationId,
        to_package_version: ControlPackageVersion,
    ) -> Self {
        Self {
            migration_id,
            migration_version: ControlMigrationVersion::new(1),
            from_package_version: None,
            to_package_version,
            graph_policy: ControlMigrationGraphPolicy::PreservesGraph,
            preserves_source_maps: true,
            breaking_change_policy: ControlBreakingChangePolicy::NonBreaking,
            replacement: None,
            diagnostic_id: None,
        }
    }

    pub fn with_graph_policy(mut self, graph_policy: ControlMigrationGraphPolicy) -> Self {
        self.graph_policy = graph_policy;
        self
    }

    pub fn with_breaking_change_policy(
        mut self,
        breaking_change_policy: ControlBreakingChangePolicy,
        diagnostic_id: impl Into<String>,
    ) -> Self {
        self.breaking_change_policy = breaking_change_policy;
        self.diagnostic_id = Some(diagnostic_id.into());
        self
    }

    pub fn with_replacement_target(mut self, replacement: ControlReplacementTarget) -> Self {
        self.replacement = Some(replacement);
        self
    }

    pub fn validate_contract(&self, _package_id: &ControlPackageId) -> Result<(), String> {
        if let Some(from_package_version) = self.from_package_version {
            if self.to_package_version.value() <= from_package_version.value() {
                return Err(format!(
                    "control migration {} must move to a greater package version",
                    self.migration_id.as_str()
                ));
            }
        }
        if self.breaking_change_policy == ControlBreakingChangePolicy::BreakingRequiresDiagnostic
            && self.diagnostic_id.as_deref().map_or(true, str::is_empty)
        {
            return Err(format!(
                "breaking control migration {} must name a diagnostic id",
                self.migration_id.as_str()
            ));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ControlMigrationContractError {
    EmptyId { kind: &'static str },
    UnnamespacedId { kind: &'static str, value: String },
    InvalidIdCharacter { kind: &'static str, value: String },
}

impl fmt::Display for ControlMigrationContractError {
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

impl std::error::Error for ControlMigrationContractError {}

fn validate_migration_id(
    value: &str,
    kind: &'static str,
) -> Result<(), ControlMigrationContractError> {
    if value.is_empty() {
        return Err(ControlMigrationContractError::EmptyId { kind });
    }
    if !value.contains('.') {
        return Err(ControlMigrationContractError::UnnamespacedId {
            kind,
            value: value.to_owned(),
        });
    }
    if !value
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || matches!(character, '.' | '_' | '-'))
    {
        return Err(ControlMigrationContractError::InvalidIdCharacter {
            kind,
            value: value.to_owned(),
        });
    }
    Ok(())
}
