use ui_composition::*;

fn content(instance: &str) -> MountedContentRef {
    MountedContentRef::new(
        ContentOwnerId::new("fixture.owner").unwrap(),
        ContentProfileId::new("fixture.content").unwrap(),
        ContentInstanceRef::new(format!("fixture.{instance}")).unwrap(),
    )
}

fn valid_definition() -> CompositionDefinitionV1 {
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
            RegionKind::MountPoint {
                mounted_unit: MountedUnitId::new(1),
            },
        )],
        vec![MountedUnitDefinition::new(
            MountedUnitId::new(1),
            content("main"),
            [],
            UnavailableContentPolicy::ShowFallback,
        )],
    )
}

#[test]
fn formation_normalizes_record_order_and_exposes_read_only_snapshot() {
    let state = CompositionState::form(valid_definition()).unwrap();
    let snapshot = state.snapshot();
    assert_eq!(snapshot.revision(), StateRevision::new(1));
    assert_eq!(snapshot.targets().len(), 1);
    assert_eq!(
        snapshot.region(RegionId::new(1)).unwrap().id,
        RegionId::new(1)
    );
}

#[test]
fn formation_rejects_duplicate_ids_with_stable_code() {
    let valid = valid_definition();
    let duplicate = valid.mounted_units()[0].clone();
    let invalid = CompositionDefinitionV1::new(
        valid.id(),
        valid.revision(),
        valid.targets().to_vec(),
        valid.roots().to_vec(),
        valid.regions().to_vec(),
        vec![duplicate.clone(), duplicate],
    );
    let rejection = CompositionState::form(invalid).unwrap_err();
    assert!(
        rejection
            .diagnostics()
            .iter()
            .any(|value| value.code() == CompositionDiagnosticCode::DuplicateMountedUnitId)
    );
}

#[test]
fn formation_rejects_cycle_and_parent_ambiguity() {
    let valid = valid_definition();
    let fraction = SplitFraction::try_new(5000).unwrap();
    let invalid = CompositionDefinitionV1::new(
        valid.id(),
        valid.revision(),
        valid.targets().to_vec(),
        vec![CompositionRootDefinition::new(
            CompositionRootId::new(1),
            PresentationTargetId::new(1),
            RegionId::new(1),
            true,
        )],
        vec![
            RegionDefinition::new(
                RegionId::new(1),
                None,
                RegionKind::Split {
                    axis: SplitAxis::Horizontal,
                    fraction,
                    first: RegionId::new(2),
                    second: RegionId::new(3),
                },
            ),
            RegionDefinition::new(
                RegionId::new(2),
                None,
                RegionKind::Overlay {
                    base: RegionId::new(1),
                    ordered_overlays: vec![],
                },
            ),
            RegionDefinition::new(
                RegionId::new(3),
                None,
                RegionKind::MountPoint {
                    mounted_unit: MountedUnitId::new(1),
                },
            ),
        ],
        valid.mounted_units().to_vec(),
    );
    let rejection = CompositionState::form(invalid).unwrap_err();
    assert!(
        rejection
            .diagnostics()
            .iter()
            .any(|value| value.code() == CompositionDiagnosticCode::RegionCycle)
    );
}

#[test]
fn all_liveness_states_are_structurally_neutral() {
    let state = CompositionState::form(valid_definition()).unwrap();
    for liveness in [
        ContentLiveness::Resolved,
        ContentLiveness::Missing,
        ContentLiveness::Loading,
        ContentLiveness::Suspended,
        ContentLiveness::Denied,
        ContentLiveness::UnsupportedProfile,
        ContentLiveness::Crashed,
    ] {
        let observation = ContentLivenessObservation {
            mounted_unit: MountedUnitId::new(1),
            state: liveness,
        };
        assert_eq!(observation.mounted_unit, MountedUnitId::new(1));
        assert_eq!(state.snapshot().mounted_units().len(), 1);
    }
}

#[test]
fn semantic_references_validate_construction_and_deserialization() {
    assert!(TargetProfileId::new("fixture.desktop").is_ok());
    assert!(TargetProfileId::new("desktop").is_err());
    assert!(TargetProfileId::new("fixture.-desktop").is_err());
    assert!(serde_json::from_str::<TargetProfileId>("\"desktop\"").is_err());
    assert!(serde_json::from_str::<TargetProfileId>("\"fixture.desktop-\"").is_err());

    let reference = TargetProfileId::new("fixture.desktop-wide").unwrap();
    let encoded = serde_json::to_string(&reference).unwrap();
    assert_eq!(
        serde_json::from_str::<TargetProfileId>(&encoded).unwrap(),
        reference
    );
}

fn assert_formation_code(definition: CompositionDefinitionV1, expected: CompositionDiagnosticCode) {
    let rejection = CompositionState::form(definition).unwrap_err();
    assert!(
        rejection
            .diagnostics()
            .iter()
            .any(|record| record.code() == expected),
        "missing diagnostic {}; actual: {:?}",
        expected.as_str(),
        rejection.diagnostics()
    );
}

