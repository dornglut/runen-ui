//! Transient adaptive projection and proposal mechanism over UI composition.

#![forbid(unsafe_code)]

pub mod accessibility;
pub mod diagnostic;
pub mod fixture;
pub mod interaction;
pub mod projection;
pub mod promotion;
pub mod proposal;

pub use accessibility::*;
pub use diagnostic::*;
pub use fixture::*;
pub use interaction::*;
pub use projection::*;
pub use promotion::*;
pub use proposal::*;

#[cfg(test)]
mod tests {
    use std::sync::Arc;

    use ui_composition::{MountedUnitId, RegionId, StateRevision};
    use ui_input::{SemanticActionEvent, SemanticInputSource, UiSemanticAction};
    use ui_math::{UiPoint, UiRect};

    use super::*;

    #[test]
    fn adaptive_projection_and_cancel_never_mutate_canonical_state() {
        let fixture = normal_benchmark_fixture();
        let definition_before = fixture.state.definition().clone();
        let revision_before = fixture.state.revision();
        let projection = Arc::new(
            AdaptiveProjectionState::derive(fixture.state.snapshot(), &fixture.policy).unwrap(),
        );
        let mut session = DragSession::begin(projection, MountedUnitId::new(1));
        session.update_pointer(UiPoint::new(10.0, 10.0));
        assert_eq!(
            session.handle_semantic_action(SemanticActionEvent::new(
                SemanticInputSource::Keyboard,
                UiSemanticAction::Cancel,
            )),
            SessionSemanticOutcome::Cancelled
        );
        assert!(session.commit().is_none());
        assert_eq!(fixture.state.definition(), &definition_before);
        assert_eq!(fixture.state.revision(), revision_before);
    }

    #[test]
    fn adaptive_semantic_commit_is_source_independent_and_proposal_only() {
        for source in [
            SemanticInputSource::Pointer,
            SemanticInputSource::Keyboard,
            SemanticInputSource::Touch,
            SemanticInputSource::Controller,
        ] {
            let fixture = normal_benchmark_fixture();
            let projection = Arc::new(
                AdaptiveProjectionState::derive(fixture.state.snapshot(), &fixture.policy).unwrap(),
            );
            let mut session = DragSession::begin(projection, MountedUnitId::new(1));
            session.update_pointer(UiPoint::new(10.0, 10.0));
            assert_eq!(
                session.handle_semantic_action(SemanticActionEvent::new(
                    source,
                    UiSemanticAction::Commit,
                )),
                SessionSemanticOutcome::CommitRequested
            );
            let proposal = session.commit().unwrap();
            assert_eq!(
                proposal.classification,
                AdaptiveEditClassification::StructuralTransaction
            );
            assert!(proposal.requires_host_transaction());
            assert_eq!(fixture.state.revision(), StateRevision::new(1));
        }
    }

    #[test]
    fn adaptive_promotion_is_explicit_named_scoped_and_deterministic() {
        let delta = AdaptivePromotionDelta::new(
            StateRevision::new(3),
            "Compact laptop",
            "user",
            vec![
                AdaptivePromotionOverride {
                    region: RegionId::new(2),
                    projected_bounds: UiRect::new(0.0, 0.0, 100.0, 100.0),
                },
                AdaptivePromotionOverride {
                    region: RegionId::new(1),
                    projected_bounds: UiRect::new(100.0, 0.0, 100.0, 100.0),
                },
            ],
        )
        .unwrap();
        assert_eq!(
            delta.classification(),
            AdaptiveEditClassification::PromotionCandidate
        );
        assert_eq!(delta.overrides[0].region, RegionId::new(1));
        assert!(
            AdaptivePromotionDelta::new(StateRevision::new(3), "", "user", Vec::new()).is_err()
        );
    }

    #[test]
    fn adaptive_accessibility_metadata_covers_contrast_scale_motion_touch_and_focus() {
        let fixture = normal_benchmark_fixture();
        let projection =
            AdaptiveProjectionState::derive(fixture.state.snapshot(), &fixture.policy).unwrap();
        assert!(projection.accessibility().is_complete());
        assert!(projection.accessibility().nodes().iter().all(|node| {
            node.focus_visible
                && node.minimum_touch_target >= 44.0
                && node.text_scale == 1.0
                && matches!(node.transition_duration_ms, 0 | 120)
        }));
        assert!(
            projection
                .accessibility()
                .nodes()
                .iter()
                .any(|node| node.high_contrast)
        );
        assert!(
            projection
                .accessibility()
                .nodes()
                .iter()
                .any(|node| !node.high_contrast)
        );
        assert!(
            projection
                .accessibility()
                .nodes()
                .iter()
                .any(|node| node.transition_duration_ms == 0)
        );
    }

    #[test]
    fn adaptive_active_mechanism_has_no_composition_mutation_entrypoint() {
        let sources = [
            include_str!("projection/model.rs"),
            include_str!("interaction/session.rs"),
            include_str!("proposal/model.rs"),
            include_str!("promotion/mod.rs"),
        ]
        .join("\n");
        for forbidden in [
            "&mut CompositionState",
            ".transact(",
            "apply_authorized",
            "CompositionCommand",
            "domain::editor",
            "ui_surface",
            "gamepad",
        ] {
            assert!(
                !sources.contains(forbidden),
                "forbidden adaptive authority: {forbidden}"
            );
        }
    }

    #[test]
    fn adaptive_dock_proposals_preserve_all_five_zones_as_host_materialized_intent() {
        let fixture = normal_benchmark_fixture();
        let projection = Arc::new(
            AdaptiveProjectionState::derive(fixture.state.snapshot(), &fixture.policy).unwrap(),
        );
        let cases = [
            (UiPoint::new(10.0, 128.0), DockZone::Left),
            (UiPoint::new(350.0, 128.0), DockZone::Right),
            (UiPoint::new(180.0, 10.0), DockZone::Top),
            (UiPoint::new(180.0, 246.0), DockZone::Bottom),
            (UiPoint::new(180.0, 128.0), DockZone::Center),
        ];

        for (point, expected_zone) in cases {
            let mut session = DragSession::begin(Arc::clone(&projection), MountedUnitId::new(1));
            session.update_pointer(point).unwrap();
            let proposal = session.commit().unwrap();
            let AdaptiveProposalKind::DockUnit { zone, .. } = proposal.kind else {
                panic!("drag commit must preserve dock intent")
            };
            assert_eq!(zone, expected_zone);
            assert!(proposal.requires_host_transaction());
        }
    }
}
