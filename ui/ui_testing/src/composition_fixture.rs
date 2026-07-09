use std::collections::{BTreeMap, BTreeSet};

use ui_composition::*;

pub fn composition_conformance_fixtures()
-> Result<Vec<CompositionFixture>, NamespacedReferenceError> {
    [
        (1, "browser", "desktop", ContentLiveness::Resolved),
        (2, "terminal", "desktop", ContentLiveness::Missing),
        (3, "dashboard", "desktop", ContentLiveness::Loading),
        (4, "mobile", "mobile", ContentLiveness::Suspended),
        (5, "game", "game", ContentLiveness::Denied),
    ]
    .into_iter()
    .map(|(id, name, target, liveness)| fixture(id, name, target, liveness))
    .collect()
}

pub fn run_composition_conformance_fixtures()
-> Result<Vec<CompositionFixtureRun>, NamespacedReferenceError> {
    Ok(composition_conformance_fixtures()?
        .iter()
        .map(CompositionFixture::run)
        .collect())
}

fn fixture(
    id: u64,
    name: &str,
    target_profile: &str,
    liveness: ContentLiveness,
) -> Result<CompositionFixture, NamespacedReferenceError> {
    let unit_id = MountedUnitId::new(1);
    let content = MountedContentRef::new(
        ContentOwnerId::new(format!("fixture.{name}"))?,
        ContentProfileId::new(format!("fixture.{name}_content"))?,
        ContentInstanceRef::new(format!("fixture.{name}_instance"))?,
    );
    let target_profile = TargetProfileId::new(format!("fixture.{target_profile}"))?;
    let capability = CapabilityId::new("fixture.observe")?;
    let definition = CompositionDefinitionV1::new(
        CompositionDefinitionId::new(id),
        DefinitionRevision::new(1),
        vec![PresentationTargetDefinition::new(
            PresentationTargetId::new(1),
            target_profile.clone(),
        )],
        vec![CompositionRootDefinition::new(
            CompositionRootId::new(1),
            PresentationTargetId::new(1),
            RegionId::new(1),
            true,
        )],
        vec![RegionDefinition::new(
            RegionId::new(1),
            None,
            RegionKind::MountPoint {
                mounted_unit: unit_id,
            },
        )],
        vec![MountedUnitDefinition::new(
            unit_id,
            content.clone(),
            [capability.clone()],
            UnavailableContentPolicy::ShowFallback,
        )],
    );
    Ok(CompositionFixture {
        id: CompositionFixtureId::new(id),
        host_profile: HostProfileId::new(format!("fixture.{name}_host"))?,
        target_profiles: BTreeSet::from([target_profile]),
        definition,
        mounted_content: BTreeMap::from([(unit_id, content)]),
        liveness: BTreeMap::from([(unit_id, liveness)]),
        expected_validity: ExpectedCompositionValidity::Valid,
        expected_capabilities: BTreeSet::from([capability]),
        expected_diagnostics: BTreeSet::new(),
        expected_adaptive_proposals: BTreeSet::from([AdaptiveProposalExpectationId::new(
            "fixture.no_change",
        )?]),
        forbidden_imports: BTreeSet::from([
            "browser_engine".to_owned(),
            "terminal_process".to_owned(),
            "gameplay_runtime".to_owned(),
        ]),
        forbidden_product_behaviors: BTreeSet::from([format!("execute_{name}_behavior")]),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fixtures_are_headless_structural_conformance_only() {
        let fixtures = composition_conformance_fixtures().unwrap();
        assert_eq!(fixtures.len(), 5);
        assert!(
            fixtures
                .iter()
                .all(|fixture| !fixture.forbidden_imports.is_empty())
        );
        assert!(
            fixtures
                .iter()
                .all(|fixture| !fixture.forbidden_product_behaviors.is_empty())
        );
        assert!(
            run_composition_conformance_fixtures()
                .unwrap()
                .iter()
                .all(CompositionFixtureRun::passed)
        );
    }

    #[test]
    fn unavailable_fallback_order_covers_all_liveness_states() {
        for liveness in [
            ContentLiveness::Missing,
            ContentLiveness::Loading,
            ContentLiveness::Suspended,
            ContentLiveness::Denied,
            ContentLiveness::UnsupportedProfile,
            ContentLiveness::Crashed,
        ] {
            assert_eq!(
                select_content_projection_fallback(
                    liveness,
                    true,
                    true,
                    UnavailableContentPolicy::AllowHide,
                    true
                ),
                Some(ContentProjectionFallback::AppProvidedUnavailable)
            );
            assert_eq!(
                select_content_projection_fallback(
                    liveness,
                    false,
                    true,
                    UnavailableContentPolicy::AllowHide,
                    true
                ),
                Some(ContentProjectionFallback::NeutralDiagnosticPlaceholder)
            );
            assert_eq!(
                select_content_projection_fallback(
                    liveness,
                    false,
                    false,
                    UnavailableContentPolicy::AllowHide,
                    true
                ),
                Some(ContentProjectionFallback::Hidden)
            );
            assert_eq!(
                select_content_projection_fallback(
                    liveness,
                    false,
                    false,
                    UnavailableContentPolicy::ShowFallback,
                    true
                ),
                None
            );
        }
    }
}
