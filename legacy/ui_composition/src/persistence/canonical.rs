use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use crate::{
    CompositionDefinitionV1, CompositionFixture, CompositionFixtureId, CompositionHistory,
    CompositionJournalEntry, ExtensionProfileId,
};

use super::{
    CompositionCoreEnvelopeV1, CompositionExtensionEnvelopeV1, CompositionGenerationPointerV1,
    CompositionPersistenceDiagnosticCode as Code, CompositionPersistenceDiagnosticStage as Stage,
    CompositionPersistenceDiagnosticSubject as Subject, CompositionPersistenceRejection,
};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompositionFixtureCatalogV1 {
    schema_version: u16,
    fixtures: Vec<CompositionFixture>,
}

impl CompositionFixtureCatalogV1 {
    pub const SCHEMA_VERSION: u16 = 1;

    pub fn new(mut fixtures: Vec<CompositionFixture>) -> Self {
        fixtures.sort_by_key(|fixture| fixture.id);
        Self {
            schema_version: Self::SCHEMA_VERSION,
            fixtures,
        }
    }

    pub fn fixtures(&self) -> &[CompositionFixture] {
        &self.fixtures
    }

    pub fn fixture(&self, id: CompositionFixtureId) -> Option<&CompositionFixture> {
        self.fixtures.iter().find(|fixture| fixture.id == id)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CompositionJournalDocumentV1 {
    schema_version: u16,
    entries: Vec<CompositionJournalEntry>,
}

impl CompositionJournalDocumentV1 {
    pub const SCHEMA_VERSION: u16 = 1;

    pub fn from_history(history: &CompositionHistory) -> Self {
        let mut entries = history.journal().to_vec();
        entries.sort_by_key(|entry| (entry.applied_revision(), entry.transaction().id()));
        Self {
            schema_version: Self::SCHEMA_VERSION,
            entries,
        }
    }

    pub fn entries(&self) -> &[CompositionJournalEntry] {
        &self.entries
    }
}

pub struct CanonicalCompositionDocuments;

impl CanonicalCompositionDocuments {
    pub fn definition(
        definition: &CompositionDefinitionV1,
    ) -> Result<String, CompositionPersistenceRejection> {
        encode_canonical(
            &definition.clone().normalized(),
            Subject::Layout(definition.id()),
        )
    }

    pub fn decode_definition(
        source: &str,
    ) -> Result<CompositionDefinitionV1, CompositionPersistenceRejection> {
        let definition: CompositionDefinitionV1 = decode_canonical(
            source,
            Subject::General("composition_definition".to_owned()),
        )?;
        if definition.clone().normalized() != definition {
            return Err(noncanonical_order("composition_definition"));
        }
        Ok(definition)
    }

    pub fn fixture_catalog(
        catalog: &CompositionFixtureCatalogV1,
    ) -> Result<String, CompositionPersistenceRejection> {
        encode_canonical(
            catalog,
            Subject::General("composition_fixture_catalog".to_owned()),
        )
    }

    pub fn decode_fixture_catalog(
        source: &str,
    ) -> Result<CompositionFixtureCatalogV1, CompositionPersistenceRejection> {
        let catalog: CompositionFixtureCatalogV1 = decode_canonical(
            source,
            Subject::General("composition_fixture_catalog".to_owned()),
        )?;
        let mut sorted = catalog.fixtures.clone();
        sorted.sort_by_key(|fixture| fixture.id);
        if sorted != catalog.fixtures {
            return Err(noncanonical_order("composition_fixture_catalog"));
        }
        Ok(catalog)
    }

    pub fn journal(
        journal: &CompositionJournalDocumentV1,
    ) -> Result<String, CompositionPersistenceRejection> {
        encode_canonical(journal, Subject::General("composition_journal".to_owned()))
    }

    pub fn decode_journal(
        source: &str,
    ) -> Result<CompositionJournalDocumentV1, CompositionPersistenceRejection> {
        let journal: CompositionJournalDocumentV1 =
            decode_canonical(source, Subject::General("composition_journal".to_owned()))?;
        let mut sorted = journal.entries.clone();
        sorted.sort_by_key(|entry| (entry.applied_revision(), entry.transaction().id()));
        if sorted != journal.entries {
            return Err(noncanonical_order("composition_journal"));
        }
        Ok(journal)
    }

    pub fn core_envelope(
        envelope: &CompositionCoreEnvelopeV1,
    ) -> Result<String, CompositionPersistenceRejection> {
        encode_canonical(envelope, Subject::Layout(envelope.shared.layout_id))
    }

    pub fn decode_core_envelope(
        source: &str,
    ) -> Result<CompositionCoreEnvelopeV1, CompositionPersistenceRejection> {
        let envelope: CompositionCoreEnvelopeV1 = decode_canonical(
            source,
            Subject::General("composition_core_envelope".to_owned()),
        )?;
        let mut links = envelope.extension_links.clone();
        links.sort_by(|left, right| left.identity.cmp(&right.identity));
        if envelope.definition.clone().normalized() != envelope.definition
            || links != envelope.extension_links
        {
            return Err(noncanonical_order("composition_core_envelope"));
        }
        Ok(envelope)
    }

    pub fn extension_envelope(
        envelope: &CompositionExtensionEnvelopeV1,
    ) -> Result<String, CompositionPersistenceRejection> {
        encode_canonical(
            envelope,
            Subject::Extension(envelope.link.identity.profile.clone()),
        )
    }

    pub fn decode_extension_envelope(
        source: &str,
    ) -> Result<CompositionExtensionEnvelopeV1, CompositionPersistenceRejection> {
        decode_canonical(
            source,
            Subject::General("composition_extension_envelope".to_owned()),
        )
    }

    pub fn generation_pointer(
        pointer: &CompositionGenerationPointerV1,
    ) -> Result<String, CompositionPersistenceRejection> {
        encode_canonical(
            pointer,
            Subject::General("active_generation_pointer".to_owned()),
        )
    }

    pub fn decode_generation_pointer(
        source: &str,
    ) -> Result<CompositionGenerationPointerV1, CompositionPersistenceRejection> {
        decode_canonical(
            source,
            Subject::General("active_generation_pointer".to_owned()),
        )
    }
}

fn noncanonical_order(subject: &'static str) -> CompositionPersistenceRejection {
    super::diagnostic::rejection(
        Code::NonCanonicalDocument,
        Stage::Canonical,
        Subject::General(subject.to_owned()),
        "Sort unordered composition records by their canonical typed identities before encoding.",
    )
}

pub(crate) fn encode_canonical<T: Serialize>(
    value: &T,
    subject: Subject,
) -> Result<String, CompositionPersistenceRejection> {
    let config = ron::ser::PrettyConfig::new()
        .new_line("\n".to_owned())
        .indentor("  ".to_owned())
        .separator(" ".to_owned())
        .struct_names(false)
        .separate_tuple_members(false)
        .enumerate_arrays(false)
        .compact_arrays(false);
    let mut source = ron::ser::to_string_pretty(value, config).map_err(|error| {
        super::diagnostic::rejection(
            Code::CanonicalEncodeFailed,
            Stage::Canonical,
            subject.clone(),
            "Fix the composition document before canonical encoding.",
        )
        .with_context("source_error", error.to_string())
    })?;
    while source.ends_with('\n') {
        source.pop();
    }
    source.push('\n');
    Ok(source)
}

pub(crate) fn decode_canonical<T: DeserializeOwned + Serialize>(
    source: &str,
    subject: Subject,
) -> Result<T, CompositionPersistenceRejection> {
    let value = ron::from_str::<T>(source).map_err(|error| {
        super::diagnostic::rejection(
            Code::CanonicalDecodeFailed,
            Stage::Canonical,
            subject.clone(),
            "Use the supported canonical composition RON schema.",
        )
        .with_context("source_error", error.to_string())
    })?;
    let canonical = encode_canonical(&value, subject.clone())?;
    if canonical != source {
        return Err(super::diagnostic::rejection(
            Code::NonCanonicalDocument,
            Stage::Canonical,
            subject,
            "Re-encode the document with CanonicalCompositionDocuments before loading it.",
        ));
    }
    Ok(value)
}

pub(crate) fn validate_extension_payload(
    source: &str,
    profile: &ExtensionProfileId,
) -> Result<(), CompositionPersistenceRejection> {
    let subject = Subject::Extension(profile.clone());
    if source.is_empty()
        || source.contains('\r')
        || !source.ends_with('\n')
        || source.ends_with("\n\n")
        || has_comment_outside_string(source)
    {
        return Err(super::diagnostic::rejection(
            Code::NonCanonicalDocument,
            Stage::Canonical,
            subject,
            "Provide canonical UTF-8 extension RON with LF endings, no comments, and one trailing newline.",
        ));
    }
    ron::from_str::<ron::Value>(source).map_err(|error| {
        super::diagnostic::rejection(
            Code::CanonicalDecodeFailed,
            Stage::Canonical,
            subject,
            "Provide a syntactically valid typed extension RON payload.",
        )
        .with_context("source_error", error.to_string())
    })?;
    Ok(())
}

fn has_comment_outside_string(source: &str) -> bool {
    let bytes = source.as_bytes();
    let mut index = 0;
    let mut in_string = false;
    let mut escaped = false;
    while index < bytes.len() {
        let byte = bytes[index];
        if in_string {
            if escaped {
                escaped = false;
            } else if byte == b'\\' {
                escaped = true;
            } else if byte == b'"' {
                in_string = false;
            }
        } else if byte == b'"' {
            in_string = true;
        } else if byte == b'/' && index + 1 < bytes.len() && matches!(bytes[index + 1], b'/' | b'*')
        {
            return true;
        }
        index += 1;
    }
    false
}

trait RejectionContext {
    fn with_context(self, key: &'static str, value: String) -> Self;
}

impl RejectionContext for CompositionPersistenceRejection {
    fn with_context(mut self, key: &'static str, value: String) -> Self {
        if let Some(record) = self.diagnostics.first_mut() {
            *record = record.clone().with_context(key, value);
        }
        self
    }
}
