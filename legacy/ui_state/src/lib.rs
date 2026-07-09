//! File: domain/ui/ui_state/src/lib.rs
//! Crate: ui_state

use std::collections::BTreeMap;
use std::fmt;

use serde::{Deserialize, Serialize};
use ui_program::{
    StatePersistence, StateRequirement, StateRequirementId, StateRequirementLifecycle,
};
use ui_schema::{UiSchemaRef, UiSchemaValue};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct UiStateKey(String);

impl UiStateKey {
    pub fn new(value: impl Into<String>) -> Self {
        Self::try_new(value).expect("UI state keys must be stable namespaced IDs")
    }

    pub fn try_new(value: impl Into<String>) -> Result<Self, UiStateError> {
        let value = value.into();
        validate_state_id(&value)?;
        Ok(Self(value))
    }

    pub fn from_requirement_id(requirement_id: &StateRequirementId) -> Self {
        Self::new(requirement_id.as_str())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for UiStateKey {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for UiStateKey {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UiStateBucket {
    Transient,
    Preview,
    Committed,
    Focus,
    Hover,
    Drag,
    Animation,
    HostFed,
    PackageOwned,
}

impl UiStateBucket {
    pub const fn from_lifecycle(lifecycle: StateRequirementLifecycle) -> Self {
        match lifecycle {
            StateRequirementLifecycle::Transient | StateRequirementLifecycle::PressedCaptured => {
                Self::Transient
            }
            StateRequirementLifecycle::Preview => Self::Preview,
            StateRequirementLifecycle::Committed => Self::Committed,
            StateRequirementLifecycle::Focus => Self::Focus,
            StateRequirementLifecycle::Hover => Self::Hover,
            StateRequirementLifecycle::Drag => Self::Drag,
            StateRequirementLifecycle::Animation => Self::Animation,
            StateRequirementLifecycle::HostFed => Self::HostFed,
            StateRequirementLifecycle::PackageOwned => Self::PackageOwned,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct UiStateCell {
    pub key: UiStateKey,
    pub owner_control_id: String,
    pub bucket: UiStateBucket,
    pub schema: UiSchemaRef,
    pub persistence: StatePersistence,
    #[serde(default)]
    pub value: Option<UiSchemaValue>,
    #[serde(default)]
    pub revision: u64,
}

impl UiStateCell {
    pub fn from_requirement(requirement: &StateRequirement) -> Self {
        Self {
            key: UiStateKey::from_requirement_id(&requirement.requirement_id),
            owner_control_id: requirement.owner_control.as_str().to_owned(),
            bucket: UiStateBucket::from_lifecycle(requirement.lifecycle),
            schema: requirement.schema.clone(),
            persistence: requirement.persistence,
            value: None,
            revision: 0,
        }
    }

    pub fn set_value(&mut self, value: UiSchemaValue) {
        self.value = Some(value);
        self.revision = self.revision.saturating_add(1);
    }

    pub fn clear_value(&mut self) {
        self.value = None;
        self.revision = self.revision.saturating_add(1);
    }
}

pub trait UiStateContract {
    const BUCKET: UiStateBucket;

    fn key(&self) -> &UiStateKey;
    fn schema(&self) -> &UiSchemaRef;
    fn persistence(&self) -> StatePersistence;

    fn to_cell(&self, owner_control_id: impl Into<String>) -> UiStateCell {
        UiStateCell {
            key: self.key().clone(),
            owner_control_id: owner_control_id.into(),
            bucket: Self::BUCKET,
            schema: self.schema().clone(),
            persistence: self.persistence(),
            value: None,
            revision: 0,
        }
    }
}

macro_rules! define_state_contract {
    ($type_name:ident, $bucket:expr) => {
        #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
        pub struct $type_name {
            pub key: UiStateKey,
            pub schema: UiSchemaRef,
            pub persistence: StatePersistence,
        }

        impl $type_name {
            pub fn new(key: impl Into<UiStateKey>, schema: UiSchemaRef) -> Self {
                Self {
                    key: key.into(),
                    schema,
                    persistence: StatePersistence::Ephemeral,
                }
            }

            pub fn with_persistence(mut self, persistence: StatePersistence) -> Self {
                self.persistence = persistence;
                self
            }
        }

        impl UiStateContract for $type_name {
            const BUCKET: UiStateBucket = $bucket;

            fn key(&self) -> &UiStateKey {
                &self.key
            }

            fn schema(&self) -> &UiSchemaRef {
                &self.schema
            }

            fn persistence(&self) -> StatePersistence {
                self.persistence
            }
        }
    };
}

define_state_contract!(TransientState, UiStateBucket::Transient);
define_state_contract!(PreviewState, UiStateBucket::Preview);
define_state_contract!(CommittedState, UiStateBucket::Committed);
define_state_contract!(FocusState, UiStateBucket::Focus);
define_state_contract!(HoverState, UiStateBucket::Hover);
define_state_contract!(DragState, UiStateBucket::Drag);
define_state_contract!(AnimationState, UiStateBucket::Animation);
define_state_contract!(HostFedState, UiStateBucket::HostFed);
define_state_contract!(PackageOwnedState, UiStateBucket::PackageOwned);

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct UiStateModel {
    cells: BTreeMap<UiStateKey, UiStateCell>,
}

impl UiStateModel {
    pub fn ensure_requirement(&mut self, requirement: &StateRequirement) -> &mut UiStateCell {
        let key = UiStateKey::from_requirement_id(&requirement.requirement_id);
        self.cells
            .entry(key)
            .or_insert_with(|| UiStateCell::from_requirement(requirement))
    }

    pub fn ensure_requirements<'a>(
        &mut self,
        requirements: impl IntoIterator<Item = &'a StateRequirement>,
    ) {
        for requirement in requirements {
            self.ensure_requirement(requirement);
        }
    }

    pub fn insert_cell(&mut self, cell: UiStateCell) -> Option<UiStateCell> {
        self.cells.insert(cell.key.clone(), cell)
    }

    pub fn cell(&self, key: &UiStateKey) -> Option<&UiStateCell> {
        self.cells.get(key)
    }

    pub fn cells(&self) -> impl Iterator<Item = &UiStateCell> {
        self.cells.values()
    }

    pub fn bucket_cells(&self, bucket: UiStateBucket) -> impl Iterator<Item = &UiStateCell> {
        self.cells
            .values()
            .filter(move |cell| cell.bucket == bucket)
    }

    pub fn value(&self, key: impl Into<UiStateKey>) -> Option<&UiSchemaValue> {
        let key = key.into();
        self.cells.get(&key).and_then(|cell| cell.value.as_ref())
    }

    pub fn set_value(
        &mut self,
        key: impl Into<UiStateKey>,
        value: UiSchemaValue,
    ) -> Result<(), UiStateError> {
        let key = key.into();
        let cell = self
            .cells
            .get_mut(&key)
            .ok_or_else(|| UiStateError::UnknownStateKey {
                key: key.as_str().to_owned(),
            })?;
        cell.set_value(value);
        Ok(())
    }

    pub fn apply_preview(
        &mut self,
        key: impl Into<UiStateKey>,
        value: UiSchemaValue,
    ) -> Result<(), UiStateError> {
        self.set_bucket_value(key, UiStateBucket::Preview, value)
    }

    pub fn apply_host_value(
        &mut self,
        key: impl Into<UiStateKey>,
        value: UiSchemaValue,
    ) -> Result<(), UiStateError> {
        self.set_bucket_value(key, UiStateBucket::HostFed, value)
    }

    pub fn apply_package_value(
        &mut self,
        key: impl Into<UiStateKey>,
        value: UiSchemaValue,
    ) -> Result<(), UiStateError> {
        self.set_bucket_value(key, UiStateBucket::PackageOwned, value)
    }

    pub fn set_activity(
        &mut self,
        key: impl Into<UiStateKey>,
        bucket: UiStateBucket,
        active: bool,
    ) -> Result<(), UiStateError> {
        match bucket {
            UiStateBucket::Focus | UiStateBucket::Hover | UiStateBucket::Drag => {
                self.set_bucket_value(key, bucket, UiSchemaValue::bool(active))
            }
            other => Err(UiStateError::InvalidActivityBucket { bucket: other }),
        }
    }

    pub fn commit_preview(
        &mut self,
        preview_key: impl Into<UiStateKey>,
        committed_key: impl Into<UiStateKey>,
    ) -> Result<UiSchemaValue, UiStateError> {
        let preview_key = preview_key.into();
        let committed_key = committed_key.into();
        let value = {
            let preview =
                self.cells
                    .get(&preview_key)
                    .ok_or_else(|| UiStateError::UnknownStateKey {
                        key: preview_key.as_str().to_owned(),
                    })?;
            if preview.bucket != UiStateBucket::Preview {
                return Err(UiStateError::WrongStateBucket {
                    key: preview_key.as_str().to_owned(),
                    expected: UiStateBucket::Preview,
                    actual: preview.bucket,
                });
            }
            preview
                .value
                .clone()
                .ok_or_else(|| UiStateError::MissingStateValue {
                    key: preview_key.as_str().to_owned(),
                })?
        };

        {
            let committed = self.cells.get_mut(&committed_key).ok_or_else(|| {
                UiStateError::UnknownStateKey {
                    key: committed_key.as_str().to_owned(),
                }
            })?;
            if committed.bucket != UiStateBucket::Committed {
                return Err(UiStateError::WrongStateBucket {
                    key: committed_key.as_str().to_owned(),
                    expected: UiStateBucket::Committed,
                    actual: committed.bucket,
                });
            }
            committed.set_value(value.clone());
        }

        if let Some(preview) = self.cells.get_mut(&preview_key) {
            preview.clear_value();
        }

        Ok(value)
    }

    fn set_bucket_value(
        &mut self,
        key: impl Into<UiStateKey>,
        bucket: UiStateBucket,
        value: UiSchemaValue,
    ) -> Result<(), UiStateError> {
        let key = key.into();
        let cell = self
            .cells
            .get_mut(&key)
            .ok_or_else(|| UiStateError::UnknownStateKey {
                key: key.as_str().to_owned(),
            })?;
        if cell.bucket != bucket {
            return Err(UiStateError::WrongStateBucket {
                key: key.as_str().to_owned(),
                expected: bucket,
                actual: cell.bucket,
            });
        }
        cell.set_value(value);
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UiStateError {
    EmptyStateKey,
    UnnamespacedStateKey {
        value: String,
    },
    InvalidStateKeyCharacter {
        value: String,
    },
    UnknownStateKey {
        key: String,
    },
    MissingStateValue {
        key: String,
    },
    WrongStateBucket {
        key: String,
        expected: UiStateBucket,
        actual: UiStateBucket,
    },
    InvalidActivityBucket {
        bucket: UiStateBucket,
    },
}

impl fmt::Display for UiStateError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyStateKey => write!(formatter, "empty UI state key"),
            Self::UnnamespacedStateKey { value } => {
                write!(formatter, "UI state key {value} is not namespaced")
            }
            Self::InvalidStateKeyCharacter { value } => {
                write!(
                    formatter,
                    "UI state key {value} contains an invalid character"
                )
            }
            Self::UnknownStateKey { key } => write!(formatter, "unknown UI state key {key}"),
            Self::MissingStateValue { key } => {
                write!(formatter, "UI state key {key} has no value to commit")
            }
            Self::WrongStateBucket {
                key,
                expected,
                actual,
            } => write!(
                formatter,
                "UI state key {key} belongs to {actual:?}, expected {expected:?}"
            ),
            Self::InvalidActivityBucket { bucket } => {
                write!(
                    formatter,
                    "UI state bucket {bucket:?} is not an activity bucket"
                )
            }
        }
    }
}

impl std::error::Error for UiStateError {}

fn validate_state_id(value: &str) -> Result<(), UiStateError> {
    if value.is_empty() {
        return Err(UiStateError::EmptyStateKey);
    }
    if !value.contains('.') {
        return Err(UiStateError::UnnamespacedStateKey {
            value: value.to_owned(),
        });
    }
    if !value
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || matches!(character, '.' | '_' | '-'))
    {
        return Err(UiStateError::InvalidStateKeyCharacter {
            value: value.to_owned(),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use ui_program::{
        ControlNodeId, StatePersistence, StateRequirement, StateRequirementId,
        StateRequirementLifecycle,
    };

    #[test]
    fn state_contract_classifies_lifecycle_buckets_and_commits_preview() {
        let mut state = UiStateModel::default();
        for lifecycle in [
            StateRequirementLifecycle::Transient,
            StateRequirementLifecycle::Preview,
            StateRequirementLifecycle::Committed,
            StateRequirementLifecycle::Focus,
            StateRequirementLifecycle::Hover,
            StateRequirementLifecycle::Drag,
            StateRequirementLifecycle::Animation,
            StateRequirementLifecycle::HostFed,
            StateRequirementLifecycle::PackageOwned,
        ] {
            state.ensure_requirement(&requirement(lifecycle));
        }

        state
            .apply_preview("state.preview", UiSchemaValue::number(0.5))
            .expect("preview state exists");
        let committed = state
            .commit_preview("state.preview", "state.committed")
            .expect("preview should commit into committed state");
        state
            .set_activity("state.focus", UiStateBucket::Focus, true)
            .expect("focus state exists");
        state
            .set_activity("state.hover", UiStateBucket::Hover, true)
            .expect("hover state exists");
        state
            .set_activity("state.drag", UiStateBucket::Drag, true)
            .expect("drag state exists");
        state
            .apply_host_value("state.host_fed", UiSchemaValue::string("host-value"))
            .expect("host-fed state exists");
        state
            .apply_package_value(
                "state.package_owned",
                UiSchemaValue::string("package-value"),
            )
            .expect("package-owned state exists");

        assert_eq!(committed, UiSchemaValue::number(0.5));
        assert_eq!(
            state.value("state.committed"),
            Some(&UiSchemaValue::number(0.5))
        );
        assert_eq!(state.value("state.preview"), None);
        assert_eq!(
            state.value("state.focus").and_then(UiSchemaValue::as_bool),
            Some(true)
        );
        assert_eq!(
            state.value("state.host_fed"),
            Some(&UiSchemaValue::string("host-value"))
        );
        assert_eq!(
            state
                .bucket_cells(UiStateBucket::PackageOwned)
                .next()
                .map(|cell| cell.persistence),
            Some(StatePersistence::HostBacked)
        );
    }

    #[test]
    fn named_state_contracts_materialize_owner_buckets() {
        let contracts = [
            TransientState::new("state.transient", UiSchemaRef::new("ui.state.transient", 1))
                .to_cell("control.root"),
            PreviewState::new("state.preview", UiSchemaRef::new("ui.state.preview", 1))
                .to_cell("control.root"),
            CommittedState::new("state.committed", UiSchemaRef::new("ui.state.committed", 1))
                .to_cell("control.root"),
            FocusState::new("state.focus", UiSchemaRef::new("ui.state.focus", 1))
                .to_cell("control.root"),
            HoverState::new("state.hover", UiSchemaRef::new("ui.state.hover", 1))
                .to_cell("control.root"),
            DragState::new("state.drag", UiSchemaRef::new("ui.state.drag", 1))
                .to_cell("control.root"),
            AnimationState::new("state.animation", UiSchemaRef::new("ui.state.animation", 1))
                .to_cell("control.root"),
            HostFedState::new("state.host_fed", UiSchemaRef::new("ui.state.host_fed", 1))
                .with_persistence(StatePersistence::HostBacked)
                .to_cell("control.root"),
            PackageOwnedState::new(
                "state.package_owned",
                UiSchemaRef::new("ui.state.package_owned", 1),
            )
            .with_persistence(StatePersistence::Retained)
            .to_cell("control.root"),
        ];

        assert_eq!(
            contracts.iter().map(|cell| cell.bucket).collect::<Vec<_>>(),
            [
                UiStateBucket::Transient,
                UiStateBucket::Preview,
                UiStateBucket::Committed,
                UiStateBucket::Focus,
                UiStateBucket::Hover,
                UiStateBucket::Drag,
                UiStateBucket::Animation,
                UiStateBucket::HostFed,
                UiStateBucket::PackageOwned,
            ]
        );
        assert_eq!(contracts[7].persistence, StatePersistence::HostBacked);
        assert_eq!(contracts[8].persistence, StatePersistence::Retained);
    }

    fn requirement(lifecycle: StateRequirementLifecycle) -> StateRequirement {
        let id = match lifecycle {
            StateRequirementLifecycle::Transient => "state.transient",
            StateRequirementLifecycle::Preview => "state.preview",
            StateRequirementLifecycle::Committed => "state.committed",
            StateRequirementLifecycle::Focus => "state.focus",
            StateRequirementLifecycle::Hover => "state.hover",
            StateRequirementLifecycle::PressedCaptured => "state.pressed",
            StateRequirementLifecycle::Drag => "state.drag",
            StateRequirementLifecycle::Animation => "state.animation",
            StateRequirementLifecycle::HostFed => "state.host_fed",
            StateRequirementLifecycle::PackageOwned => "state.package_owned",
        };
        StateRequirement::new(
            StateRequirementId::new(id),
            ControlNodeId::new("control.root"),
            lifecycle,
            UiSchemaRef::new("ui.state.value", 1),
        )
        .with_persistence(StatePersistence::HostBacked)
    }
}
