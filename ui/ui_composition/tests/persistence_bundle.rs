use ui_composition::*;

fn definition() -> CompositionDefinitionV1 {
    CompositionDefinitionV1::new(
        CompositionDefinitionId::new(11),
        DefinitionRevision::new(5),
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
            UnavailableContentPolicy::ShowFallback,
        )],
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

fn extension(profile: &str, value: u32) -> CanonicalExtensionPayload {
    CanonicalExtensionPayload::new(
        ExtensionProfileId::new(profile).unwrap(),
        ExtensionSchemaVersion::new(1).unwrap(),
        format!("(value:{value})\n"),
    )
    .unwrap()
}

fn candidate() -> CompositionBundleCandidate {
    CompositionBundleCandidate::form(
        definition(),
        compatibility(),
        vec![
            extension("fixture.editor.panels", 1),
            extension("fixture.editor.session", 2),
        ],
    )
    .unwrap()
}

fn assert_code(
    result: Result<ValidatedCompositionBundle, CompositionPersistenceRejection>,
    code: CompositionPersistenceDiagnosticCode,
) {
    let rejection = result.unwrap_err();
    assert!(
        rejection
            .diagnostics()
            .iter()
            .any(|diagnostic| diagnostic.code() == code),
        "missing {code:?}; actual diagnostics: {:?}",
        rejection.diagnostics()
    );
}

#[test]
fn valid_bundle_requires_exact_shared_metadata_links_digests_and_compatibility() {
    let candidate = candidate();
    let requirement = CompositionCompatibilityRequirement {
        app_profile: AppProfileId::new("fixture.editor").unwrap(),
        app_schema_version: AppSchemaVersion::new(3).unwrap(),
    };
    let validated = candidate.validate(Some(&requirement)).unwrap();
    assert_eq!(validated.generation_id(), candidate.generation_id());
    assert_eq!(validated.extensions().len(), 2);

    let incompatible = CompositionCompatibilityRequirement {
        app_profile: AppProfileId::new("fixture.other").unwrap(),
        app_schema_version: AppSchemaVersion::new(3).unwrap(),
    };
    assert_code(
        candidate.validate(Some(&incompatible)),
        CompositionPersistenceDiagnosticCode::CompatibilityMismatch,
    );
}

#[test]
fn bundle_rejects_missing_extra_and_duplicate_extensions() {
    let candidate = candidate();
    let core = candidate.core().clone();

    let mut missing = candidate.extensions().to_vec();
    missing.pop();
    assert_code(
        ValidatedCompositionBundle::from_envelopes(core.clone(), missing, None),
        CompositionPersistenceDiagnosticCode::MissingExtension,
    );

    let mut extra = candidate.extensions().to_vec();
    let mut undeclared = extra[0].clone();
    undeclared.link.identity.profile = ExtensionProfileId::new("fixture.editor.extra").unwrap();
    extra.push(undeclared);
    assert_code(
        ValidatedCompositionBundle::from_envelopes(core.clone(), extra, None),
        CompositionPersistenceDiagnosticCode::ExtraExtension,
    );

    let mut duplicate = candidate.extensions().to_vec();
    duplicate.push(duplicate[0].clone());
    assert_code(
        ValidatedCompositionBundle::from_envelopes(core, duplicate, None),
        CompositionPersistenceDiagnosticCode::DuplicateExtension,
    );
}

#[test]
fn bundle_rejects_tampered_payload_link_shared_metadata_and_core() {
    let candidate = candidate();

    let mut payloads = candidate.extensions().to_vec();
    payloads[0].payload_ron = "(value:99)\n".to_owned();
    assert_code(
        ValidatedCompositionBundle::from_envelopes(candidate.core().clone(), payloads, None),
        CompositionPersistenceDiagnosticCode::DigestMismatch,
    );

    let mut links = candidate.extensions().to_vec();
    links[0].link.core_payload_digest = CompositionDigest::hash(b"not-the-core");
    assert_code(
        ValidatedCompositionBundle::from_envelopes(candidate.core().clone(), links, None),
        CompositionPersistenceDiagnosticCode::LinkMismatch,
    );

    let mut shared = candidate.extensions().to_vec();
    shared[0].shared.definition_revision = DefinitionRevision::new(77);
    assert_code(
        ValidatedCompositionBundle::from_envelopes(candidate.core().clone(), shared, None),
        CompositionPersistenceDiagnosticCode::SharedMetadataMismatch,
    );

    let mut core = candidate.core().clone();
    core.core_payload_digest = CompositionDigest::hash(b"tampered-core");
    assert_code(
        ValidatedCompositionBundle::from_envelopes(core, candidate.extensions().to_vec(), None),
        CompositionPersistenceDiagnosticCode::LinkMismatch,
    );
}

struct CompleteSnapshots;

impl CompositionExtensionSnapshotPort for CompleteSnapshots {
    fn snapshot_extensions(
        &self,
        _layout_id: CompositionDefinitionId,
        _source_revision: StateRevision,
    ) -> Result<Vec<CanonicalExtensionPayload>, CompositionPersistenceRejection> {
        Ok(vec![extension("fixture.editor.snapshot", 8)])
    }
}

struct FailedSnapshots;

impl CompositionExtensionSnapshotPort for FailedSnapshots {
    fn snapshot_extensions(
        &self,
        _layout_id: CompositionDefinitionId,
        _source_revision: StateRevision,
    ) -> Result<Vec<CanonicalExtensionPayload>, CompositionPersistenceRejection> {
        Err(CompositionPersistenceRejection::single(
            CompositionPersistenceDiagnosticRecord::error(
                CompositionPersistenceDiagnosticCode::ReadbackFailed,
                CompositionPersistenceDiagnosticStage::Promotion,
                CompositionPersistenceDiagnosticSubject::General("extension_snapshot".to_owned()),
                "Retry after the app can snapshot every required extension.",
            ),
        ))
    }
}

#[test]
fn promotion_snapshots_app_extensions_as_one_all_or_none_bundle() {
    let state = CompositionState::form(definition()).unwrap();
    let promotion = state.promote_definition(
        CompositionDefinitionId::new(99),
        LayoutDisplayName::new("Editing").unwrap(),
        CompositionLayoutScope::Project,
        compatibility(),
    );
    let bundle = promotion.snapshot_bundle(&CompleteSnapshots).unwrap();
    assert_eq!(bundle.extensions().len(), 1);
    assert_eq!(
        bundle.core().shared.layout_id,
        CompositionDefinitionId::new(99)
    );
    assert!(promotion.snapshot_bundle(&FailedSnapshots).is_err());

    let source = CanonicalCompositionDocuments::core_envelope(bundle.core()).unwrap();
    assert!(!source.contains("ContentLiveness"));
    assert!(!source.contains("AdaptiveProjectionState"));
}
