//! File: domain/ui/ui_controls/src/story_proof/mod.rs
//! Crate: ui_controls

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

use super::descriptor::ControlPackageDescriptor;
use super::ids::{ControlKindId, ControlStoryId};
use super::validation::{
    ControlPackageValidationDiagnostic, ControlPackageValidationReason,
    ControlPackageValidationReport,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ControlStoryProofCategory {
    Normal,
    Edge,
    Failure,
    Accessibility,
    Interaction,
    Layout,
    Text,
    Render,
    Budget,
    MountReadiness,
}

impl ControlStoryProofCategory {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::Edge => "edge",
            Self::Failure => "failure",
            Self::Accessibility => "accessibility",
            Self::Interaction => "interaction",
            Self::Layout => "layout",
            Self::Text => "text",
            Self::Render => "render",
            Self::Budget => "budget",
            Self::MountReadiness => "mount-readiness",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ControlStoryProofExpectedOutcome {
    Pass,
    ExpectedFailure,
}

impl ControlStoryProofExpectedOutcome {
    pub const fn is_expected_failure(self) -> bool {
        matches!(self, Self::ExpectedFailure)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum ControlStoryProofProfile {
    #[default]
    DescriptorOnly,
    MinimumMaturity,
    MountReadiness,
}

impl ControlStoryProofProfile {
    pub const fn required_categories(self) -> &'static [ControlStoryProofCategory] {
        match self {
            Self::DescriptorOnly => &[],
            Self::MinimumMaturity => &[
                ControlStoryProofCategory::Normal,
                ControlStoryProofCategory::Failure,
                ControlStoryProofCategory::Accessibility,
                ControlStoryProofCategory::Render,
                ControlStoryProofCategory::Budget,
            ],
            Self::MountReadiness => &[
                ControlStoryProofCategory::Normal,
                ControlStoryProofCategory::Edge,
                ControlStoryProofCategory::Failure,
                ControlStoryProofCategory::Accessibility,
                ControlStoryProofCategory::Interaction,
                ControlStoryProofCategory::Layout,
                ControlStoryProofCategory::Text,
                ControlStoryProofCategory::Render,
                ControlStoryProofCategory::Budget,
                ControlStoryProofCategory::MountReadiness,
            ],
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ControlStoryProofRequirement {
    pub story_id: ControlStoryId,
    pub category: ControlStoryProofCategory,
    pub expected_outcome: ControlStoryProofExpectedOutcome,
    #[serde(default = "default_required")]
    pub required: bool,
    #[serde(default)]
    pub notes: String,
}

impl ControlStoryProofRequirement {
    pub fn new(story_id: ControlStoryId, category: ControlStoryProofCategory) -> Self {
        Self {
            story_id,
            category,
            expected_outcome: ControlStoryProofExpectedOutcome::Pass,
            required: true,
            notes: String::new(),
        }
    }

    pub fn expected_failure(story_id: ControlStoryId) -> Self {
        Self::new(story_id, ControlStoryProofCategory::Failure)
            .with_expected_outcome(ControlStoryProofExpectedOutcome::ExpectedFailure)
    }

    pub fn with_expected_outcome(
        mut self,
        expected_outcome: ControlStoryProofExpectedOutcome,
    ) -> Self {
        self.expected_outcome = expected_outcome;
        self
    }

    pub fn optional(mut self) -> Self {
        self.required = false;
        self
    }

    pub fn with_notes(mut self, notes: impl Into<String>) -> Self {
        self.notes = notes.into();
        self
    }

    pub fn requirement_key(&self) -> String {
        format!("{}::{}", self.story_id.as_str(), self.category.as_str())
    }
}

fn default_required() -> bool {
    true
}

pub type ControlStoryMatrixEntry = ControlStoryProofRequirement;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlStoryMatrixDescriptor {
    pub control_kind_id: ControlKindId,
    pub profile: ControlStoryProofProfile,
    #[serde(default)]
    pub requirements: Vec<ControlStoryProofRequirement>,
}

impl ControlStoryMatrixDescriptor {
    pub fn new(control_kind_id: ControlKindId, profile: ControlStoryProofProfile) -> Self {
        Self {
            control_kind_id,
            profile,
            requirements: Vec::new(),
        }
    }

    pub fn with_requirement(mut self, requirement: ControlStoryProofRequirement) -> Self {
        self.requirements.push(requirement);
        self
    }

    pub fn validate_against_package(
        &self,
        package: &ControlPackageDescriptor,
    ) -> ControlPackageValidationReport {
        let mut report = ControlPackageValidationReport::new();
        let story_ids = package
            .stories
            .iter()
            .map(|story| story.story_id.as_str().to_owned())
            .collect::<BTreeSet<_>>();

        if package.control_kind(&self.control_kind_id).is_none() {
            report.push(ControlPackageValidationDiagnostic::kind(
                self.control_kind_id.clone(),
                ControlPackageValidationReason::UnresolvedReference,
                format!(
                    "control kind {} is not present in package {}",
                    self.control_kind_id.as_str(),
                    package.package_id.as_str()
                ),
            ));
        }

        let mut requirement_keys = BTreeSet::new();
        for requirement in &self.requirements {
            if !requirement_keys.insert(requirement.requirement_key()) {
                report.push(ControlPackageValidationDiagnostic::kind(
                    self.control_kind_id.clone(),
                    ControlPackageValidationReason::DuplicateStoryId,
                    format!(
                        "duplicate story proof requirement {} for category {}",
                        requirement.story_id.as_str(),
                        requirement.category.as_str()
                    ),
                ));
            }

            if !story_ids.contains(requirement.story_id.as_str()) {
                report.push(ControlPackageValidationDiagnostic::kind(
                    self.control_kind_id.clone(),
                    ControlPackageValidationReason::MissingStory,
                    format!(
                        "story proof requirement {} is unresolved",
                        requirement.story_id.as_str()
                    ),
                ));
            }
        }

        for required_category in self.profile.required_categories() {
            if !self.requirements.iter().any(|requirement| {
                requirement.required && requirement.category == *required_category
            }) {
                report.push(ControlPackageValidationDiagnostic::kind(
                    self.control_kind_id.clone(),
                    ControlPackageValidationReason::UnresolvedReference,
                    format!(
                        "story proof profile {:?} requires {} proof",
                        self.profile,
                        required_category.as_str()
                    ),
                ));
            }
        }

        report
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum ControlStoryProofVerdict {
    Satisfied,
    Unsatisfied,
    NotEvaluated,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlStoryProofDiagnostic {
    pub control_kind_id: ControlKindId,
    pub story_id: Option<ControlStoryId>,
    pub category: Option<ControlStoryProofCategory>,
    pub message: String,
}

impl ControlStoryProofDiagnostic {
    pub fn new(
        control_kind_id: ControlKindId,
        story_id: Option<ControlStoryId>,
        category: Option<ControlStoryProofCategory>,
        message: impl Into<String>,
    ) -> Self {
        Self {
            control_kind_id,
            story_id,
            category,
            message: message.into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlStoryProofSummary {
    pub control_kind_id: ControlKindId,
    pub verdict: ControlStoryProofVerdict,
    pub total_requirements: usize,
    pub satisfied_requirements: usize,
    pub first_unsatisfied_requirement: Option<ControlStoryProofRequirement>,
    pub first_blocking_diagnostic: Option<ControlStoryProofDiagnostic>,
}

impl ControlStoryProofSummary {
    pub fn not_evaluated(
        control_kind_id: ControlKindId,
        requirements: &[ControlStoryProofRequirement],
    ) -> Self {
        Self {
            control_kind_id,
            verdict: ControlStoryProofVerdict::NotEvaluated,
            total_requirements: requirements.len(),
            satisfied_requirements: 0,
            first_unsatisfied_requirement: requirements
                .iter()
                .find(|requirement| requirement.required)
                .cloned(),
            first_blocking_diagnostic: None,
        }
    }

    pub fn from_satisfied_story_ids(
        control_kind_id: ControlKindId,
        requirements: &[ControlStoryProofRequirement],
        satisfied_story_ids: impl IntoIterator<Item = ControlStoryId>,
    ) -> Self {
        let satisfied_story_ids = satisfied_story_ids
            .into_iter()
            .map(|story_id| story_id.as_str().to_owned())
            .collect::<BTreeSet<_>>();
        let mut satisfied_requirements = 0;
        let mut first_unsatisfied_requirement = None;

        for requirement in requirements {
            let satisfied = satisfied_story_ids.contains(requirement.story_id.as_str());
            if satisfied {
                satisfied_requirements += 1;
            } else if requirement.required && first_unsatisfied_requirement.is_none() {
                first_unsatisfied_requirement = Some(requirement.clone());
            }
        }

        let verdict = if first_unsatisfied_requirement.is_none() {
            ControlStoryProofVerdict::Satisfied
        } else {
            ControlStoryProofVerdict::Unsatisfied
        };

        Self {
            control_kind_id,
            verdict,
            total_requirements: requirements.len(),
            satisfied_requirements,
            first_unsatisfied_requirement,
            first_blocking_diagnostic: None,
        }
    }

    pub fn with_first_blocking_diagnostic(
        mut self,
        diagnostic: ControlStoryProofDiagnostic,
    ) -> Self {
        self.verdict = ControlStoryProofVerdict::Unsatisfied;
        self.first_blocking_diagnostic = Some(diagnostic);
        self
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ControlStoryProofEnvelope {
    pub matrix: ControlStoryMatrixDescriptor,
    pub summary: ControlStoryProofSummary,
}

impl ControlStoryProofEnvelope {
    pub fn not_evaluated(matrix: ControlStoryMatrixDescriptor) -> Self {
        let summary = ControlStoryProofSummary::not_evaluated(
            matrix.control_kind_id.clone(),
            &matrix.requirements,
        );
        Self { matrix, summary }
    }

    pub fn from_satisfied_story_ids(
        matrix: ControlStoryMatrixDescriptor,
        satisfied_story_ids: impl IntoIterator<Item = ControlStoryId>,
    ) -> Self {
        let summary = ControlStoryProofSummary::from_satisfied_story_ids(
            matrix.control_kind_id.clone(),
            &matrix.requirements,
            satisfied_story_ids,
        );
        Self { matrix, summary }
    }
}
