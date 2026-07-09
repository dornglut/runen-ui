//! File: domain/ui/ui_controls/src/catalog/layout.rs
//! Crate: ui_controls

use crate::layout::ControlLayoutCapabilitySummary;

use super::{ControlInspectionDescriptor, ControlInspectionFact, ControlInspectionSection};

pub trait ControlLayoutInspectionExt {
    fn with_control_layout_summary(self, summary: &ControlLayoutCapabilitySummary) -> Self;
}

impl ControlLayoutInspectionExt for ControlInspectionDescriptor {
    fn with_control_layout_summary(mut self, summary: &ControlLayoutCapabilitySummary) -> Self {
        for fact in summary.inspection_facts() {
            self.facts.push(ControlInspectionFact {
                section: ControlInspectionSection::Metadata,
                key: format!("layout.{}", fact.key),
                value: fact.value,
            });
        }
        self
    }
}
