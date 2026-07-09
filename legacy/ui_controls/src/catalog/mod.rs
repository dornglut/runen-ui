//! File: domain/ui/ui_controls/src/catalog/mod.rs
//! Crate: ui_controls

use serde::{Deserialize, Serialize};

use crate::migration::ControlDeprecationStatus;
use crate::package::ControlPackageDescriptor;
use crate::package::descriptor::ControlKindDescriptor;
use crate::package::story_proof::{ControlStoryProofSummary, ControlStoryProofVerdict};

pub mod entry;
pub mod index;
pub mod inspection;
pub mod layout;
pub mod query;
pub mod render_projection;

pub use entry::*;
pub use index::*;
pub use inspection::*;
pub use layout::*;
pub use query::*;
pub use render_projection::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ControlCatalogDeprecationStatus {
    Active,
    Deprecated,
    Removed,
}

impl ControlCatalogDeprecationStatus {
    pub fn from_status(status: &ControlDeprecationStatus) -> Self {
        match status {
            ControlDeprecationStatus::Active => Self::Active,
            ControlDeprecationStatus::Deprecated { .. } => Self::Deprecated,
            ControlDeprecationStatus::Removed { .. } => Self::Removed,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlDiagnosticBadge {
    pub diagnostic_id: String,
    pub severity: String,
    pub message_template: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlCompatibilitySummary {
    pub supports_story_proof: bool,
    pub supports_gallery_inspection: bool,
    pub supports_workbench_consumption: bool,
    pub supports_designer_consumption: bool,
    pub supports_runtime_mount: bool,
}

impl ControlCompatibilitySummary {
    pub fn from_control_kind(kind: &ControlKindDescriptor) -> Self {
        Self {
            supports_story_proof: kind.compatibility.supports_story_proof,
            supports_gallery_inspection: kind.compatibility.supports_gallery_inspection,
            supports_workbench_consumption: kind.compatibility.supports_workbench_consumption,
            supports_designer_consumption: kind.compatibility.supports_designer_consumption,
            supports_runtime_mount: kind.compatibility.supports_runtime_mount,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlStoryProofBadge {
    pub verdict: ControlStoryProofVerdict,
    #[serde(default)]
    pub first_unsatisfied_story_id: Option<String>,
    #[serde(default)]
    pub first_blocking_message: Option<String>,
}

impl ControlStoryProofBadge {
    pub fn from_summary(summary: &ControlStoryProofSummary) -> Self {
        Self {
            verdict: summary.verdict,
            first_unsatisfied_story_id: summary
                .first_unsatisfied_requirement
                .as_ref()
                .map(|requirement| requirement.story_id.as_str().to_owned()),
            first_blocking_message: summary
                .first_blocking_diagnostic
                .as_ref()
                .map(|diagnostic| diagnostic.message.to_owned()),
        }
    }
}

pub(crate) fn diagnostic_badges(
    package: &ControlPackageDescriptor,
    kind: &ControlKindDescriptor,
) -> Vec<ControlDiagnosticBadge> {
    kind.diagnostic_ids
        .iter()
        .map(|diagnostic_id| {
            let descriptor = package
                .diagnostics
                .iter()
                .find(|descriptor| descriptor.diagnostic_id.as_str() == diagnostic_id.as_str());
            ControlDiagnosticBadge {
                diagnostic_id: diagnostic_id.as_str().to_owned(),
                severity: descriptor
                    .map(|descriptor| format!("{:?}", descriptor.severity))
                    .unwrap_or_else(|| "Unknown".to_owned()),
                message_template: descriptor
                    .map(|descriptor| descriptor.message_template.to_owned())
                    .unwrap_or_else(|| "diagnostic descriptor is not attached".to_owned()),
            }
        })
        .collect()
}
