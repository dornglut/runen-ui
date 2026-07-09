//! Deterministic evaluator pass contracts.

use serde::{Deserialize, Serialize};
use ui_artifacts::{
    AccessibilityRow, BindingSnapshotRow, CollectionDiffPlanEntry, ControlTableRow, InspectionRow,
    InteractionDispatchRow, LayoutPlanRow, StyleResolutionRow, TextLayoutRequest,
    VisualOperatorRow,
};
use ui_binding::{BindingDirtyReport, BindingSnapshot};
use ui_program::UiEventPacket;
use ui_state::{UiStateBucket, UiStateKey};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct InputEvaluationPass {
    pub events: Vec<UiEventPacket>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct LayoutEvaluationPass {
    pub rows: Vec<LayoutPlanRow>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StateEvaluationPass {
    pub rows: Vec<StateEvaluationRow>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StateEvaluationRow {
    pub state_key: UiStateKey,
    pub bucket: UiStateBucket,
    pub revision: u64,
    #[serde(default)]
    pub source_map_index: Option<u32>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct BindingEvaluationPass {
    pub table_rows: Vec<BindingSnapshotRow>,
    pub snapshots: Vec<BindingSnapshot>,
    pub dirty_report: BindingDirtyReport,
    pub collection_diff_plan: Vec<CollectionDiffPlanEntry>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct InteractionEvaluationPass {
    pub rows: Vec<InteractionDispatchRow>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct VisualEvaluationPass {
    pub operators: Vec<VisualOperatorRow>,
    pub text_layout_requests: Vec<TextLayoutRequest>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccessibilityEvaluationPass {
    pub rows: Vec<AccessibilityRow>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct InspectionEvaluationPass {
    pub rows: Vec<InspectionRow>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlEvaluationPass {
    pub rows: Vec<ControlTableRow>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct StyleEvaluationPass {
    pub rows: Vec<StyleResolutionRow>,
}
