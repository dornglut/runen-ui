use serde::{Deserialize, Serialize};

use crate::package::ids::ControlKindId;

use super::{
    ControlSurface2DAccessibilitySupport, ControlSurface2DBudgetEvidenceKind,
    ControlSurface2DInputMode, ControlSurface2DInteractionSupport, ControlSurface2DLayerKind,
};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlSurface2DDescriptor {
    pub control_kind_id: ControlKindId,
    pub input_modes: Vec<ControlSurface2DInputMode>,
    pub layer_kinds: Vec<ControlSurface2DLayerKind>,
    pub budget_evidence: Vec<ControlSurface2DBudgetEvidenceKind>,
    pub accessibility: ControlSurface2DAccessibilitySupport,
    pub interaction: ControlSurface2DInteractionSupport,
    #[serde(default = "default_true")]
    pub proof_required: bool,
    #[serde(default)]
    pub renderer_backend_required: bool,
    #[serde(default)]
    pub executes_host_commands: bool,
    #[serde(default)]
    pub mutates_product_state: bool,
    #[serde(default)]
    pub graph_or_timeline_semantics: bool,
}

impl ControlSurface2DDescriptor {
    pub fn new(control_kind_id: ControlKindId) -> Self {
        Self {
            control_kind_id,
            input_modes: vec![
                ControlSurface2DInputMode::KeyboardPan,
                ControlSurface2DInputMode::KeyboardZoom,
                ControlSurface2DInputMode::KeyboardFitContent,
                ControlSurface2DInputMode::PointerCapture,
                ControlSurface2DInputMode::WheelScroll,
                ControlSurface2DInputMode::TrackpadPinchStatus,
                ControlSurface2DInputMode::TouchPanZoomStatus,
                ControlSurface2DInputMode::ControllerNavigationStatus,
            ],
            layer_kinds: vec![
                ControlSurface2DLayerKind::Background,
                ControlSurface2DLayerKind::Grid,
                ControlSurface2DLayerKind::DiagnosticOverlay,
                ControlSurface2DLayerKind::SelectionBox,
            ],
            budget_evidence: vec![
                ControlSurface2DBudgetEvidenceKind::TransformProjection,
                ControlSurface2DBudgetEvidenceKind::PanZoomUpdate,
                ControlSurface2DBudgetEvidenceKind::HoverCoordinateUpdate,
                ControlSurface2DBudgetEvidenceKind::SelectionRectangleUpdate,
                ControlSurface2DBudgetEvidenceKind::FitContentCalculation,
                ControlSurface2DBudgetEvidenceKind::LargeContentBoundsProjection,
                ControlSurface2DBudgetEvidenceKind::RuntimeReportGeneration,
                ControlSurface2DBudgetEvidenceKind::StaticMountReportGeneration,
                ControlSurface2DBudgetEvidenceKind::PrimitiveCount,
            ],
            accessibility: ControlSurface2DAccessibilitySupport::complete(),
            interaction: ControlSurface2DInteractionSupport::complete(),
            proof_required: true,
            renderer_backend_required: false,
            executes_host_commands: false,
            mutates_product_state: false,
            graph_or_timeline_semantics: false,
        }
    }

    pub fn summary(&self) -> ControlSurface2DSupportSummary {
        ControlSurface2DSupportSummary::from_descriptor(self)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlSurface2DSupportSummary {
    pub control_kind_id: ControlKindId,
    pub surface2d_supported: bool,
    pub input_modes: Vec<String>,
    pub layer_kinds: Vec<String>,
    pub budget_evidence: Vec<String>,
    pub accessibility_complete: bool,
    pub interaction_complete: bool,
    pub renderer_backend_required: bool,
    pub executes_host_commands: bool,
    pub mutates_product_state: bool,
    pub graph_or_timeline_semantics: bool,
}

impl ControlSurface2DSupportSummary {
    pub fn from_descriptor(descriptor: &ControlSurface2DDescriptor) -> Self {
        let mut input_modes = descriptor
            .input_modes
            .iter()
            .map(|mode| mode.as_str().to_owned())
            .collect::<Vec<_>>();
        input_modes.sort();
        input_modes.dedup();
        let mut layer_kinds = descriptor
            .layer_kinds
            .iter()
            .map(|kind| kind.as_str().to_owned())
            .collect::<Vec<_>>();
        layer_kinds.sort();
        layer_kinds.dedup();
        let mut budget_evidence = descriptor
            .budget_evidence
            .iter()
            .map(|kind| kind.as_str().to_owned())
            .collect::<Vec<_>>();
        budget_evidence.sort();
        budget_evidence.dedup();
        Self {
            control_kind_id: descriptor.control_kind_id.clone(),
            surface2d_supported: descriptor.proof_required,
            input_modes,
            layer_kinds,
            budget_evidence,
            accessibility_complete: descriptor.accessibility.is_complete(),
            interaction_complete: descriptor.interaction.is_complete(),
            renderer_backend_required: descriptor.renderer_backend_required,
            executes_host_commands: descriptor.executes_host_commands,
            mutates_product_state: descriptor.mutates_product_state,
            graph_or_timeline_semantics: descriptor.graph_or_timeline_semantics,
        }
    }

    pub fn inspection_facts(&self) -> Vec<ControlSurface2DInspectionFact> {
        vec![
            ControlSurface2DInspectionFact::new(
                "surface2d.supported",
                bool_string(self.surface2d_supported),
            ),
            ControlSurface2DInspectionFact::new(
                "surface2d.input_modes",
                self.input_modes.join(","),
            ),
            ControlSurface2DInspectionFact::new("surface2d.layers", self.layer_kinds.join(",")),
            ControlSurface2DInspectionFact::new(
                "surface2d.budget_evidence",
                self.budget_evidence.join(","),
            ),
            ControlSurface2DInspectionFact::new(
                "surface2d.accessibility_complete",
                bool_string(self.accessibility_complete),
            ),
            ControlSurface2DInspectionFact::new(
                "surface2d.interaction_complete",
                bool_string(self.interaction_complete),
            ),
            ControlSurface2DInspectionFact::new(
                "surface2d.renderer_backend_required",
                bool_string(self.renderer_backend_required),
            ),
            ControlSurface2DInspectionFact::new(
                "surface2d.executes_host_commands",
                bool_string(self.executes_host_commands),
            ),
            ControlSurface2DInspectionFact::new(
                "surface2d.mutates_product_state",
                bool_string(self.mutates_product_state),
            ),
            ControlSurface2DInspectionFact::new(
                "surface2d.graph_or_timeline_semantics",
                bool_string(self.graph_or_timeline_semantics),
            ),
        ]
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlSurface2DInspectionFact {
    pub key: String,
    pub value: String,
}

impl ControlSurface2DInspectionFact {
    pub fn new(key: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            value: value.into(),
        }
    }
}

fn default_true() -> bool {
    true
}

fn bool_string(value: bool) -> &'static str {
    if value { "true" } else { "false" }
}
