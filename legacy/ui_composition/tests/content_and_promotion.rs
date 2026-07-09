use ui_composition::*;

#[test]
fn promotion_uses_ratified_structure_and_new_definition_identity() {
    let definition = CompositionDefinitionV1::new(
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
            RegionKind::MountPoint {
                mounted_unit: MountedUnitId::new(1),
            },
        )],
        vec![MountedUnitDefinition::new(
            MountedUnitId::new(1),
            MountedContentRef::new(
                ContentOwnerId::new("fixture.owner").unwrap(),
                ContentProfileId::new("fixture.content").unwrap(),
                ContentInstanceRef::new("fixture.main").unwrap(),
            ),
            [],
            UnavailableContentPolicy::AllowHide,
        )],
    );
    let state = CompositionState::form(definition).unwrap();
    let promotion = state.promote_definition(
        CompositionDefinitionId::new(9),
        LayoutDisplayName::new("Primary editing").unwrap(),
        CompositionLayoutScope::Project,
        CompositionCompatibility::new(
            AppProfileId::new("fixture.editor").unwrap(),
            AppSchemaVersion::new(1).unwrap(),
            AppSchemaVersion::new(1).unwrap(),
        )
        .unwrap(),
    );
    assert_eq!(promotion.source_revision(), StateRevision::new(1));
    assert_eq!(promotion.candidate().id(), CompositionDefinitionId::new(9));
    assert_eq!(
        promotion.candidate().mounted_units()[0]
            .content()
            .instance()
            .as_str(),
        "fixture.main"
    );
    assert!(
        promotion.candidate().mounted_units()[0]
            .unavailable_policy()
            .permits_hide()
    );
}