fn definition_from(
    target: PresentationTargetDefinition,
    root: CompositionRootDefinition,
    regions: Vec<RegionDefinition>,
    units: Vec<MountedUnitDefinition>,
) -> CompositionDefinitionV1 {
    CompositionDefinitionV1::new(
        CompositionDefinitionId::new(1),
        DefinitionRevision::new(1),
        vec![target],
        vec![root],
        regions,
        units,
    )
}

#[test]
fn formation_diagnostic_matrix_covers_definition_and_reference_invariants() {
    assert_formation_code(
        CompositionDefinitionV1::new(
            CompositionDefinitionId::new(1),
            DefinitionRevision::new(1),
            vec![],
            vec![],
            vec![],
            vec![],
        ),
        CompositionDiagnosticCode::EmptyDefinition,
    );

    let valid = valid_definition();
    let mut encoded = serde_json::to_value(&valid).unwrap();
    encoded["schema_version"] = serde_json::json!(2);
    assert_formation_code(
        serde_json::from_value(encoded).unwrap(),
        CompositionDiagnosticCode::UnsupportedSchemaVersion,
    );

    for (definition, code) in [
        (
            CompositionDefinitionV1::new(
                valid.id(),
                valid.revision(),
                vec![valid.targets()[0].clone(), valid.targets()[0].clone()],
                valid.roots().to_vec(),
                valid.regions().to_vec(),
                valid.mounted_units().to_vec(),
            ),
            CompositionDiagnosticCode::DuplicateTargetId,
        ),
        (
            CompositionDefinitionV1::new(
                valid.id(),
                valid.revision(),
                valid.targets().to_vec(),
                vec![valid.roots()[0].clone(), valid.roots()[0].clone()],
                valid.regions().to_vec(),
                valid.mounted_units().to_vec(),
            ),
            CompositionDiagnosticCode::DuplicateRootId,
        ),
        (
            CompositionDefinitionV1::new(
                valid.id(),
                valid.revision(),
                valid.targets().to_vec(),
                valid.roots().to_vec(),
                vec![valid.regions()[0].clone(), valid.regions()[0].clone()],
                valid.mounted_units().to_vec(),
            ),
            CompositionDiagnosticCode::DuplicateRegionId,
        ),
    ] {
        assert_formation_code(definition, code);
    }

    assert_formation_code(
        definition_from(
            valid.targets()[0].clone(),
            CompositionRootDefinition::new(
                CompositionRootId::new(1),
                PresentationTargetId::new(99),
                RegionId::new(1),
                true,
            ),
            valid.regions().to_vec(),
            valid.mounted_units().to_vec(),
        ),
        CompositionDiagnosticCode::UnknownTarget,
    );
    assert_formation_code(
        definition_from(
            valid.targets()[0].clone(),
            CompositionRootDefinition::new(
                CompositionRootId::new(1),
                PresentationTargetId::new(1),
                RegionId::new(99),
                true,
            ),
            valid.regions().to_vec(),
            valid.mounted_units().to_vec(),
        ),
        CompositionDiagnosticCode::UnknownRegion,
    );
    assert_formation_code(
        definition_from(
            valid.targets()[0].clone(),
            valid.roots()[0].clone(),
            vec![RegionDefinition::new(
                RegionId::new(1),
                None,
                RegionKind::MountPoint {
                    mounted_unit: MountedUnitId::new(99),
                },
            )],
            valid.mounted_units().to_vec(),
        ),
        CompositionDiagnosticCode::UnknownMountedUnit,
    );
    assert_formation_code(
        definition_from(
            valid.targets()[0].clone(),
            CompositionRootDefinition::new(
                CompositionRootId::new(1),
                PresentationTargetId::new(1),
                RegionId::new(1),
                false,
            ),
            valid.regions().to_vec(),
            valid.mounted_units().to_vec(),
        ),
        CompositionDiagnosticCode::InvalidPrimaryRootCount,
    );
}

