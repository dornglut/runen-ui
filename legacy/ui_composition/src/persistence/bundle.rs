use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;

use crate::{CompositionDefinitionV1, CompositionState};

use super::{
    CanonicalCompositionDocuments, CanonicalExtensionPayload, CompositionCompatibility,
    CompositionCompatibilityRequirement, CompositionCoreEnvelopeV1, CompositionDigest,
    CompositionExtensionEnvelopeV1, CompositionExtensionIdentity, CompositionExtensionLinkV1,
    CompositionGenerationId, CompositionPersistenceDiagnosticCode as Code,
    CompositionPersistenceDiagnosticRecord as Record,
    CompositionPersistenceDiagnosticStage as Stage,
    CompositionPersistenceDiagnosticSubject as Subject, CompositionPersistenceRejection,
    CompositionSharedMetadataV1, encode_canonical,
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompositionBundleCandidate {
    core: CompositionCoreEnvelopeV1,
    extensions: Vec<CompositionExtensionEnvelopeV1>,
    generation_id: CompositionGenerationId,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValidatedCompositionBundle {
    core: CompositionCoreEnvelopeV1,
    extensions: Vec<CompositionExtensionEnvelopeV1>,
    generation_id: CompositionGenerationId,
}

#[derive(Serialize)]
struct CoreDigestInput<'a> {
    digest_schema: &'static str,
    shared: &'a CompositionSharedMetadataV1,
    extension_identities: &'a [CompositionExtensionIdentity],
    definition: &'a CompositionDefinitionV1,
}

#[derive(Serialize)]
struct ExtensionDigestInput<'a> {
    digest_schema: &'static str,
    shared: &'a CompositionSharedMetadataV1,
    identity: &'a CompositionExtensionIdentity,
    payload_ron: &'a str,
}

impl CompositionBundleCandidate {
    pub fn form(
        definition: CompositionDefinitionV1,
        compatibility: CompositionCompatibility,
        mut extension_payloads: Vec<CanonicalExtensionPayload>,
    ) -> Result<Self, CompositionPersistenceRejection> {
        let state = CompositionState::form(definition).map_err(|rejection| {
            CompositionPersistenceRejection::single(
                Record::error(
                    Code::SharedMetadataMismatch,
                    Stage::Bundle,
                    Subject::General("composition_definition".to_owned()),
                    "Form a valid composition definition before creating a persistence bundle.",
                )
                .with_context(
                    "formation_diagnostics",
                    rejection.diagnostics().len().to_string(),
                ),
            )
        })?;
        let definition = state.definition().clone();
        extension_payloads.sort_by(|left, right| left.identity().cmp(right.identity()));
        if extension_payloads
            .windows(2)
            .any(|pair| pair[0].identity() == pair[1].identity())
        {
            let Some(identity) = extension_payloads
                .windows(2)
                .find(|pair| pair[0].identity() == pair[1].identity())
                .map(|pair| pair[0].identity().clone())
            else {
                return Err(super::diagnostic::rejection(
                    Code::DuplicateExtension,
                    Stage::Bundle,
                    Subject::General("extension_identity".to_owned()),
                    "Provide exactly one extension payload for each profile and schema version.",
                ));
            };
            return Err(super::diagnostic::rejection(
                Code::DuplicateExtension,
                Stage::Bundle,
                Subject::Extension(identity.profile),
                "Provide exactly one extension payload for each profile and schema version.",
            ));
        }
        let shared = CompositionSharedMetadataV1 {
            layout_id: definition.id(),
            definition_revision: definition.revision(),
            core_schema_version: definition.schema_version(),
            compatibility,
        };
        let identities = extension_payloads
            .iter()
            .map(|payload| payload.identity().clone())
            .collect::<Vec<_>>();
        let core_digest_source = encode_canonical(
            &CoreDigestInput {
                digest_schema: "runenwerk.ui_composition.core_digest.v1",
                shared: &shared,
                extension_identities: &identities,
                definition: &definition,
            },
            Subject::Layout(definition.id()),
        )?;
        let core_payload_digest = CompositionDigest::hash(core_digest_source.as_bytes());
        let mut links = Vec::with_capacity(extension_payloads.len());
        let mut extensions = Vec::with_capacity(extension_payloads.len());
        for payload in extension_payloads {
            let digest_source = encode_canonical(
                &ExtensionDigestInput {
                    digest_schema: "runenwerk.ui_composition.extension_digest.v1",
                    shared: &shared,
                    identity: payload.identity(),
                    payload_ron: payload.canonical_ron(),
                },
                Subject::Extension(payload.identity().profile.clone()),
            )?;
            let link = CompositionExtensionLinkV1 {
                identity: payload.identity().clone(),
                core_payload_digest: core_payload_digest.clone(),
                extension_payload_digest: CompositionDigest::hash(digest_source.as_bytes()),
            };
            links.push(link.clone());
            extensions.push(CompositionExtensionEnvelopeV1 {
                envelope_schema_version: CompositionExtensionEnvelopeV1::ENVELOPE_SCHEMA_VERSION,
                shared: shared.clone(),
                link,
                payload_ron: payload.canonical_ron().to_owned(),
            });
        }
        let core = CompositionCoreEnvelopeV1 {
            envelope_schema_version: CompositionCoreEnvelopeV1::ENVELOPE_SCHEMA_VERSION,
            shared,
            core_payload_digest,
            extension_links: links,
            definition,
        };
        let generation_id = generation_id(&core, &extensions)?;
        Ok(Self {
            core,
            extensions,
            generation_id,
        })
    }

