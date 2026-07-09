use std::collections::{BTreeMap, BTreeSet};

use ui_composition::*;

struct Allow;

impl CompositionLifecyclePolicy for Allow {
    fn evaluate(
        &self,
        _snapshot: CompositionSnapshot<'_>,
        _transaction: &CompositionTransaction,
    ) -> CompositionPolicyDecision {
        CompositionPolicyDecision::Accepted
    }
}

impl CompositionCapabilityPolicy for Allow {
    fn evaluate(
        &self,
        _snapshot: CompositionSnapshot<'_>,
        _transaction: &CompositionTransaction,
    ) -> CompositionPolicyDecision {
        CompositionPolicyDecision::Accepted
    }
}

impl CompositionTargetPolicy for Allow {
    fn evaluate(
        &self,
        _snapshot: CompositionSnapshot<'_>,
        _transaction: &CompositionTransaction,
    ) -> CompositionPolicyDecision {
        CompositionPolicyDecision::Accepted
    }
}

fn content(instance: &str) -> MountedContentRef {
    MountedContentRef::new(
        ContentOwnerId::new("fixture.owner").unwrap(),
        ContentProfileId::new("fixture.content").unwrap(),
        ContentInstanceRef::new(format!("fixture.{instance}")).unwrap(),
    )
}

fn definition(reverse: bool) -> CompositionDefinitionV1 {
    let mut regions = vec![
        RegionDefinition::new(
            RegionId::new(10),
            None,
            RegionKind::Split {
                axis: SplitAxis::Horizontal,
                fraction: SplitFraction::try_new(5000).unwrap(),
                first: RegionId::new(20),
                second: RegionId::new(30),
            },
        ),
        RegionDefinition::new(
            RegionId::new(20),
            None,
            RegionKind::MountPoint {
                mounted_unit: MountedUnitId::new(1),
            },
        ),
        RegionDefinition::new(
            RegionId::new(30),
            None,
            RegionKind::MountPoint {
                mounted_unit: MountedUnitId::new(2),
            },
        ),
    ];
    let mut units = vec![
        MountedUnitDefinition::new(
            MountedUnitId::new(1),
            content("one"),
            [CapabilityId::new("fixture.edit").unwrap()],
            UnavailableContentPolicy::ShowFallback,
        ),
        MountedUnitDefinition::new(
            MountedUnitId::new(2),
            content("two"),
            [CapabilityId::new("fixture.inspect").unwrap()],
            UnavailableContentPolicy::AllowHide,
        ),
    ];
    if reverse {
        regions.reverse();
        units.reverse();
    }
    CompositionDefinitionV1::new(
        CompositionDefinitionId::new(7),
        DefinitionRevision::new(3),
        vec![PresentationTargetDefinition::new(
            PresentationTargetId::new(1),
            TargetProfileId::new("fixture.desktop").unwrap(),
        )],
        vec![CompositionRootDefinition::new(
            CompositionRootId::new(1),
            PresentationTargetId::new(1),
            RegionId::new(10),
            true,
        )],
        regions,
        units,
    )
}

fn compatibility() -> CompositionCompatibility {
    CompositionCompatibility::new(
        AppProfileId::new("fixture.editor").unwrap(),
        AppSchemaVersion::new(2).unwrap(),
        AppSchemaVersion::new(4).unwrap(),
    )
    .unwrap()
}

fn extension(profile: &str, payload: &str) -> CanonicalExtensionPayload {
    CanonicalExtensionPayload::new(
        ExtensionProfileId::new(profile).unwrap(),
        ExtensionSchemaVersion::new(1).unwrap(),
        payload,
    )
    .unwrap()
}

fn fixture(id: u64, definition: CompositionDefinitionV1) -> CompositionFixture {
    let mounted_content = definition
        .mounted_units()
        .iter()
        .map(|unit| (unit.id, unit.content().clone()))
        .collect::<BTreeMap<_, _>>();
    let liveness = mounted_content
        .keys()
        .copied()
        .map(|unit| (unit, ContentLiveness::Resolved))
        .collect::<BTreeMap<_, _>>();
    CompositionFixture {
        id: CompositionFixtureId::new(id),
        host_profile: HostProfileId::new("fixture.host").unwrap(),
        target_profiles: [TargetProfileId::new("fixture.desktop").unwrap()]
            .into_iter()
            .collect(),
        definition,
        mounted_content,
        liveness,
        expected_validity: ExpectedCompositionValidity::Valid,
        expected_capabilities: [
            CapabilityId::new("fixture.edit").unwrap(),
            CapabilityId::new("fixture.inspect").unwrap(),
        ]
        .into_iter()
        .collect(),
        expected_diagnostics: BTreeSet::new(),
        expected_adaptive_proposals: BTreeSet::new(),
        forbidden_imports: ["product.runtime".to_owned()].into_iter().collect(),
        forbidden_product_behaviors: ["execute product behavior".to_owned()]
            .into_iter()
            .collect(),
    }
}