#[test]
fn formation_diagnostic_matrix_covers_closed_region_algebra() {
    let valid = valid_definition();
    let unit = valid.mounted_units()[0].clone();
    let target = valid.targets()[0].clone();
    let root = valid.roots()[0].clone();

    let empty_stack_definition = definition_from(
        target.clone(),
        root.clone(),
        vec![RegionDefinition::new(
            RegionId::new(1),
            None,
            RegionKind::Stack {
                ordered_units: vec![],
                active_unit: MountedUnitId::new(1),
            },
        )],
        vec![unit.clone()],
    );
    assert_formation_code(
        empty_stack_definition.clone(),
        CompositionDiagnosticCode::EmptyStack,
    );
    assert_formation_code(
        empty_stack_definition,
        CompositionDiagnosticCode::InvalidActiveUnit,
    );

    assert_formation_code(
        definition_from(
            target.clone(),
            root.clone(),
            vec![RegionDefinition::new(
                RegionId::new(1),
                None,
                RegionKind::Stack {
                    ordered_units: vec![MountedUnitId::new(1), MountedUnitId::new(1)],
                    active_unit: MountedUnitId::new(1),
                },
            )],
            vec![unit.clone()],
        ),
        CompositionDiagnosticCode::DuplicateStackUnit,
    );

    let overlay_definition = definition_from(
        target.clone(),
        root.clone(),
        vec![
            RegionDefinition::new(
                RegionId::new(1),
                None,
                RegionKind::Overlay {
                    base: RegionId::new(2),
                    ordered_overlays: vec![RegionId::new(2), RegionId::new(2)],
                },
            ),
            RegionDefinition::new(
                RegionId::new(2),
                None,
                RegionKind::MountPoint {
                    mounted_unit: MountedUnitId::new(1),
                },
            ),
        ],
        vec![unit.clone()],
    );
    assert_formation_code(
        overlay_definition.clone(),
        CompositionDiagnosticCode::DuplicateOverlayRegion,
    );
    assert_formation_code(
        overlay_definition,
        CompositionDiagnosticCode::OverlayContainsBase,
    );

    assert_formation_code(
        definition_from(
            target.clone(),
            root.clone(),
            vec![
                RegionDefinition::new(
                    RegionId::new(1),
                    None,
                    RegionKind::Split {
                        axis: SplitAxis::Horizontal,
                        fraction: SplitFraction::try_new(5000).unwrap(),
                        first: RegionId::new(2),
                        second: RegionId::new(2),
                    },
                ),
                RegionDefinition::new(
                    RegionId::new(2),
                    None,
                    RegionKind::MountPoint {
                        mounted_unit: MountedUnitId::new(1),
                    },
                ),
            ],
            vec![unit.clone()],
        ),
        CompositionDiagnosticCode::IdenticalSplitChildren,
    );

    let unreachable = definition_from(
        target,
        root,
        vec![
            valid.regions()[0].clone(),
            RegionDefinition::new(
                RegionId::new(2),
                None,
                RegionKind::Stack {
                    ordered_units: vec![],
                    active_unit: MountedUnitId::new(1),
                },
            ),
        ],
        vec![unit],
    );
    assert_formation_code(
        unreachable.clone(),
        CompositionDiagnosticCode::InvalidRegionParentCount,
    );
    assert_formation_code(unreachable, CompositionDiagnosticCode::UnreachableRegion);
}

#[test]
fn diagnostic_namespace_and_context_conversion_are_stable() {
    let codes = [
        CompositionDiagnosticCode::EmptyDefinition,
        CompositionDiagnosticCode::UnsupportedSchemaVersion,
        CompositionDiagnosticCode::DuplicateTargetId,
        CompositionDiagnosticCode::DuplicateRootId,
        CompositionDiagnosticCode::DuplicateRegionId,
        CompositionDiagnosticCode::DuplicateMountedUnitId,
        CompositionDiagnosticCode::UnknownTarget,
        CompositionDiagnosticCode::UnknownRegion,
        CompositionDiagnosticCode::UnknownMountedUnit,
        CompositionDiagnosticCode::InvalidPrimaryRootCount,
        CompositionDiagnosticCode::InvalidRegionParentCount,
        CompositionDiagnosticCode::RegionCycle,
        CompositionDiagnosticCode::UnreachableRegion,
        CompositionDiagnosticCode::EmptyStack,
        CompositionDiagnosticCode::DuplicateStackUnit,
        CompositionDiagnosticCode::InvalidActiveUnit,
        CompositionDiagnosticCode::DuplicateOverlayRegion,
        CompositionDiagnosticCode::OverlayContainsBase,
        CompositionDiagnosticCode::IdenticalSplitChildren,
        CompositionDiagnosticCode::InvalidMountedUnitLocation,
        CompositionDiagnosticCode::EmptyTransaction,
        CompositionDiagnosticCode::StaleRevision,
        CompositionDiagnosticCode::RevisionOverflow,
        CompositionDiagnosticCode::DuplicateTransactionId,
        CompositionDiagnosticCode::PolicyRejected,
        CompositionDiagnosticCode::InvalidCommand,
        CompositionDiagnosticCode::HistoryUnavailable,
        CompositionDiagnosticCode::HistoryConflict,
        CompositionDiagnosticCode::PromotionRejected,
        CompositionDiagnosticCode::FixtureExpectationFailed,
    ];
    let spellings = codes
        .iter()
        .map(|code| code.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    assert_eq!(spellings.len(), codes.len());
    assert!(
        spellings
            .iter()
            .all(|code| code.starts_with("ui_composition."))
    );

    let record = CompositionDiagnosticRecord::error(
        CompositionDiagnosticCode::InvalidCommand,
        CompositionDiagnosticStage::Transaction,
        CompositionDiagnosticSubject::General("invalid subject with spaces".to_owned()),
        "Use a typed command.",
    );
    assert!(
        record
            .clone()
            .try_with_context("invalid key", "value")
            .is_err()
    );
    let _ = record
        .try_with_context("command_family", "activate_unit")
        .unwrap()
        .to_foundation_diagnostic();
}
