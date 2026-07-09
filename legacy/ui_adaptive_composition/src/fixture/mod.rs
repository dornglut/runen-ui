//! Deterministic adaptive benchmark and conformance fixtures.

use ui_composition::{
    CompositionDefinitionId, CompositionDefinitionV1, CompositionRootDefinition, CompositionRootId,
    CompositionState, ContentInstanceRef, ContentOwnerId, ContentProfileId, DefinitionRevision,
    MountedContentRef, MountedUnitDefinition, MountedUnitId, NamespacedReferenceError,
    PresentationTargetDefinition, PresentationTargetId, RegionDefinition, RegionId, RegionKind,
    SplitAxis, SplitFraction, TargetProfileId, UnavailableContentPolicy,
};
use ui_math::UiRect;

use crate::{
    AdaptiveProjectionPolicy, AdaptiveRegionPolicy, AdaptiveTargetConstraints, CompactBehavior,
};

pub struct AdaptiveBenchmarkFixture {
    pub state: CompositionState,
    pub policy: AdaptiveProjectionPolicy,
}

pub fn normal_benchmark_fixture() -> AdaptiveBenchmarkFixture {
    benchmark_fixture(4, 16).expect("normal benchmark references are compiled-in valid")
}

pub fn large_benchmark_fixture() -> AdaptiveBenchmarkFixture {
    benchmark_fixture(16, 64).expect("large benchmark references are compiled-in valid")
}

fn benchmark_fixture(
    target_count: usize,
    units_per_target: usize,
) -> Result<AdaptiveBenchmarkFixture, NamespacedReferenceError> {
    let mut targets = Vec::new();
    let mut roots = Vec::new();
    let mut regions = Vec::new();
    let mut units = Vec::new();
    let mut target_constraints = Vec::new();
    let mut region_policies = Vec::new();
    let mut next_region = 1_u64;
    let mut next_unit = 1_u64;
    for target_index in 0..target_count {
        let target_id = PresentationTargetId::new((target_index + 1) as u64);
        targets.push(PresentationTargetDefinition::new(
            target_id,
            TargetProfileId::new(format!("fixture.target_{}", target_index + 1))?,
        ));
        target_constraints.push(AdaptiveTargetConstraints {
            target: target_id,
            bounds: UiRect::new(0.0, 0.0, 1440.0, 1024.0),
            text_scale: 1.0,
            minimum_touch_target: 44.0,
            high_contrast: target_index % 2 == 0,
            reduced_motion: target_index % 2 == 1,
        });
        let mut level = Vec::new();
        for _ in 0..units_per_target {
            let unit_id = MountedUnitId::new(next_unit);
            next_unit += 1;
            units.push(MountedUnitDefinition::new(
                unit_id,
                MountedContentRef::new(
                    ContentOwnerId::new("fixture.adaptive")?,
                    ContentProfileId::new("fixture.adaptive_panel")?,
                    ContentInstanceRef::new(format!("fixture.panel_{}", unit_id.raw()))?,
                ),
                [],
                UnavailableContentPolicy::ShowFallback,
            ));
            let region_id = RegionId::new(next_region);
            next_region += 1;
            regions.push(RegionDefinition::new(
                region_id,
                None,
                RegionKind::Stack {
                    ordered_units: vec![unit_id],
                    active_unit: unit_id,
                },
            ));
            region_policies.push(AdaptiveRegionPolicy {
                region: region_id,
                minimum_width: 120.0,
                minimum_height: 80.0,
                priority: unit_id.raw() as u16,
                compact_behavior: if unit_id.raw().is_multiple_of(3) {
                    CompactBehavior::Drawer
                } else {
                    CompactBehavior::Overflow
                },
            });
            level.push(region_id);
        }
        let mut depth = 0;
        while level.len() > 1 {
            let mut next = Vec::new();
            for pair in level.chunks_exact(2) {
                let region_id = RegionId::new(next_region);
                next_region += 1;
                regions.push(RegionDefinition::new(
                    region_id,
                    None,
                    RegionKind::Split {
                        axis: if depth % 2 == 0 {
                            SplitAxis::Horizontal
                        } else {
                            SplitAxis::Vertical
                        },
                        fraction: SplitFraction::try_new(5_000).unwrap(),
                        first: pair[0],
                        second: pair[1],
                    },
                ));
                next.push(region_id);
            }
            level = next;
            depth += 1;
        }
        let wrapper = RegionId::new(next_region);
        next_region += 1;
        regions.push(RegionDefinition::new(
            wrapper,
            None,
            RegionKind::Overlay {
                base: level[0],
                ordered_overlays: Vec::new(),
            },
        ));
        roots.push(CompositionRootDefinition::new(
            CompositionRootId::new((target_index + 1) as u64),
            target_id,
            wrapper,
            true,
        ));
    }
    let definition = CompositionDefinitionV1::new(
        CompositionDefinitionId::new(9_000 + target_count as u64),
        DefinitionRevision::new(1),
        targets,
        roots,
        regions,
        units,
    );
    let state = CompositionState::form(definition).expect("benchmark topology must form");
    let policy = AdaptiveProjectionPolicy::new(target_constraints, region_policies)
        .expect("benchmark policy must form");
    Ok(AdaptiveBenchmarkFixture { state, policy })
}

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use ui_composition::MountedUnitId;
    use ui_math::UiPoint;

    use crate::{AdaptiveProjectionState, DragSession};

    use super::*;

    #[test]
    fn adaptive_normal_and_large_fixtures_have_exact_contract_sizes() {
        let normal = normal_benchmark_fixture();
        assert_eq!(normal.state.definition().regions().len(), 128);
        assert_eq!(normal.state.definition().mounted_units().len(), 64);
        assert_eq!(normal.state.definition().targets().len(), 4);
        let large = large_benchmark_fixture();
        assert_eq!(large.state.definition().regions().len(), 2_048);
        assert_eq!(large.state.definition().mounted_units().len(), 1_024);
        assert_eq!(large.state.definition().targets().len(), 16);
    }

    #[test]
    fn adaptive_drag_updates_share_graph_and_report_zero_full_clones() {
        let fixture = normal_benchmark_fixture();
        let projection = Arc::new(
            AdaptiveProjectionState::derive(fixture.state.snapshot(), &fixture.policy).unwrap(),
        );
        let mut session = DragSession::begin(projection, MountedUnitId::new(1));
        assert!(session.update_pointer(UiPoint::new(10.0, 10.0)).is_some());
        assert_eq!(session.metrics().full_graph_clones, 0);
        assert_eq!(session.metrics().changed_regions, 1);
        assert_eq!(session.metrics().bounded_allocation_units, 1);
        assert_eq!(session.shared_region_count(), 128);
    }
}
