//! Product-free adaptive composition conformance fixtures.

use ui_adaptive_composition::{
    AdaptiveProjectionPolicy, AdaptiveProjectionState, AdaptiveRegionPolicy,
    AdaptiveTargetConstraints, CompactBehavior,
};
use ui_composition::CompositionFixture;
use ui_input::{SemanticDirection, SemanticInputSource, UiSemanticAction};
use ui_math::UiRect;

use crate::composition_conformance_fixtures;

pub struct AdaptiveCompositionFixtureRun {
    pub fixture_id: ui_composition::CompositionFixtureId,
    pub projection_valid: bool,
    pub semantic_sources: Vec<SemanticInputSource>,
    pub expected_action: UiSemanticAction,
}

impl AdaptiveCompositionFixtureRun {
    pub fn passed(&self) -> bool {
        self.projection_valid
            && self.semantic_sources.len() == 4
            && self.expected_action == UiSemanticAction::Focus(SemanticDirection::Next)
    }
}

pub fn adaptive_composition_conformance_fixtures()
-> Result<Vec<CompositionFixture>, ui_composition::NamespacedReferenceError> {
    composition_conformance_fixtures()
}

pub fn run_adaptive_composition_conformance_fixtures()
-> Result<Vec<AdaptiveCompositionFixtureRun>, ui_composition::NamespacedReferenceError> {
    adaptive_composition_conformance_fixtures()?
        .into_iter()
        .map(run_fixture)
        .collect()
}

fn run_fixture(
    fixture: CompositionFixture,
) -> Result<AdaptiveCompositionFixtureRun, ui_composition::NamespacedReferenceError> {
    let fixture_id = fixture.id;
    let run = fixture.run();
    let Some(state) = run.state else {
        return Ok(AdaptiveCompositionFixtureRun {
            fixture_id,
            projection_valid: false,
            semantic_sources: Vec::new(),
            expected_action: UiSemanticAction::Focus(SemanticDirection::Next),
        });
    };
    let targets = state
        .definition()
        .targets()
        .iter()
        .map(|target| AdaptiveTargetConstraints {
            target: target.id,
            bounds: UiRect::new(0.0, 0.0, 1024.0, 768.0),
            text_scale: 1.25,
            minimum_touch_target: 44.0,
            high_contrast: true,
            reduced_motion: true,
        });
    let regions = state
        .definition()
        .regions()
        .iter()
        .map(|region| AdaptiveRegionPolicy {
            region: region.id,
            minimum_width: 320.0,
            minimum_height: 240.0,
            priority: 1,
            compact_behavior: CompactBehavior::Drawer,
        });
    let policy = AdaptiveProjectionPolicy::new(targets, regions)
        .expect("fixture constraints should be valid");
    let projection_valid = AdaptiveProjectionState::derive(state.snapshot(), &policy)
        .is_ok_and(|projection| projection.accessibility().is_complete());
    Ok(AdaptiveCompositionFixtureRun {
        fixture_id,
        projection_valid,
        semantic_sources: vec![
            SemanticInputSource::Pointer,
            SemanticInputSource::Keyboard,
            SemanticInputSource::Touch,
            SemanticInputSource::Controller,
        ],
        expected_action: UiSemanticAction::Focus(SemanticDirection::Next),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn adaptive_composition_fixtures_remain_product_free_and_accessible() {
        let fixtures = adaptive_composition_conformance_fixtures().unwrap();
        assert_eq!(fixtures.len(), 5);
        assert!(fixtures.iter().all(|fixture| {
            !fixture.forbidden_imports.is_empty() && !fixture.forbidden_product_behaviors.is_empty()
        }));
        assert!(
            run_adaptive_composition_conformance_fixtures()
                .unwrap()
                .iter()
                .all(AdaptiveCompositionFixtureRun::passed)
        );
    }
}
