//! Host-supplied, app-neutral adaptive constraints.

use std::collections::BTreeMap;

use ui_composition::{PresentationTargetId, RegionId};
use ui_math::UiRect;

use crate::{
    AdaptiveCompositionRejection, AdaptiveDiagnosticCode as Code,
    AdaptiveDiagnosticRecord as Record, AdaptiveDiagnosticStage as Stage,
    AdaptiveDiagnosticSubject as Subject,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CompactBehavior {
    Preserve,
    Drawer,
    Overflow,
    HideWhenExplicitlyAllowed,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AdaptiveTargetConstraints {
    pub target: PresentationTargetId,
    pub bounds: UiRect,
    pub text_scale: f32,
    pub minimum_touch_target: f32,
    pub high_contrast: bool,
    pub reduced_motion: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AdaptiveRegionPolicy {
    pub region: RegionId,
    pub minimum_width: f32,
    pub minimum_height: f32,
    pub priority: u16,
    pub compact_behavior: CompactBehavior,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct AdaptiveProjectionPolicy {
    targets: BTreeMap<PresentationTargetId, AdaptiveTargetConstraints>,
    regions: BTreeMap<RegionId, AdaptiveRegionPolicy>,
}

impl AdaptiveProjectionPolicy {
    pub fn new(
        targets: impl IntoIterator<Item = AdaptiveTargetConstraints>,
        regions: impl IntoIterator<Item = AdaptiveRegionPolicy>,
    ) -> Result<Self, AdaptiveCompositionRejection> {
        let mut result = Self::default();
        for target in targets {
            if !valid_rect(target.bounds)
                || !target.text_scale.is_finite()
                || target.text_scale <= 0.0
                || !target.minimum_touch_target.is_finite()
                || target.minimum_touch_target <= 0.0
                || result.targets.insert(target.target, target).is_some()
            {
                return Err(invalid(
                    Subject::Target(target.target),
                    "Provide one finite target constraint with positive text scale and touch size.",
                ));
            }
        }
        for region in regions {
            if !region.minimum_width.is_finite()
                || !region.minimum_height.is_finite()
                || region.minimum_width < 0.0
                || region.minimum_height < 0.0
                || result.regions.insert(region.region, region).is_some()
            {
                return Err(invalid(
                    Subject::Region(region.region),
                    "Provide one finite non-negative adaptive policy per region.",
                ));
            }
        }
        Ok(result)
    }

    pub fn target(&self, id: PresentationTargetId) -> Option<AdaptiveTargetConstraints> {
        self.targets.get(&id).copied()
    }

    pub fn region(&self, id: RegionId) -> AdaptiveRegionPolicy {
        self.regions
            .get(&id)
            .copied()
            .unwrap_or(AdaptiveRegionPolicy {
                region: id,
                minimum_width: 0.0,
                minimum_height: 0.0,
                priority: u16::MAX,
                compact_behavior: CompactBehavior::Preserve,
            })
    }
}

fn valid_rect(rect: UiRect) -> bool {
    rect.x.is_finite()
        && rect.y.is_finite()
        && rect.width.is_finite()
        && rect.height.is_finite()
        && rect.width >= 0.0
        && rect.height >= 0.0
}

fn invalid(subject: Subject, message: &'static str) -> AdaptiveCompositionRejection {
    AdaptiveCompositionRejection::single(Record::error(
        Code::ConstraintInvalid,
        Stage::Policy,
        subject,
        message,
    ))
}
