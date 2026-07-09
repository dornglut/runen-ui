use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{CompositionDefinitionId, ExtensionProfileId};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum CompositionPersistenceDiagnosticCode {
    CanonicalEncodeFailed,
    CanonicalDecodeFailed,
    NonCanonicalDocument,
    InvalidDigest,
    DigestMismatch,
    InvalidVersion,
    InvalidCompatibility,
    CompatibilityMismatch,
    MissingExtension,
    ExtraExtension,
    DuplicateExtension,
    LinkMismatch,
    SharedMetadataMismatch,
    UnsupportedLegacySchema,
    UnsupportedSchema,
    StaleGeneration,
    InvalidPathComponent,
    StagingFailed,
    WriteFailed,
    SyncFailed,
    ReadbackFailed,
    GenerationCommitFailed,
    PointerWriteFailed,
    PointerCommitFailed,
    ActiveGenerationCorrupt,
    LastGoodRecovered,
    NoValidGeneration,
    ScopeSelectionFailed,
}

impl CompositionPersistenceDiagnosticCode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::CanonicalEncodeFailed => "composition_persistence.canonical.encode_failed",
            Self::CanonicalDecodeFailed => "composition_persistence.canonical.decode_failed",
            Self::NonCanonicalDocument => "composition_persistence.canonical.non_canonical",
            Self::InvalidDigest => "composition_persistence.digest.invalid",
            Self::DigestMismatch => "composition_persistence.digest.mismatch",
            Self::InvalidVersion => "composition_persistence.schema.invalid_version",
            Self::InvalidCompatibility => "composition_persistence.compatibility.invalid",
            Self::CompatibilityMismatch => "composition_persistence.compatibility.mismatch",
            Self::MissingExtension => "composition_persistence.extension.missing",
            Self::ExtraExtension => "composition_persistence.extension.extra",
            Self::DuplicateExtension => "composition_persistence.extension.duplicate",
            Self::LinkMismatch => "composition_persistence.extension.link_mismatch",
            Self::SharedMetadataMismatch => {
                "composition_persistence.bundle.shared_metadata_mismatch"
            }
            Self::UnsupportedLegacySchema => "composition_persistence.schema.unsupported_legacy",
            Self::UnsupportedSchema => "composition_persistence.schema.unsupported",
            Self::StaleGeneration => "composition_persistence.generation.stale_expected",
            Self::InvalidPathComponent => "composition_persistence.path.invalid_component",
            Self::StagingFailed => "composition_persistence.storage.staging_failed",
            Self::WriteFailed => "composition_persistence.storage.write_failed",
            Self::SyncFailed => "composition_persistence.storage.sync_failed",
            Self::ReadbackFailed => "composition_persistence.storage.readback_failed",
            Self::GenerationCommitFailed => {
                "composition_persistence.storage.generation_commit_failed"
            }
            Self::PointerWriteFailed => "composition_persistence.storage.pointer_write_failed",
            Self::PointerCommitFailed => "composition_persistence.storage.pointer_commit_failed",
            Self::ActiveGenerationCorrupt => "composition_persistence.recovery.active_corrupt",
            Self::LastGoodRecovered => "composition_persistence.recovery.last_good",
            Self::NoValidGeneration => "composition_persistence.recovery.no_valid_generation",
            Self::ScopeSelectionFailed => "composition_persistence.scope.selection_failed",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum CompositionPersistenceDiagnosticStage {
    Canonical,
    Digest,
    Compatibility,
    Bundle,
    Promotion,
    Storage,
    Recovery,
    Scope,
    Legacy,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum CompositionPersistenceDiagnosticSubject {
    Layout(CompositionDefinitionId),
    Extension(ExtensionProfileId),
    Generation(String),
    Path(String),
    General(String),
}

impl CompositionPersistenceDiagnosticSubject {
    fn kind(&self) -> &'static str {
        match self {
            Self::Layout(_) => "composition_layout",
            Self::Extension(_) => "composition_extension",
            Self::Generation(_) => "composition_generation",
            Self::Path(_) => "composition_path",
            Self::General(_) => "composition_persistence",
        }
    }

    fn canonical_id(&self) -> String {
        match self {
            Self::Layout(value) => value.to_string(),
            Self::Extension(value) => value.to_string(),
            Self::Generation(value) | Self::Path(value) | Self::General(value) => value.clone(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompositionPersistenceDiagnosticRecord {
    code: CompositionPersistenceDiagnosticCode,
    severity: crate::CompositionDiagnosticSeverity,
    stage: CompositionPersistenceDiagnosticStage,
    subject: CompositionPersistenceDiagnosticSubject,
    message: String,
    context: BTreeMap<String, String>,
}

impl CompositionPersistenceDiagnosticRecord {
    pub fn error(
        code: CompositionPersistenceDiagnosticCode,
        stage: CompositionPersistenceDiagnosticStage,
        subject: CompositionPersistenceDiagnosticSubject,
        message: impl Into<String>,
    ) -> Self {
        Self {
            code,
            severity: crate::CompositionDiagnosticSeverity::Error,
            stage,
            subject,
            message: message.into(),
            context: BTreeMap::new(),
        }
    }

    pub fn warning(
        code: CompositionPersistenceDiagnosticCode,
        stage: CompositionPersistenceDiagnosticStage,
        subject: CompositionPersistenceDiagnosticSubject,
        message: impl Into<String>,
    ) -> Self {
        Self {
            code,
            severity: crate::CompositionDiagnosticSeverity::Warning,
            stage,
            subject,
            message: message.into(),
            context: BTreeMap::new(),
        }
    }

    pub fn with_context(mut self, key: &'static str, value: impl Into<String>) -> Self {
        debug_assert!(diagnostics::DiagnosticMetadataKey::from_static(key).is_ok());
        self.context.insert(key.to_owned(), value.into());
        self
    }

    pub const fn code(&self) -> CompositionPersistenceDiagnosticCode {
        self.code
    }

    pub const fn severity(&self) -> crate::CompositionDiagnosticSeverity {
        self.severity
    }

    pub const fn stage(&self) -> CompositionPersistenceDiagnosticStage {
        self.stage
    }

    pub fn subject(&self) -> &CompositionPersistenceDiagnosticSubject {
        &self.subject
    }

    pub fn message(&self) -> &str {
        &self.message
    }

    pub fn context(&self) -> &BTreeMap<String, String> {
        &self.context
    }

    pub fn to_foundation_diagnostic(&self) -> diagnostics::Diagnostic {
        use diagnostics::{
            Diagnostic, DiagnosticCode, DiagnosticDomain, DiagnosticMessage,
            DiagnosticMetadataEntry, DiagnosticMetadataKey, DiagnosticMetadataValue,
            DiagnosticSubject, DiagnosticSubjectId, DiagnosticSubjectKind, Severity,
        };
        let severity = match self.severity {
            crate::CompositionDiagnosticSeverity::Info => Severity::Info,
            crate::CompositionDiagnosticSeverity::Warning => Severity::Warning,
            crate::CompositionDiagnosticSeverity::Error => Severity::Error,
            crate::CompositionDiagnosticSeverity::Fatal => Severity::Fatal,
        };
        let mut subject = DiagnosticSubject::new(DiagnosticSubjectKind::from_static_unchecked(
            self.subject.kind(),
        ));
        if let Ok(id) = DiagnosticSubjectId::new(self.subject.canonical_id()) {
            subject = subject.with_id(id);
        }
        let mut diagnostic = Diagnostic::new(
            severity,
            DiagnosticCode::from_static_unchecked(self.code.as_str()),
            DiagnosticDomain::from_static_unchecked("composition_persistence"),
            DiagnosticMessage::new(self.message.clone()),
        )
        .with_subject(subject);
        if let Ok(key) = DiagnosticMetadataKey::from_static("stage") {
            diagnostic.push_metadata(DiagnosticMetadataEntry::new(
                key,
                DiagnosticMetadataValue::string(format!("{:?}", self.stage).to_ascii_lowercase()),
            ));
        }
        for (key, value) in &self.context {
            if let Ok(key) = DiagnosticMetadataKey::new(key.clone()) {
                diagnostic.push_metadata(DiagnosticMetadataEntry::new(
                    key,
                    DiagnosticMetadataValue::string(value.clone()),
                ));
            }
        }
        diagnostic
    }

    fn sort_key(
        &self,
    ) -> (
        CompositionPersistenceDiagnosticStage,
        crate::CompositionDiagnosticSeverity,
        &'static str,
        &'static str,
        String,
    ) {
        (
            self.stage,
            self.severity,
            self.code.as_str(),
            self.subject.kind(),
            self.subject.canonical_id(),
        )
    }
}

impl PartialOrd for CompositionPersistenceDiagnosticRecord {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for CompositionPersistenceDiagnosticRecord {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.sort_key().cmp(&other.sort_key())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompositionPersistenceRejection {
    pub(crate) diagnostics: Vec<CompositionPersistenceDiagnosticRecord>,
}

impl CompositionPersistenceRejection {
    pub fn new(mut diagnostics: Vec<CompositionPersistenceDiagnosticRecord>) -> Self {
        diagnostics.sort();
        diagnostics.dedup();
        Self { diagnostics }
    }

    pub fn single(diagnostic: CompositionPersistenceDiagnosticRecord) -> Self {
        Self::new(vec![diagnostic])
    }

    pub fn diagnostics(&self) -> &[CompositionPersistenceDiagnosticRecord] {
        &self.diagnostics
    }
}

impl std::fmt::Display for CompositionPersistenceRejection {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "composition persistence rejected with {} diagnostic(s)",
            self.diagnostics.len()
        )
    }
}

impl std::error::Error for CompositionPersistenceRejection {}

pub(crate) fn rejection(
    code: CompositionPersistenceDiagnosticCode,
    stage: CompositionPersistenceDiagnosticStage,
    subject: CompositionPersistenceDiagnosticSubject,
    message: impl Into<String>,
) -> CompositionPersistenceRejection {
    CompositionPersistenceRejection::single(CompositionPersistenceDiagnosticRecord::error(
        code, stage, subject, message,
    ))
}
