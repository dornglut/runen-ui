use serde::{Deserialize, Serialize};
use ui_program::{RouteCapability, RouteId, RouteSchemaVersion};

use super::ids::{
    ControlBudgetEvidenceId, ControlFixtureId, ControlPackageId, ControlRenderEvidenceId,
    ControlStoryId, ControlTargetProfileRef,
};
use crate::migration::ControlDeprecationStatus;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlPackageCategory(pub String);
impl Default for ControlPackageCategory {
    fn default() -> Self {
        Self("uncategorized".to_owned())
    }
}
impl ControlPackageCategory {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlTag(pub String);
impl ControlTag {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlCompatibilityFlags {
    pub supports_story_proof: bool,
    pub supports_gallery_inspection: bool,
    pub supports_workbench_consumption: bool,
    pub supports_designer_consumption: bool,
    pub supports_runtime_mount: bool,
}
impl ControlCompatibilityFlags {
    pub const fn descriptor_only() -> Self {
        Self {
            supports_story_proof: true,
            supports_gallery_inspection: true,
            supports_workbench_consumption: true,
            supports_designer_consumption: true,
            supports_runtime_mount: false,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlCatalogMetadata {
    pub sort_key: String,
    pub group: String,
    #[serde(default)]
    pub discoverable: bool,
}
impl ControlCatalogMetadata {
    pub fn new(sort_key: impl Into<String>, group: impl Into<String>) -> Self {
        Self {
            sort_key: sort_key.into(),
            group: group.into(),
            discoverable: true,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlFixtureDescriptor {
    pub fixture_id: ControlFixtureId,
    pub description: String,
}
impl ControlFixtureDescriptor {
    pub fn new(fixture_id: ControlFixtureId, description: impl Into<String>) -> Self {
        Self {
            fixture_id,
            description: description.into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlStoryDescriptor {
    pub story_id: ControlStoryId,
    pub description: String,
    #[serde(default)]
    pub requires_runtime_evidence: bool,
}
impl ControlStoryDescriptor {
    pub fn new(story_id: ControlStoryId, description: impl Into<String>) -> Self {
        Self {
            story_id,
            description: description.into(),
            requires_runtime_evidence: true,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlRenderEvidenceRequirement {
    pub evidence_id: ControlRenderEvidenceId,
    pub description: String,
}
impl ControlRenderEvidenceRequirement {
    pub fn new(evidence_id: ControlRenderEvidenceId, description: impl Into<String>) -> Self {
        Self {
            evidence_id,
            description: description.into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlBudgetEvidenceRequirement {
    pub evidence_id: ControlBudgetEvidenceId,
    pub description: String,
}
impl ControlBudgetEvidenceRequirement {
    pub fn new(evidence_id: ControlBudgetEvidenceId, description: impl Into<String>) -> Self {
        Self {
            evidence_id,
            description: description.into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlRequirement {
    pub requirement_id: String,
    pub description: String,
}
impl ControlRequirement {
    pub fn new(requirement_id: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            requirement_id: requirement_id.into(),
            description: description.into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlRouteRequirement {
    pub route_id: RouteId,
    pub schema_version: RouteSchemaVersion,
    #[serde(default)]
    pub capabilities: Vec<RouteCapability>,
}
impl ControlRouteRequirement {
    pub fn new(route_id: RouteId, schema_version: RouteSchemaVersion) -> Self {
        Self {
            route_id,
            schema_version,
            capabilities: Vec::new(),
        }
    }
    pub fn with_capability(mut self, capability: RouteCapability) -> Self {
        self.capabilities.push(capability);
        self
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ControlMountEligibility {
    NotEligible {
        reason: String,
    },
    RequiresEvidence {
        story_ids: Vec<ControlStoryId>,
        render_evidence_ids: Vec<ControlRenderEvidenceId>,
        budget_evidence_ids: Vec<ControlBudgetEvidenceId>,
    },
}
impl Default for ControlMountEligibility {
    fn default() -> Self {
        Self::NotEligible {
            reason: "story proof and runtime evidence are not attached yet".to_owned(),
        }
    }
}
impl ControlMountEligibility {
    pub fn not_eligible(reason: impl Into<String>) -> Self {
        Self::NotEligible {
            reason: reason.into(),
        }
    }
    pub fn requires_evidence(
        story_ids: impl IntoIterator<Item = ControlStoryId>,
        render_evidence_ids: impl IntoIterator<Item = ControlRenderEvidenceId>,
        budget_evidence_ids: impl IntoIterator<Item = ControlBudgetEvidenceId>,
    ) -> Self {
        Self::RequiresEvidence {
            story_ids: story_ids.into_iter().collect(),
            render_evidence_ids: render_evidence_ids.into_iter().collect(),
            budget_evidence_ids: budget_evidence_ids.into_iter().collect(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlCatalogEntry {
    pub package_id: ControlPackageId,
    pub target_profile: ControlTargetProfileRef,
    #[serde(default)]
    pub deprecation: ControlDeprecationStatus,
}
