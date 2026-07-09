use proptest::prelude::*;
use ui_composition::*;

fn definition(unit_count: u64, reverse: bool) -> CompositionDefinitionV1 {
    let mut units = (1..=unit_count)
        .map(|id| {
            MountedUnitDefinition::new(
                MountedUnitId::new(id),
                MountedContentRef::new(
                    ContentOwnerId::new("fixture.owner").unwrap(),
                    ContentProfileId::new("fixture.content").unwrap(),
                    ContentInstanceRef::new(format!("fixture.unit-{id}")).unwrap(),
                ),
                [],
                UnavailableContentPolicy::ShowFallback,
            )
        })
        .collect::<Vec<_>>();
    if reverse {
        units.reverse();
    }
    CompositionDefinitionV1::new(
        CompositionDefinitionId::new(1),
        DefinitionRevision::new(1),
        vec![PresentationTargetDefinition::new(
            PresentationTargetId::new(1),
            TargetProfileId::new("fixture.desktop").unwrap(),
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
            RegionKind::Stack {
                ordered_units: (1..=unit_count).map(MountedUnitId::new).collect(),
                active_unit: MountedUnitId::new(1),
            },
        )],
        units,
    )
}

proptest! {
    #[test]
    fn formation_is_independent_of_record_insertion_order(unit_count in 1u64..40) {
        let forward = CompositionState::form(definition(unit_count, false)).unwrap();
        let reverse = CompositionState::form(definition(unit_count, true)).unwrap();
        prop_assert_eq!(forward, reverse);
    }

    #[test]
    fn one_fault_duplicate_location_is_rejected(unit_count in 2u64..40) {
        let valid = definition(unit_count, false);
        let mut regions = valid.regions().to_vec();
        regions.push(RegionDefinition::new(RegionId::new(2), None, RegionKind::MountPoint { mounted_unit: MountedUnitId::new(1) }));
        let invalid = CompositionDefinitionV1::new(valid.id(), valid.revision(), valid.targets().to_vec(), valid.roots().to_vec(), regions, valid.mounted_units().to_vec());
        let rejection = CompositionState::form(invalid).unwrap_err();
        prop_assert!(rejection.diagnostics().iter().any(|value| value.code() == CompositionDiagnosticCode::InvalidMountedUnitLocation));
    }
}
