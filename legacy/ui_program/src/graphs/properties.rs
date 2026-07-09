//! Control property graph contracts.

use serde::{Deserialize, Serialize};
use ui_schema::{UiSchemaRef, UiSchemaValue};

use crate::source_map::UiProgramSourceMapAttachment;

use super::ids::{ControlNodeId, ControlPropertySnapshotId};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ControlPropertyGraph {
    pub rows: Vec<ControlPropertySnapshot>,
}

impl ControlPropertyGraph {
    pub fn add_snapshot(&mut self, snapshot: ControlPropertySnapshot) {
        self.rows.push(snapshot);
    }

    pub fn snapshot_for_control(
        &self,
        control_id: &ControlNodeId,
    ) -> Option<&ControlPropertySnapshot> {
        self.rows
            .iter()
            .find(|snapshot| &snapshot.owner_control == control_id)
    }

    pub fn snapshots_for_control(
        &self,
        control_id: &ControlNodeId,
    ) -> impl Iterator<Item = &ControlPropertySnapshot> {
        self.rows
            .iter()
            .filter(move |snapshot| &snapshot.owner_control == control_id)
    }
}

impl Eq for ControlPropertyGraph {}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ControlPropertySnapshot {
    pub snapshot_id: ControlPropertySnapshotId,
    pub owner_control: ControlNodeId,
    pub schema: UiSchemaRef,
    pub value: UiSchemaValue,
    #[serde(default)]
    pub source_map: Option<UiProgramSourceMapAttachment>,
}

impl ControlPropertySnapshot {
    pub fn new(
        snapshot_id: ControlPropertySnapshotId,
        owner_control: ControlNodeId,
        schema: UiSchemaRef,
        value: UiSchemaValue,
    ) -> Self {
        Self {
            snapshot_id,
            owner_control,
            schema,
            value,
            source_map: None,
        }
    }

    pub fn with_source_map(mut self, source_map: UiProgramSourceMapAttachment) -> Self {
        self.source_map = Some(source_map);
        self
    }

    pub fn get(&self, key: &str) -> Option<&UiSchemaValue> {
        self.value.get(key)
    }
}

impl Eq for ControlPropertySnapshot {}
