//! UiEvaluator output contract.

use serde::{Deserialize, Serialize};
use ui_artifacts::UiRuntimeArtifactDiagnostic;

use crate::{
    AccessibilityEvaluationPass, BindingEvaluationPass, ControlEvaluationPass, InputEvaluationPass,
    InspectionEvaluationPass, InteractionEvaluationPass, LayoutEvaluationPass, StateEvaluationPass,
    StyleEvaluationPass, VisualEvaluationPass,
};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct UiOutput {
    pub input: InputEvaluationPass,
    pub controls: ControlEvaluationPass,
    pub layout: LayoutEvaluationPass,
    pub style: StyleEvaluationPass,
    pub state: StateEvaluationPass,
    pub binding: BindingEvaluationPass,
    pub interaction: InteractionEvaluationPass,
    pub visual: VisualEvaluationPass,
    pub accessibility: AccessibilityEvaluationPass,
    pub inspection: InspectionEvaluationPass,
    pub diagnostics: Vec<UiRuntimeArtifactDiagnostic>,
}
