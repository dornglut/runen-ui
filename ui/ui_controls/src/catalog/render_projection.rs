//! File: domain/ui/ui_controls/src/catalog/render_projection.rs
//! Crate: ui_controls
//! Purpose: Read-only catalog projection for control render summaries.

use crate::package::render::ControlRenderCapabilitySummary;

use super::{ControlInspectionDescriptor, ControlInspectionFact, ControlInspectionSection};

pub trait ControlRenderInspectionExt {
    fn with_control_render_summary(self, summary: &ControlRenderCapabilitySummary) -> Self;
}

impl ControlRenderInspectionExt for ControlInspectionDescriptor {
    fn with_control_render_summary(mut self, summary: &ControlRenderCapabilitySummary) -> Self {
        for fact in summary.inspection_facts() {
            self.facts.push(ControlInspectionFact {
                section: ControlInspectionSection::Metadata,
                key: ["render", fact.key.as_str()].join("."),
                value: fact.value,
            });
        }
        self
    }
}
