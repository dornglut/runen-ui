//! Explicit promotion candidates derived from transient adaptive projection.

use std::collections::BTreeSet;

use ui_composition::{RegionId, StateRevision};
use ui_math::UiRect;

use crate::{
    AdaptiveCompositionRejection, AdaptiveDiagnosticCode as Code,
    AdaptiveDiagnosticRecord as Record, AdaptiveDiagnosticStage as Stage,
    AdaptiveDiagnosticSubject as Subject, AdaptiveEditClassification,
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AdaptivePromotionOverride {
    pub region: RegionId,
    pub projected_bounds: UiRect,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AdaptivePromotionDelta {
    pub source_revision: StateRevision,
    pub name: String,
    pub scope: String,
    pub overrides: Vec<AdaptivePromotionOverride>,
}

impl AdaptivePromotionDelta {
    pub fn new(
        source_revision: StateRevision,
        name: impl Into<String>,
        scope: impl Into<String>,
        mut overrides: Vec<AdaptivePromotionOverride>,
    ) -> Result<Self, AdaptiveCompositionRejection> {
        let name = name.into();
        let scope = scope.into();
        overrides.sort_by_key(|value| value.region);
        let unique = overrides
            .iter()
            .map(|value| value.region)
            .collect::<BTreeSet<_>>();
        if name.trim().is_empty()
            || scope.trim().is_empty()
            || overrides.is_empty()
            || unique.len() != overrides.len()
        {
            return Err(AdaptiveCompositionRejection::single(Record::error(
                Code::PromotionDeltaInvalid,
                Stage::Promotion,
                Subject::Revision(source_revision),
                "Name and scope the promotion and provide one projected override per region.",
            )));
        }
        Ok(Self {
            source_revision,
            name,
            scope,
            overrides,
        })
    }

    pub const fn classification(&self) -> AdaptiveEditClassification {
        AdaptiveEditClassification::PromotionCandidate
    }
}