    pub fn core(&self) -> &CompositionCoreEnvelopeV1 {
        &self.core
    }

    pub fn extensions(&self) -> &[CompositionExtensionEnvelopeV1] {
        &self.extensions
    }

    pub fn generation_id(&self) -> &CompositionGenerationId {
        &self.generation_id
    }

    pub fn validate(
        &self,
        requirement: Option<&CompositionCompatibilityRequirement>,
    ) -> Result<ValidatedCompositionBundle, CompositionPersistenceRejection> {
        validate_envelopes(&self.core, &self.extensions, requirement)
    }

    pub fn into_validated(self) -> ValidatedCompositionBundle {
        ValidatedCompositionBundle {
            core: self.core,
            extensions: self.extensions,
            generation_id: self.generation_id,
        }
    }
}

impl ValidatedCompositionBundle {
    pub fn from_envelopes(
        core: CompositionCoreEnvelopeV1,
        extensions: Vec<CompositionExtensionEnvelopeV1>,
        requirement: Option<&CompositionCompatibilityRequirement>,
    ) -> Result<Self, CompositionPersistenceRejection> {
        validate_envelopes(&core, &extensions, requirement)
    }

    pub fn core(&self) -> &CompositionCoreEnvelopeV1 {
        &self.core
    }

    pub fn extensions(&self) -> &[CompositionExtensionEnvelopeV1] {
        &self.extensions
    }

    pub fn generation_id(&self) -> &CompositionGenerationId {
        &self.generation_id
    }
}

