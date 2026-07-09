//! UiProgram graph integrity validation for compilation.

use std::collections::{BTreeMap, BTreeSet};

use ui_artifacts::UiRuntimeArtifactDiagnostic;
use ui_program::{UiProgram, UiSchemaRef};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct UiGraphIntegrityReport {
    pub checks: Vec<GraphIntegrityCheck>,
}

impl UiGraphIntegrityReport {
    pub fn from_program(program: &UiProgram) -> Self {
        let control_ids = program
            .graphs
            .control
            .nodes
            .iter()
            .map(|control| control.node_id.as_str().to_owned())
            .collect::<BTreeSet<_>>();

        let mut snapshots_by_control = BTreeMap::<String, Vec<String>>::new();
        for snapshot in &program.graphs.properties.rows {
            snapshots_by_control
                .entry(snapshot.owner_control.as_str().to_owned())
                .or_default()
                .push(snapshot.snapshot_id.as_str().to_owned());
        }

        let expected_property_schemas = expected_property_schemas(program);
        let mut checks = Vec::new();

        for control in &program.graphs.control.nodes {
            let snapshots = snapshots_by_control
                .get(control.node_id.as_str())
                .cloned()
                .unwrap_or_default();
            checks.push(GraphIntegrityCheck {
                subject: GraphIntegritySubject::ControlPropertySnapshotCount {
                    control_id: control.node_id.as_str().to_owned(),
                    snapshot_ids: snapshots.clone(),
                },
                status: match snapshots.len() {
                    1 => GraphIntegrityStatus::Satisfied,
                    0 => GraphIntegrityStatus::MissingPropertySnapshot,
                    _ => GraphIntegrityStatus::DuplicatePropertySnapshots,
                },
            });
        }

        for snapshot in &program.graphs.properties.rows {
            let owner_control = snapshot.owner_control.as_str().to_owned();
            checks.push(GraphIntegrityCheck {
                subject: GraphIntegritySubject::PropertySnapshotOwner {
                    snapshot_id: snapshot.snapshot_id.as_str().to_owned(),
                    owner_control: owner_control.clone(),
                },
                status: if control_ids.contains(&owner_control) {
                    GraphIntegrityStatus::Satisfied
                } else {
                    GraphIntegrityStatus::MissingOwnerControl
                },
            });

            for expected_schema in expected_property_schemas
                .get(snapshot.owner_control.as_str())
                .into_iter()
                .flatten()
            {
                checks.push(GraphIntegrityCheck {
                    subject: GraphIntegritySubject::PropertySnapshotSchema {
                        snapshot_id: snapshot.snapshot_id.as_str().to_owned(),
                        owner_control: owner_control.clone(),
                        expected_schema: schema_display(expected_schema),
                        actual_schema: schema_display(&snapshot.schema),
                    },
                    status: if &snapshot.schema == expected_schema {
                        GraphIntegrityStatus::Satisfied
                    } else {
                        GraphIntegrityStatus::PropertySchemaMismatch
                    },
                });
            }
        }

        Self { checks }
    }

    pub fn passed(&self) -> bool {
        self.checks.iter().all(GraphIntegrityCheck::is_satisfied)
    }

    pub fn diagnostics(&self) -> Vec<UiRuntimeArtifactDiagnostic> {
        self.checks
            .iter()
            .filter_map(GraphIntegrityCheck::diagnostic)
            .collect()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GraphIntegrityCheck {
    pub subject: GraphIntegritySubject,
    pub status: GraphIntegrityStatus,
}

impl GraphIntegrityCheck {
    pub fn is_satisfied(&self) -> bool {
        self.status == GraphIntegrityStatus::Satisfied
    }

    pub fn diagnostic(&self) -> Option<UiRuntimeArtifactDiagnostic> {
        match (&self.subject, self.status) {
            (_, GraphIntegrityStatus::Satisfied) => None,
            (
                GraphIntegritySubject::ControlPropertySnapshotCount {
                    control_id,
                    snapshot_ids: _,
                },
                GraphIntegrityStatus::MissingPropertySnapshot,
            ) => Some(UiRuntimeArtifactDiagnostic::error(
                "ui.compiler.graph.missing_property_snapshot",
                format!("control {control_id} has no property snapshot"),
            )),
            (
                GraphIntegritySubject::ControlPropertySnapshotCount {
                    control_id,
                    snapshot_ids,
                },
                GraphIntegrityStatus::DuplicatePropertySnapshots,
            ) => Some(UiRuntimeArtifactDiagnostic::error(
                "ui.compiler.graph.duplicate_property_snapshots",
                format!(
                    "control {control_id} has {} property snapshots: {}",
                    snapshot_ids.len(),
                    snapshot_ids.join(", ")
                ),
            )),
            (
                GraphIntegritySubject::PropertySnapshotOwner {
                    snapshot_id,
                    owner_control,
                },
                GraphIntegrityStatus::MissingOwnerControl,
            ) => Some(UiRuntimeArtifactDiagnostic::error(
                "ui.compiler.graph.property_snapshot_missing_owner_control",
                format!(
                    "property snapshot {snapshot_id} references missing owner control {owner_control}"
                ),
            )),
            (
                GraphIntegritySubject::PropertySnapshotSchema {
                    snapshot_id,
                    owner_control,
                    expected_schema,
                    actual_schema,
                },
                GraphIntegrityStatus::PropertySchemaMismatch,
            ) => Some(UiRuntimeArtifactDiagnostic::error(
                "ui.compiler.graph.property_snapshot_schema_mismatch",
                format!(
                    "property snapshot {snapshot_id} for control {owner_control} uses schema {actual_schema}; expected {expected_schema}"
                ),
            )),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GraphIntegritySubject {
    ControlPropertySnapshotCount {
        control_id: String,
        snapshot_ids: Vec<String>,
    },
    PropertySnapshotOwner {
        snapshot_id: String,
        owner_control: String,
    },
    PropertySnapshotSchema {
        snapshot_id: String,
        owner_control: String,
        expected_schema: String,
        actual_schema: String,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GraphIntegrityStatus {
    Satisfied,
    MissingPropertySnapshot,
    DuplicatePropertySnapshots,
    MissingOwnerControl,
    PropertySchemaMismatch,
}

fn expected_property_schemas(program: &UiProgram) -> BTreeMap<String, Vec<UiSchemaRef>> {
    let mut expected = BTreeMap::<String, Vec<UiSchemaRef>>::new();
    for rule in &program.graphs.style.rules {
        insert_unique_schema(
            expected
                .entry(rule.target_control.as_str().to_owned())
                .or_default(),
            &rule.property_schema,
        );
    }
    expected
}

fn insert_unique_schema(schemas: &mut Vec<UiSchemaRef>, schema: &UiSchemaRef) {
    if !schemas.iter().any(|existing| existing == schema) {
        schemas.push(schema.clone());
    }
}

fn schema_display(schema: &UiSchemaRef) -> String {
    format!("{}@{}", schema.id.as_str(), schema.version.value())
}
