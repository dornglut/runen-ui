//! Immutable region hit index built once per projection.

use std::sync::Arc;

use ui_composition::RegionId;
use ui_math::UiPoint;

use crate::{AdaptivePresentationMode, ProjectedRegion};

#[derive(Clone, Debug)]
pub struct RegionHitIndex {
    regions: Arc<[ProjectedRegion]>,
}

impl RegionHitIndex {
    pub fn new(regions: Arc<[ProjectedRegion]>) -> Self {
        Self { regions }
    }

    pub fn hit_test(&self, point: UiPoint) -> Option<RegionId> {
        self.regions
            .iter()
            .filter(|region| {
                !matches!(region.mode, AdaptivePresentationMode::Hidden)
                    && region.bounds.contains(point)
            })
            .min_by(|left, right| {
                let left_area = left.bounds.width * left.bounds.height;
                let right_area = right.bounds.width * right.bounds.height;
                left_area
                    .total_cmp(&right_area)
                    .then_with(|| left.priority.cmp(&right.priority))
                    .then_with(|| left.region.cmp(&right.region))
            })
            .map(|region| region.region)
    }

    pub fn shared_region_count(&self) -> usize {
        self.regions.len()
    }
}