fn validate_envelopes(
    core: &CompositionCoreEnvelopeV1,
    extensions: &[CompositionExtensionEnvelopeV1],
    requirement: Option<&CompositionCompatibilityRequirement>,
) -> Result<ValidatedCompositionBundle, CompositionPersistenceRejection> {
    let mut diagnostics = Vec::new();
    if core.envelope_schema_version != CompositionCoreEnvelopeV1::ENVELOPE_SCHEMA_VERSION
        || core.shared.core_schema_version != CompositionDefinitionV1::SCHEMA_VERSION
    {
        diagnostics.push(Record::error(
            Code::UnsupportedSchema,
            Stage::Bundle,
            Subject::Layout(core.shared.layout_id),
            "Load only composition core envelope version 1 with definition schema version 1.",
        ));
    }
    if core.shared.layout_id != core.definition.id()
        || core.shared.definition_revision != core.definition.revision()
    {
        diagnostics.push(Record::error(
            Code::SharedMetadataMismatch,
            Stage::Bundle,
            Subject::Layout(core.shared.layout_id),
            "Match the core envelope layout ID and revision to its definition payload.",
        ));
    }
    if let Some(requirement) = requirement
        && !core.shared.compatibility.accepts(requirement)
    {
        diagnostics.push(Record::error(
            Code::CompatibilityMismatch,
            Stage::Compatibility,
            Subject::Layout(core.shared.layout_id),
            "Select a layout bundle compatible with the current app profile and schema version.",
        ));
    }
    let link_by_identity = core
        .extension_links
        .iter()
        .map(|link| (link.identity.clone(), link))
        .collect::<BTreeMap<_, _>>();
    if link_by_identity.len() != core.extension_links.len() {
        diagnostics.push(Record::error(
            Code::DuplicateExtension,
            Stage::Bundle,
            Subject::Layout(core.shared.layout_id),
            "Keep exactly one sorted extension link per profile and schema version.",
        ));
    }
    let extension_identities = extensions
        .iter()
        .map(|extension| extension.link.identity.clone())
        .collect::<BTreeSet<_>>();
    for identity in link_by_identity.keys() {
        if !extension_identities.contains(identity) {
            diagnostics.push(Record::error(
                Code::MissingExtension,
                Stage::Bundle,
                Subject::Extension(identity.profile.clone()),
                "Provide the extension document named by the core envelope link.",
            ));
        }
    }
    for extension in extensions {
        let identity = &extension.link.identity;
        let Some(expected_link) = link_by_identity.get(identity) else {
            diagnostics.push(Record::error(
                Code::ExtraExtension,
                Stage::Bundle,
                Subject::Extension(identity.profile.clone()),
                "Remove extension documents not declared by the core envelope.",
            ));
            continue;
        };
        if extension.envelope_schema_version
            != CompositionExtensionEnvelopeV1::ENVELOPE_SCHEMA_VERSION
        {
            diagnostics.push(Record::error(
                Code::UnsupportedSchema,
                Stage::Bundle,
                Subject::Extension(identity.profile.clone()),
                "Load only composition extension envelope version 1.",
            ));
        }
        if extension.shared != core.shared {
            diagnostics.push(Record::error(
                Code::SharedMetadataMismatch,
                Stage::Bundle,
                Subject::Extension(identity.profile.clone()),
                "Match layout, revision, core schema, and compatibility metadata across the bundle.",
            ));
        }
        if &extension.link != *expected_link
            || extension.link.core_payload_digest != core.core_payload_digest
        {
            diagnostics.push(Record::error(
                Code::LinkMismatch,
                Stage::Bundle,
                Subject::Extension(identity.profile.clone()),
                "Match the extension link and core payload digest exactly.",
            ));
        }
        if let Err(rejection) =
            super::canonical::validate_extension_payload(&extension.payload_ron, &identity.profile)
        {
            diagnostics.extend(rejection.diagnostics().iter().cloned());
        }
    }
    if !diagnostics.is_empty() {
        return Err(CompositionPersistenceRejection::new(diagnostics));
    }
    let payloads = extensions
        .iter()
        .map(|extension| {
            CanonicalExtensionPayload::new(
                extension.link.identity.profile.clone(),
                extension.link.identity.schema_version,
                extension.payload_ron.clone(),
            )
        })
        .collect::<Result<Vec<_>, _>>()?;
    let rebuilt = CompositionBundleCandidate::form(
        core.definition.clone(),
        core.shared.compatibility.clone(),
        payloads,
    )?;
    if rebuilt.core != *core || rebuilt.extensions != extensions {
        return Err(super::diagnostic::rejection(
            Code::DigestMismatch,
            Stage::Digest,
            Subject::Layout(core.shared.layout_id),
            "Recreate the bundle because canonical payload or link digests do not match.",
        ));
    }
    Ok(ValidatedCompositionBundle {
        core: core.clone(),
        extensions: extensions.to_vec(),
        generation_id: rebuilt.generation_id,
    })
}

fn generation_id(
    core: &CompositionCoreEnvelopeV1,
    extensions: &[CompositionExtensionEnvelopeV1],
) -> Result<CompositionGenerationId, CompositionPersistenceRejection> {
    let mut bytes = Vec::new();
    append_record(
        &mut bytes,
        CanonicalCompositionDocuments::core_envelope(core)?.as_bytes(),
    );
    for extension in extensions {
        append_record(
            &mut bytes,
            CanonicalCompositionDocuments::extension_envelope(extension)?.as_bytes(),
        );
    }
    Ok(CompositionGenerationId::from_digest(
        CompositionDigest::hash(&bytes),
    ))
}

fn append_record(output: &mut Vec<u8>, record: &[u8]) {
    output.extend_from_slice(&(record.len() as u64).to_le_bytes());
    output.extend_from_slice(record);
}
