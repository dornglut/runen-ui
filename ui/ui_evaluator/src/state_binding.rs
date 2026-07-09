//! State and binding application helpers for evaluation passes.

use ui_artifacts::{StateTableRow, UiRuntimeArtifactDiagnostic};
use ui_binding::{BindingEndpointAddress, BindingSnapshot};
use ui_state::{UiStateBucket, UiStateKey, UiStateModel};

use crate::StateEvaluationRow;

pub(crate) fn state_evaluation_row(
    row: &StateTableRow,
    state: &UiStateModel,
) -> StateEvaluationRow {
    let state_key = UiStateKey::from_requirement_id(&row.requirement.requirement_id);
    let (bucket, revision) = state
        .cell(&state_key)
        .map(|cell| (cell.bucket, cell.revision))
        .unwrap_or_else(|| (UiStateBucket::from_lifecycle(row.requirement.lifecycle), 0));
    StateEvaluationRow {
        state_key,
        bucket,
        revision,
        source_map_index: row.source_map_index,
    }
}

pub(crate) fn apply_dirty_bindings_to_state<'a>(
    state: &mut UiStateModel,
    snapshots: impl IntoIterator<Item = &'a BindingSnapshot>,
    diagnostics: &mut Vec<UiRuntimeArtifactDiagnostic>,
) {
    for snapshot in snapshots {
        if !snapshot.dirty {
            continue;
        }
        let Some(value) = snapshot.value.clone() else {
            continue;
        };
        if let BindingEndpointAddress::UiState { requirement_id, .. } = &snapshot.target {
            if let Err(error) = state.set_value(requirement_id.as_str(), value) {
                diagnostics.push(UiRuntimeArtifactDiagnostic::warning(
                    "ui.evaluator.state_binding_missing",
                    error.to_string(),
                ));
            }
        }
    }
}
