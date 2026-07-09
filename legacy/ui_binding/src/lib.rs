//! File: domain/ui/ui_binding/src/lib.rs
//! Crate: ui_binding

use std::collections::BTreeMap;
use std::fmt;

use serde::{Deserialize, Serialize};
use ui_artifacts::{
    BindingSnapshotTable, CollectionDiffPlan, CollectionDiffPlanEntry, RuntimeBindingEndpoint,
    RuntimeSchemaRef,
};
use ui_program::BindingEndpoint;
use ui_schema::{UiSchemaRef, UiSchemaValue};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct BindingId(String);

impl BindingId {
    pub fn new(value: impl Into<String>) -> Self {
        Self::try_new(value).expect("binding IDs must be stable namespaced IDs")
    }

    pub fn try_new(value: impl Into<String>) -> Result<Self, BindingContractError> {
        let value = value.into();
        validate_binding_id(&value, "binding")?;
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for BindingId {
    fn from(value: &str) -> Self {
        Self::new(value)
    }
}

impl From<String> for BindingId {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum BindingEndpointAddress {
    ControlProperty {
        control_id: String,
        endpoint_id: String,
    },
    UiState {
        requirement_id: String,
        endpoint_id: String,
    },
    HostData {
        endpoint_id: String,
    },
}

impl BindingEndpointAddress {
    pub fn from_program_endpoint(endpoint: &BindingEndpoint) -> Self {
        match endpoint {
            BindingEndpoint::ControlProperty {
                control_id,
                endpoint_id,
            } => Self::ControlProperty {
                control_id: control_id.as_str().to_owned(),
                endpoint_id: endpoint_id.as_str().to_owned(),
            },
            BindingEndpoint::UiState {
                requirement_id,
                endpoint_id,
            } => Self::UiState {
                requirement_id: requirement_id.as_str().to_owned(),
                endpoint_id: endpoint_id.as_str().to_owned(),
            },
            BindingEndpoint::HostData { endpoint_id } => Self::HostData {
                endpoint_id: endpoint_id.as_str().to_owned(),
            },
        }
    }

    pub fn from_runtime_endpoint(endpoint: &RuntimeBindingEndpoint) -> Self {
        match endpoint {
            RuntimeBindingEndpoint::ControlProperty {
                control_id,
                endpoint_id,
            } => Self::ControlProperty {
                control_id: control_id.to_owned(),
                endpoint_id: endpoint_id.to_owned(),
            },
            RuntimeBindingEndpoint::UiState {
                requirement_id,
                endpoint_id,
            } => Self::UiState {
                requirement_id: requirement_id.to_owned(),
                endpoint_id: endpoint_id.to_owned(),
            },
            RuntimeBindingEndpoint::HostData { endpoint_id } => Self::HostData {
                endpoint_id: endpoint_id.to_owned(),
            },
        }
    }

    pub fn host_data_endpoint(&self) -> Option<&str> {
        match self {
            Self::HostData { endpoint_id } => Some(endpoint_id),
            Self::ControlProperty { .. } | Self::UiState { .. } => None,
        }
    }

    pub fn ui_state_requirement(&self) -> Option<&str> {
        match self {
            Self::UiState { requirement_id, .. } => Some(requirement_id),
            Self::ControlProperty { .. } | Self::HostData { .. } => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BindingValueSchema {
    pub schema_id: String,
    pub schema_version: u32,
}

impl BindingValueSchema {
    pub fn from_schema_ref(schema: &UiSchemaRef) -> Self {
        Self {
            schema_id: schema.id.as_str().to_owned(),
            schema_version: schema.version.value(),
        }
    }

    pub fn from_runtime_schema(schema: &RuntimeSchemaRef) -> Self {
        Self {
            schema_id: schema.schema_id.to_owned(),
            schema_version: schema.schema_version,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct BindingSnapshot {
    pub binding_id: BindingId,
    pub source: BindingEndpointAddress,
    pub target: BindingEndpointAddress,
    pub value_schema: BindingValueSchema,
    #[serde(default)]
    pub required_capabilities: Vec<String>,
    #[serde(default)]
    pub value: Option<UiSchemaValue>,
    #[serde(default)]
    pub revision: u64,
    #[serde(default)]
    pub dirty: bool,
    #[serde(default)]
    pub authorized_read: bool,
    #[serde(default)]
    pub authorized_write: bool,
    #[serde(default)]
    pub source_map_index: Option<u32>,
}

impl BindingSnapshot {
    pub fn from_plan_entry(entry: &CollectionDiffPlanEntry) -> Self {
        Self {
            binding_id: BindingId::new(&entry.edge_id),
            source: BindingEndpointAddress::from_runtime_endpoint(&entry.source),
            target: BindingEndpointAddress::from_runtime_endpoint(&entry.target),
            value_schema: BindingValueSchema::from_runtime_schema(&entry.value_schema),
            required_capabilities: Vec::new(),
            value: None,
            revision: 0,
            dirty: false,
            authorized_read: true,
            authorized_write: true,
            source_map_index: None,
        }
    }

    pub fn with_value(mut self, value: UiSchemaValue, revision: u64) -> Self {
        self.value = Some(value);
        self.revision = revision;
        self
    }

    pub fn mark_dirty(mut self) -> Self {
        self.dirty = true;
        self
    }

    pub fn is_authorized(&self) -> bool {
        self.authorized_read && self.authorized_write
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct HostDataSnapshot {
    pub endpoint_id: String,
    pub value: UiSchemaValue,
    pub revision: u64,
}

impl HostDataSnapshot {
    pub fn new(endpoint_id: impl Into<String>, value: UiSchemaValue, revision: u64) -> Self {
        Self {
            endpoint_id: endpoint_id.into(),
            value,
            revision,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HostDataRevisionPolicy {
    Monotonic,
    Exact,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct HostDataContract {
    pub endpoint_id: String,
    pub value_schema: BindingValueSchema,
    pub revision_policy: HostDataRevisionPolicy,
    #[serde(default)]
    pub required_capabilities: Vec<String>,
}

impl HostDataContract {
    pub fn new(endpoint_id: impl Into<String>, value_schema: BindingValueSchema) -> Self {
        Self {
            endpoint_id: endpoint_id.into(),
            value_schema,
            revision_policy: HostDataRevisionPolicy::Monotonic,
            required_capabilities: Vec::new(),
        }
    }

    pub fn with_capability(mut self, capability: impl Into<String>) -> Self {
        self.required_capabilities.push(capability.into());
        self
    }

    pub fn with_revision_policy(mut self, revision_policy: HostDataRevisionPolicy) -> Self {
        self.revision_policy = revision_policy;
        self
    }

    pub fn accepts_revision(&self, current_revision: u64, candidate_revision: u64) -> bool {
        match self.revision_policy {
            HostDataRevisionPolicy::Monotonic => candidate_revision > current_revision,
            HostDataRevisionPolicy::Exact => candidate_revision == current_revision,
        }
    }

    pub fn authorizes(&self, authorizations: &[BindingAuthorization]) -> bool {
        self.required_capabilities.iter().all(|required| {
            authorizations.iter().any(|authorization| {
                authorization.capability == *required
                    && authorization.can_read
                    && authorization.can_write
            })
        })
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum CollectionDiff {
    Insert { index: usize },
    Remove { index: usize },
    Move { from: usize, to: usize },
    Replace { index: usize },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BindingCollectionDiff {
    pub binding_id: BindingId,
    pub diff: CollectionDiff,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct DirtyPropagation {
    pub dirty_bindings: Vec<BindingId>,
    pub affected_endpoints: Vec<BindingEndpointAddress>,
    pub collection_diffs: Vec<BindingCollectionDiff>,
    pub diagnostics: Vec<BindingDiagnostic>,
}

impl DirtyPropagation {
    pub fn from_report(report: BindingDirtyReport) -> Self {
        Self {
            dirty_bindings: report.dirty_bindings,
            affected_endpoints: report.affected_endpoints,
            collection_diffs: report.collection_diffs,
            diagnostics: report.diagnostics,
        }
    }

    pub fn passed(&self) -> bool {
        self.diagnostics.is_empty()
    }

    pub fn has_dirty_binding(&self, binding_id: &BindingId) -> bool {
        self.dirty_bindings.contains(binding_id)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BindingAuthorization {
    pub capability: String,
    pub can_read: bool,
    pub can_write: bool,
}

impl BindingAuthorization {
    pub fn read_write(capability: impl Into<String>) -> Self {
        Self {
            capability: capability.into(),
            can_read: true,
            can_write: true,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct BindingSnapshotSet {
    snapshots: BTreeMap<BindingId, BindingSnapshot>,
}

impl BindingSnapshotSet {
    pub fn from_table(table: &BindingSnapshotTable) -> Self {
        let mut snapshots = BTreeMap::new();
        for row in &table.rows {
            let binding_id = BindingId::new(row.binding.edge_id.as_str());
            let snapshot = BindingSnapshot {
                binding_id: binding_id.clone(),
                source: BindingEndpointAddress::from_program_endpoint(&row.binding.source),
                target: BindingEndpointAddress::from_program_endpoint(&row.binding.target),
                value_schema: BindingValueSchema::from_schema_ref(&row.binding.value_schema),
                required_capabilities: row
                    .binding
                    .required_capabilities
                    .iter()
                    .map(|capability| capability.as_str().to_owned())
                    .collect(),
                value: None,
                revision: 0,
                dirty: false,
                authorized_read: row.binding.required_capabilities.is_empty(),
                authorized_write: row.binding.required_capabilities.is_empty(),
                source_map_index: row.source_map_index,
            };
            snapshots.insert(binding_id, snapshot);
        }
        Self { snapshots }
    }

    pub fn snapshots(&self) -> impl Iterator<Item = &BindingSnapshot> {
        self.snapshots.values()
    }

    pub fn snapshot(&self, binding_id: &BindingId) -> Option<&BindingSnapshot> {
        self.snapshots.get(binding_id)
    }

    pub fn apply_authorizations(&mut self, authorizations: &[BindingAuthorization]) {
        for snapshot in self.snapshots.values_mut() {
            if snapshot.required_capabilities.is_empty() {
                snapshot.authorized_read = true;
                snapshot.authorized_write = true;
                continue;
            }

            snapshot.authorized_read = snapshot.required_capabilities.iter().all(|required| {
                authorizations.iter().any(|authorization| {
                    authorization.capability == *required && authorization.can_read
                })
            });
            snapshot.authorized_write = snapshot.required_capabilities.iter().all(|required| {
                authorizations.iter().any(|authorization| {
                    authorization.capability == *required && authorization.can_write
                })
            });
        }
    }

    pub fn apply_host_data(&mut self, host_data: &[HostDataSnapshot]) -> BindingDirtyReport {
        let mut report = BindingDirtyReport::default();
        for data in host_data {
            let mut matched = false;
            for snapshot in self.snapshots.values_mut() {
                if snapshot.source.host_data_endpoint() != Some(data.endpoint_id.as_str()) {
                    continue;
                }
                matched = true;
                if !snapshot.authorized_write {
                    report.diagnostics.push(BindingDiagnostic {
                        code: "ui.binding.authorization.write_denied".to_owned(),
                        binding_id: Some(snapshot.binding_id.clone()),
                        message: format!(
                            "binding {} is not authorized to write host data {}",
                            snapshot.binding_id.as_str(),
                            data.endpoint_id
                        ),
                    });
                    continue;
                }
                snapshot.value = Some(data.value.clone());
                snapshot.revision = data.revision;
                snapshot.dirty = true;
                push_unique(&mut report.dirty_bindings, snapshot.binding_id.clone());
                push_unique(&mut report.affected_endpoints, snapshot.target.clone());
            }

            if !matched {
                report.diagnostics.push(BindingDiagnostic {
                    code: "ui.binding.host_data.unmapped".to_owned(),
                    binding_id: None,
                    message: format!("host data endpoint {} has no binding", data.endpoint_id),
                });
            }
        }
        report
    }

    pub fn collection_diffs(&self, plan: &CollectionDiffPlan) -> Vec<BindingCollectionDiff> {
        plan.rows
            .iter()
            .enumerate()
            .filter_map(|(index, row)| {
                let binding_id = BindingId::new(&row.edge_id);
                self.snapshots
                    .get(&binding_id)
                    .filter(|snapshot| snapshot.dirty)
                    .map(|_| BindingCollectionDiff {
                        binding_id,
                        diff: CollectionDiff::Replace { index },
                    })
            })
            .collect()
    }
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct BindingDirtyReport {
    pub dirty_bindings: Vec<BindingId>,
    pub affected_endpoints: Vec<BindingEndpointAddress>,
    pub collection_diffs: Vec<BindingCollectionDiff>,
    pub diagnostics: Vec<BindingDiagnostic>,
}

impl BindingDirtyReport {
    pub fn with_collection_diffs(mut self, diffs: Vec<BindingCollectionDiff>) -> Self {
        self.collection_diffs = diffs;
        self
    }

    pub fn passed(&self) -> bool {
        self.diagnostics.is_empty()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct BindingDiagnostic {
    pub code: String,
    #[serde(default)]
    pub binding_id: Option<BindingId>,
    pub message: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BindingContractError {
    EmptyId { kind: &'static str },
    UnnamespacedId { kind: &'static str, value: String },
    InvalidIdCharacter { kind: &'static str, value: String },
}

impl fmt::Display for BindingContractError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyId { kind } => write!(formatter, "empty UI binding {kind} id"),
            Self::UnnamespacedId { kind, value } => {
                write!(formatter, "UI binding {kind} id {value} is not namespaced")
            }
            Self::InvalidIdCharacter { kind, value } => write!(
                formatter,
                "UI binding {kind} id {value} contains an invalid character"
            ),
        }
    }
}

impl std::error::Error for BindingContractError {}

fn validate_binding_id(value: &str, kind: &'static str) -> Result<(), BindingContractError> {
    if value.is_empty() {
        return Err(BindingContractError::EmptyId { kind });
    }
    if !value.contains('.') {
        return Err(BindingContractError::UnnamespacedId {
            kind,
            value: value.to_owned(),
        });
    }
    if !value
        .chars()
        .all(|character| character.is_ascii_alphanumeric() || matches!(character, '.' | '_' | '-'))
    {
        return Err(BindingContractError::InvalidIdCharacter {
            kind,
            value: value.to_owned(),
        });
    }
    Ok(())
}

fn push_unique<T>(values: &mut Vec<T>, value: T)
where
    T: PartialEq,
{
    if !values.contains(&value) {
        values.push(value);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ui_artifacts::UiRuntimeArtifact;
    use ui_program::{
        BindingEdge, BindingEdgeId, BindingEndpoint, BindingEndpointId, ControlGraphNode,
        ControlKindRef, ControlNodeId, ControlPackageRef, RouteCapability, StateRequirement,
        StateRequirementId, StateRequirementLifecycle, UiProgram, UiProgramId, UiProgramVersion,
    };

    #[test]
    fn binding_contract_tracks_host_data_dirty_state_collection_diffs_and_authorization() {
        let artifact = UiRuntimeArtifact::from_program(&binding_program());
        let mut snapshots = BindingSnapshotSet::from_table(&artifact.tables.binding_snapshots);

        snapshots
            .apply_authorizations(&[BindingAuthorization::read_write("editor.inspector.write")]);
        let mut report = snapshots.apply_host_data(&[HostDataSnapshot::new(
            "binding.inspector.name.host",
            UiSchemaValue::string("Lamp"),
            3,
        )]);
        report = report
            .with_collection_diffs(snapshots.collection_diffs(&artifact.tables.collection_diffs));

        let snapshot = snapshots
            .snapshot(&BindingId::new("binding.inspector.name"))
            .expect("binding snapshot should exist");

        assert!(snapshot.dirty);
        assert!(snapshot.is_authorized());
        assert_eq!(snapshot.value, Some(UiSchemaValue::string("Lamp")));
        assert_eq!(
            report.dirty_bindings,
            [BindingId::new("binding.inspector.name")]
        );
        assert_eq!(
            report.affected_endpoints,
            [BindingEndpointAddress::UiState {
                requirement_id: "state.inspector.name".to_owned(),
                endpoint_id: "binding.inspector.name.state".to_owned(),
            }]
        );
        assert_eq!(
            report.collection_diffs,
            [BindingCollectionDiff {
                binding_id: BindingId::new("binding.inspector.name"),
                diff: CollectionDiff::Replace { index: 0 },
            }]
        );
        assert!(report.passed());

        let propagation = DirtyPropagation::from_report(report);
        assert!(propagation.passed());
        assert!(propagation.has_dirty_binding(&BindingId::new("binding.inspector.name")));
    }

    #[test]
    fn binding_contract_requires_host_data_capability_and_revision_policy() {
        let contract = HostDataContract::new(
            "binding.inspector.name.host",
            BindingValueSchema {
                schema_id: "ui.inspector.name".to_owned(),
                schema_version: 1,
            },
        )
        .with_capability("editor.inspector.write");

        assert!(contract.accepts_revision(2, 3));
        assert!(!contract.accepts_revision(3, 3));
        assert!(contract.authorizes(&[BindingAuthorization::read_write("editor.inspector.write")]));
        assert!(!contract.authorizes(&[]));
    }

    fn binding_program() -> UiProgram {
        let control_id = ControlNodeId::new("control.inspector.name");
        let state_id = StateRequirementId::new("state.inspector.name");
        let mut program = UiProgram::new(
            UiProgramId::new("program.binding"),
            UiProgramVersion::new(1),
        );
        program.graphs.control.add_node(ControlGraphNode::new(
            control_id.clone(),
            ControlPackageRef::new("editor.inspector"),
            ControlKindRef::new("editor.inspector.field"),
        ));
        program
            .graphs
            .state
            .requirements
            .push(StateRequirement::new(
                state_id.clone(),
                control_id,
                StateRequirementLifecycle::HostFed,
                UiSchemaRef::new("ui.inspector.name", 1),
            ));
        program.graphs.binding.bindings.push(
            BindingEdge::new(
                BindingEdgeId::new("binding.inspector.name"),
                BindingEndpoint::HostData {
                    endpoint_id: BindingEndpointId::new("binding.inspector.name.host"),
                },
                BindingEndpoint::UiState {
                    requirement_id: state_id,
                    endpoint_id: BindingEndpointId::new("binding.inspector.name.state"),
                },
                UiSchemaRef::new("ui.inspector.name", 1),
            )
            .with_capability(RouteCapability::new("editor.inspector.write")),
        );
        program
    }
}
