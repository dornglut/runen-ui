//! Stable adaptive-composition diagnostics.

use ui_composition::{MountedUnitId, PresentationTargetId, RegionId, StateRevision};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AdaptiveDiagnosticCode {
    ConstraintInvalid,
    TargetMissing,
    RegionMissing,
    RegionCycle,
    ProjectionIncomplete,
    ProposalInvalid,
    SessionRevisionMismatch,
    PromotionDeltaInvalid,
    AccessibilityIncomplete,
}

impl AdaptiveDiagnosticCode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::ConstraintInvalid => "ui_adaptive_composition.policy.constraint_invalid",
            Self::TargetMissing => "ui_adaptive_composition.projection.target_missing",
            Self::RegionMissing => "ui_adaptive_composition.projection.region_missing",
            Self::RegionCycle => "ui_adaptive_composition.projection.region_cycle",
            Self::ProjectionIncomplete => "ui_adaptive_composition.projection.incomplete",
            Self::ProposalInvalid => "ui_adaptive_composition.proposal.invalid",
            Self::SessionRevisionMismatch => "ui_adaptive_composition.session.revision_mismatch",
            Self::PromotionDeltaInvalid => "ui_adaptive_composition.promotion.delta_invalid",
            Self::AccessibilityIncomplete => "ui_adaptive_composition.accessibility.incomplete",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AdaptiveDiagnosticSeverity {
    Info,
    Warning,
    Error,
    Fatal,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AdaptiveDiagnosticStage {
    Policy,
    Projection,
    HitTesting,
    Proposal,
    Preview,
    Session,
    Promotion,
    Accessibility,
    Fixture,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum AdaptiveDiagnosticSubject {
    Target(PresentationTargetId),
    Region(RegionId),
    MountedUnit(MountedUnitId),
    Revision(StateRevision),
    General(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AdaptiveDiagnosticRecord {
    pub code: AdaptiveDiagnosticCode,
    pub severity: AdaptiveDiagnosticSeverity,
    pub stage: AdaptiveDiagnosticStage,
    pub subject: AdaptiveDiagnosticSubject,
    pub message: String,
}

impl AdaptiveDiagnosticRecord {
    pub fn error(
        code: AdaptiveDiagnosticCode,
        stage: AdaptiveDiagnosticStage,
        subject: AdaptiveDiagnosticSubject,
        message: impl Into<String>,
    ) -> Self {
        Self {
            code,
            severity: AdaptiveDiagnosticSeverity::Error,
            stage,
            subject,
            message: message.into(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AdaptiveCompositionRejection {
    diagnostics: Vec<AdaptiveDiagnosticRecord>,
}

impl AdaptiveCompositionRejection {
    pub fn new(mut diagnostics: Vec<AdaptiveDiagnosticRecord>) -> Self {
        diagnostics.sort_by_key(|record| {
            (
                record.stage,
                record.severity,
                record.code.as_str(),
                format!("{:?}", record.subject),
            )
        });
        diagnostics.dedup();
        Self { diagnostics }
    }

    pub fn single(diagnostic: AdaptiveDiagnosticRecord) -> Self {
        Self::new(vec![diagnostic])
    }

    pub fn diagnostics(&self) -> &[AdaptiveDiagnosticRecord] {
        &self.diagnostics
    }
}

impl std::fmt::Display for AdaptiveCompositionRejection {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "adaptive composition rejected with {} diagnostic(s)",
            self.diagnostics.len()
        )
    }
}

impl std::error::Error for AdaptiveCompositionRejection {}
