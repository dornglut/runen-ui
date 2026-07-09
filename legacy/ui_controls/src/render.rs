//! File: domain/ui/ui_controls/src/render.rs
//! Crate: ui_controls
//! Purpose: Control-facing render evidence bridge over ui_render_data contracts.

use serde::{Deserialize, Serialize};
use ui_render_data::{UiExpectedPrimitiveCount, UiPrimitiveFamily};

use crate::package::ids::{ControlKindId, ControlRenderEvidenceId};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlRenderDescriptor {
    pub control_kind_id: ControlKindId,
    #[serde(default)]
    pub required_primitive_families: Vec<UiPrimitiveFamily>,
    #[serde(default)]
    pub expected_primitive_counts: Vec<UiExpectedPrimitiveCount>,
    #[serde(default)]
    pub render_evidence_ids: Vec<ControlRenderEvidenceId>,
}

impl ControlRenderDescriptor {
    pub fn new(control_kind_id: ControlKindId) -> Self {
        Self {
            control_kind_id,
            required_primitive_families: Vec::new(),
            expected_primitive_counts: Vec::new(),
            render_evidence_ids: Vec::new(),
        }
    }

    pub fn with_required_primitive_family(mut self, family: UiPrimitiveFamily) -> Self {
        self.required_primitive_families.push(family);
        self.required_primitive_families.sort();
        self.required_primitive_families.dedup();
        self
    }

    pub fn with_expected_primitive_count(mut self, count: UiExpectedPrimitiveCount) -> Self {
        self.expected_primitive_counts.push(count);
        self.expected_primitive_counts
            .sort_by(|left, right| left.label().cmp(&right.label()));
        self.expected_primitive_counts.dedup_by(|left, right| {
            left.family == right.family
                && left.min_count == right.min_count
                && left.max_count == right.max_count
        });
        self
    }

    pub fn with_render_evidence(mut self, evidence_id: ControlRenderEvidenceId) -> Self {
        self.render_evidence_ids.push(evidence_id);
        self.render_evidence_ids
            .sort_by(|left, right| left.as_str().cmp(right.as_str()));
        self.render_evidence_ids
            .dedup_by(|left, right| left.as_str() == right.as_str());
        self
    }

    pub fn summary(&self) -> ControlRenderCapabilitySummary {
        ControlRenderCapabilitySummary::from_descriptor(self)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlRenderCapabilitySummary {
    pub control_kind_id: ControlKindId,
    pub required_primitive_families: Vec<String>,
    pub expected_primitive_counts: Vec<String>,
    pub render_evidence_ids: Vec<String>,
    pub has_backend_render_behavior: bool,
}

impl ControlRenderCapabilitySummary {
    pub fn from_descriptor(descriptor: &ControlRenderDescriptor) -> Self {
        let mut required_primitive_families = descriptor
            .required_primitive_families
            .iter()
            .map(|family| family.as_str().to_owned())
            .collect::<Vec<_>>();
        let mut expected_primitive_counts = descriptor
            .expected_primitive_counts
            .iter()
            .map(UiExpectedPrimitiveCount::label)
            .collect::<Vec<_>>();
        let mut render_evidence_ids = descriptor
            .render_evidence_ids
            .iter()
            .map(|evidence_id| evidence_id.as_str().to_owned())
            .collect::<Vec<_>>();

        sort_dedup(&mut required_primitive_families);
        sort_dedup(&mut expected_primitive_counts);
        sort_dedup(&mut render_evidence_ids);

        Self {
            control_kind_id: descriptor.control_kind_id.clone(),
            required_primitive_families,
            expected_primitive_counts,
            render_evidence_ids,
            has_backend_render_behavior: false,
        }
    }

    pub fn inspection_facts(&self) -> Vec<ControlRenderInspectionFact> {
        vec![
            ControlRenderInspectionFact::new(
                "required_primitive_families",
                self.required_primitive_families.join(","),
            ),
            ControlRenderInspectionFact::new(
                "expected_primitive_counts",
                self.expected_primitive_counts.join(","),
            ),
            ControlRenderInspectionFact::new(
                "render_evidence_ids",
                self.render_evidence_ids.join(","),
            ),
            ControlRenderInspectionFact::new(
                "has_backend_render_behavior",
                bool_string(self.has_backend_render_behavior),
            ),
        ]
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlRenderInspectionFact {
    pub key: String,
    pub value: String,
}

impl ControlRenderInspectionFact {
    pub fn new(key: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            value: value.into(),
        }
    }
}

fn sort_dedup(values: &mut Vec<String>) {
    values.sort();
    values.dedup();
}

fn bool_string(value: bool) -> &'static str {
    if value { "true" } else { "false" }
}
