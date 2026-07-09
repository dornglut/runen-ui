//! File: domain/ui/ui_runtime/src/output/evidence.rs
//! Crate: ui_runtime
//! Purpose: Runtime-owned render output evidence generation from emitted UiFrame data.

use ui_math::UiSize;
use ui_render_data::{
    UiExpectedPrimitiveCount, UiFrame, UiPrimitiveFamily, UiRenderOutputEvidence,
    UiRenderOutputProvenance,
};
use ui_text::FontAtlasSource;
use ui_tree::{ComputedLayoutMap, UiTree};

use super::{InteractionVisualState, build_ui_frame};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UiRuntimeOutputEvidenceSource {
    pub producer_id: String,
    pub source_id: String,
}

impl UiRuntimeOutputEvidenceSource {
    pub fn new(producer_id: impl Into<String>, source_id: impl Into<String>) -> Self {
        Self {
            producer_id: producer_id.into(),
            source_id: source_id.into(),
        }
    }

    pub fn provenance(&self) -> UiRenderOutputProvenance {
        UiRenderOutputProvenance::new(self.producer_id.clone(), self.source_id.clone())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct UiRuntimeFrameOutputEvidence {
    pub frame: UiFrame,
    pub evidence: UiRenderOutputEvidence,
}

pub fn build_runtime_render_output_evidence(
    evidence_id: impl Into<String>,
    source: UiRuntimeOutputEvidenceSource,
    frame: &UiFrame,
    expected_primitive_counts: impl IntoIterator<Item = UiExpectedPrimitiveCount>,
) -> UiRenderOutputEvidence {
    UiRenderOutputEvidence::from_frame(
        evidence_id,
        source.provenance(),
        frame,
        expected_primitive_counts,
    )
}

pub fn build_ui_frame_with_render_output_evidence(
    tree: &UiTree,
    layouts: &ComputedLayoutMap,
    surface_size: UiSize,
    interaction_state: InteractionVisualState,
    atlas_source: &dyn FontAtlasSource,
    evidence_id: impl Into<String>,
    source: UiRuntimeOutputEvidenceSource,
    expected_primitive_counts: impl IntoIterator<Item = UiExpectedPrimitiveCount>,
) -> UiRuntimeFrameOutputEvidence {
    let frame = build_ui_frame(tree, layouts, surface_size, interaction_state, atlas_source);
    let evidence = build_runtime_render_output_evidence(
        evidence_id,
        source,
        &frame,
        expected_primitive_counts,
    );
    UiRuntimeFrameOutputEvidence { frame, evidence }
}

pub fn expected_panel_output() -> [UiExpectedPrimitiveCount; 3] {
    [
        UiExpectedPrimitiveCount::at_least(UiPrimitiveFamily::Rect, 1),
        UiExpectedPrimitiveCount::at_least(UiPrimitiveFamily::Border, 1),
        UiExpectedPrimitiveCount::at_least(UiPrimitiveFamily::Clip, 2),
    ]
}
