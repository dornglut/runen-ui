//! UiRuntimeArtifact evaluator orchestration.

use ui_artifacts::{UiRuntimeArtifact, UiRuntimeArtifactDiagnostic};
use ui_binding::BindingSnapshotSet;
use ui_state::UiStateModel;

use crate::state_binding::{apply_dirty_bindings_to_state, state_evaluation_row};
use crate::{
    AccessibilityEvaluationPass, BindingEvaluationPass, ControlEvaluationPass, InputEvaluationPass,
    InspectionEvaluationPass, InteractionEvaluationPass, LayoutEvaluationPass, StateEvaluationPass,
    StyleEvaluationPass, UiEvaluationContext, UiOutput, VisualEvaluationPass,
};

#[derive(Clone, Debug, Default)]
pub struct UiEvaluator;

impl UiEvaluator {
    pub fn evaluate(&self, artifact: &UiRuntimeArtifact, state: &mut UiStateModel) -> UiOutput {
        self.evaluate_with_context(artifact, state, UiEvaluationContext::default())
    }

    pub fn evaluate_with_context(
        &self,
        artifact: &UiRuntimeArtifact,
        state: &mut UiStateModel,
        context: UiEvaluationContext,
    ) -> UiOutput {
        state.ensure_requirements(
            artifact
                .tables
                .state
                .rows
                .iter()
                .map(|row| &row.requirement),
        );

        let mut binding_snapshots =
            BindingSnapshotSet::from_table(&artifact.tables.binding_snapshots);
        binding_snapshots.apply_authorizations(&context.binding_authorizations);
        let dirty_report = binding_snapshots
            .apply_host_data(&context.host_data)
            .with_collection_diffs(
                binding_snapshots.collection_diffs(&artifact.tables.collection_diffs),
            );

        let mut diagnostics = artifact.manifest.diagnostics.clone();
        for diagnostic in &dirty_report.diagnostics {
            diagnostics.push(UiRuntimeArtifactDiagnostic::warning(
                diagnostic.code.clone(),
                diagnostic.message.clone(),
            ));
        }
        apply_dirty_bindings_to_state(state, binding_snapshots.snapshots(), &mut diagnostics);

        UiOutput {
            input: InputEvaluationPass {
                events: context.host_events,
            },
            controls: ControlEvaluationPass {
                rows: artifact.tables.controls.rows.clone(),
            },
            layout: LayoutEvaluationPass {
                rows: artifact.tables.layout.rows.clone(),
            },
            style: StyleEvaluationPass {
                rows: artifact.tables.style.rows.clone(),
            },
            state: StateEvaluationPass {
                rows: artifact
                    .tables
                    .state
                    .rows
                    .iter()
                    .map(|row| state_evaluation_row(row, state))
                    .collect(),
            },
            binding: BindingEvaluationPass {
                table_rows: artifact.tables.binding_snapshots.rows.clone(),
                snapshots: binding_snapshots.snapshots().cloned().collect(),
                dirty_report,
                collection_diff_plan: artifact.tables.collection_diffs.rows.clone(),
            },
            interaction: InteractionEvaluationPass {
                rows: artifact.tables.interaction.rows.clone(),
            },
            visual: VisualEvaluationPass {
                operators: artifact.tables.visual.rows.clone(),
                text_layout_requests: artifact.tables.text_layout_requests.rows.clone(),
            },
            accessibility: AccessibilityEvaluationPass {
                rows: artifact.tables.accessibility.rows.clone(),
            },
            inspection: InspectionEvaluationPass {
                rows: artifact.tables.inspection.rows.clone(),
            },
            diagnostics,
        }
    }
}