#[test]
fn canonical_definition_and_fixture_catalog_ignore_input_order() {
    let ordered = definition(false);
    let reversed = definition(true);
    let ordered_source = CanonicalCompositionDocuments::definition(&ordered).unwrap();
    let reversed_source = CanonicalCompositionDocuments::definition(&reversed).unwrap();
    assert_eq!(ordered_source, reversed_source);
    assert_eq!(
        CanonicalCompositionDocuments::decode_definition(&ordered_source).unwrap(),
        CompositionState::form(ordered)
            .unwrap()
            .definition()
            .clone()
    );

    let first = fixture(1, definition(false));
    let second = fixture(2, definition(true));
    let left = CompositionFixtureCatalogV1::new(vec![second.clone(), first.clone()]);
    let right = CompositionFixtureCatalogV1::new(vec![first, second]);
    let left_source = CanonicalCompositionDocuments::fixture_catalog(&left).unwrap();
    assert_eq!(
        left_source,
        CanonicalCompositionDocuments::fixture_catalog(&right).unwrap()
    );
    assert_eq!(
        CanonicalCompositionDocuments::decode_fixture_catalog(&left_source).unwrap(),
        left
    );
}

#[test]
fn extension_order_does_not_change_links_documents_or_generation_identity() {
    let first = extension("fixture.editor.panels", "(visible:true)\n");
    let second = extension("fixture.editor.session", "(selected:7)\n");
    let left = CompositionBundleCandidate::form(
        definition(false),
        compatibility(),
        vec![second.clone(), first.clone()],
    )
    .unwrap();
    let right =
        CompositionBundleCandidate::form(definition(true), compatibility(), vec![first, second])
            .unwrap();

    assert_eq!(left.core(), right.core());
    assert_eq!(left.extensions(), right.extensions());
    assert_eq!(left.generation_id(), right.generation_id());
    assert_eq!(
        CanonicalCompositionDocuments::core_envelope(left.core()).unwrap(),
        CanonicalCompositionDocuments::core_envelope(right.core()).unwrap()
    );
    for extension in left.extensions() {
        let source = CanonicalCompositionDocuments::extension_envelope(extension).unwrap();
        assert_eq!(
            CanonicalCompositionDocuments::decode_extension_envelope(&source).unwrap(),
            extension.clone()
        );
    }

    let pointer = CompositionGenerationPointerV1::new(left.generation_id().clone(), None);
    let source = CanonicalCompositionDocuments::generation_pointer(&pointer).unwrap();
    assert_eq!(
        CanonicalCompositionDocuments::decode_generation_pointer(&source).unwrap(),
        pointer
    );
}

#[test]
fn canonical_decoders_reject_unknown_fields_and_noncanonical_text() {
    let source = CanonicalCompositionDocuments::definition(&definition(false)).unwrap();
    let unknown = source.replacen(
        "schema_version:",
        "unknown_field: true,\n  schema_version:",
        1,
    );
    let rejection = CanonicalCompositionDocuments::decode_definition(&unknown).unwrap_err();
    assert!(rejection.diagnostics().iter().any(|diagnostic| {
        diagnostic.code() == CompositionPersistenceDiagnosticCode::CanonicalDecodeFailed
    }));

    let noncanonical = format!("\n{source}");
    let rejection = CanonicalCompositionDocuments::decode_definition(&noncanonical).unwrap_err();
    assert!(rejection.diagnostics().iter().any(|diagnostic| {
        diagnostic.code() == CompositionPersistenceDiagnosticCode::NonCanonicalDocument
    }));
}

#[test]
fn digest_text_is_lowercase_blake3_only() {
    let digest = CompositionDigest::hash(b"deterministic");
    assert_eq!(
        CompositionDigest::parse(digest.to_string()).unwrap(),
        digest
    );
    assert!(CompositionDigest::parse(digest.to_string().to_ascii_uppercase()).is_err());
    assert!(CompositionDigest::parse("sha256:00").is_err());
}

#[test]
fn transaction_journal_round_trips_as_one_canonical_order() {
    let allow = Allow;
    let policies = CompositionPolicies {
        lifecycle: &allow,
        capability: &allow,
        target: &allow,
    };
    let mut state = CompositionState::form(definition(false)).unwrap();
    state
        .transact(
            CompositionTransaction::new(
                CompositionTransactionId::new(9),
                StateRevision::new(3),
                vec![CompositionCommand::resize_split(
                    RegionId::new(10),
                    SplitFraction::try_new(6500).unwrap(),
                )],
            ),
            policies,
        )
        .unwrap();
    let journal = CompositionJournalDocumentV1::from_history(state.history());
    let source = CanonicalCompositionDocuments::journal(&journal).unwrap();
    assert_eq!(
        CanonicalCompositionDocuments::decode_journal(&source).unwrap(),
        journal
    );
    assert_eq!(
        source,
        CanonicalCompositionDocuments::journal(&CompositionJournalDocumentV1::from_history(
            state.history()
        ))
        .unwrap()
    );
}
