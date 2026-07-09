use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::{
    CompositionDefinitionId, CompositionDiagnosticCode, CompositionFixtureId, CompositionRootId,
    CompositionTransactionId, MountedUnitId, PresentationTargetId, RegionId,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum CompositionDiagnosticSeverity {
    Info,
    Warning,
    Error,
    Fatal,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum CompositionDiagnosticStage {
    Identity,
    Formation,
    Policy,
    Transaction,
    History,
    Promotion,
    Fixture,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum CompositionDiagnosticSubject {
    Definition(CompositionDefinitionId),
    Target(PresentationTargetId),
    Root(CompositionRootId),
    Region(RegionId),
    MountedUnit(MountedUnitId),
    Transaction(CompositionTransactionId),
    Fixture(CompositionFixtureId),
    General(String),
}

impl CompositionDiagnosticSubject {
    pub fn kind(&self) -> &'static str {
        match self {
            Self::Definition(_) => "composition_definition",
            Self::Target(_) => "presentation_target",
            Self::Root(_) => "composition_root",
            Self::Region(_) => "region",
            Self::MountedUnit(_) => "mounted_unit",
            Self::Transaction(_) => "composition_transaction",
            Self::Fixture(_) => "composition_fixture",
            Self::General(_) => "composition",
        }
    }

    pub fn canonical_id(&self) -> String {
        match self {
            Self::Definition(value) => value.to_string(),
            Self::Target(value) => value.to_string(),
            Self::Root(value) => value.to_string(),
            Self::Region(value) => value.to_string(),
            Self::MountedUnit(value) => value.to_string(),
            Self::Transaction(value) => value.to_string(),
            Self::Fixture(value) => value.to_string(),
            Self::General(value) => value.clone(),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CompositionDiagnosticRecord {
    code: CompositionDiagnosticCode,
    severity: CompositionDiagnosticSeverity,
    stage: CompositionDiagnosticStage,
    subject: CompositionDiagnosticSubject,
    message: String,
    #[serde(deserialize_with = "deserialize_context")]
    context: BTreeMap<String, String>,
}

impl CompositionDiagnosticRecord {
    pub fn new(
        code: CompositionDiagnosticCode,
        severity: CompositionDiagnosticSeverity,
        stage: CompositionDiagnosticStage,
        subject: CompositionDiagnosticSubject,
        message: impl Into<String>,
    ) -> Self {
        Self {
            code,
            severity,
            stage,
            subject,
            message: message.into(),
            context: BTreeMap::new(),
        }
    }

    pub fn error(
        code: CompositionDiagnosticCode,
        stage: CompositionDiagnosticStage,
        subject: CompositionDiagnosticSubject,
        message: impl Into<String>,
    ) -> Self {
        Self::new(
            code,
            CompositionDiagnosticSeverity::Error,
            stage,
            subject,
            message,
        )
    }

    pub fn try_with_context(
        mut self,
        key: impl Into<String>,
        value: impl Into<String>,
    ) -> Result<Self, diagnostics::DiagnosticMetadataKeyError> {
        let key = key.into();
        diagnostics::DiagnosticMetadataKey::new(key.clone())?;
        self.context.insert(key, value.into());
        Ok(self)
    }

    pub(crate) fn with_context(mut self, key: &'static str, value: impl Into<String>) -> Self {
        debug_assert!(diagnostics::DiagnosticMetadataKey::from_static(key).is_ok());
        self.context.insert(key.to_owned(), value.into());
        self
    }

    pub const fn code(&self) -> CompositionDiagnosticCode {
        self.code
    }
    pub const fn severity(&self) -> CompositionDiagnosticSeverity {
        self.severity
    }
    pub const fn stage(&self) -> CompositionDiagnosticStage {
        self.stage
    }
    pub fn subject(&self) -> &CompositionDiagnosticSubject {
        &self.subject
    }
    pub fn message(&self) -> &str {
        self.message.as_str()
    }
    pub fn context(&self) -> &BTreeMap<String, String> {
        &self.context
    }

    pub fn to_ratification_issue(
        &self,
    ) -> ratification::RatificationIssue<CompositionDiagnosticCode, CompositionDiagnosticSubject>
    {
        ratification::RatificationIssue::new(
            self.code,
            self.subject.clone(),
            match self.severity {
                CompositionDiagnosticSeverity::Info => ratification::RatificationSeverity::Info,
                CompositionDiagnosticSeverity::Warning => {
                    ratification::RatificationSeverity::Warning
                }
                CompositionDiagnosticSeverity::Error => ratification::RatificationSeverity::Error,
                CompositionDiagnosticSeverity::Fatal => ratification::RatificationSeverity::Fatal,
            },
            self.message.clone(),
        )
    }

    pub fn to_foundation_diagnostic(&self) -> diagnostics::Diagnostic {
        use diagnostics::{
            Diagnostic, DiagnosticCode, DiagnosticDomain, DiagnosticMessage,
            DiagnosticMetadataEntry, DiagnosticMetadataKey, DiagnosticMetadataValue,
            DiagnosticSubject, DiagnosticSubjectId, DiagnosticSubjectKind, Severity,
        };
        let severity = match self.severity {
            CompositionDiagnosticSeverity::Info => Severity::Info,
            CompositionDiagnosticSeverity::Warning => Severity::Warning,
            CompositionDiagnosticSeverity::Error => Severity::Error,
            CompositionDiagnosticSeverity::Fatal => Severity::Fatal,
        };
        let subject = DiagnosticSubject::new(DiagnosticSubjectKind::from_static_unchecked(
            self.subject.kind(),
        ));
        let subject = match DiagnosticSubjectId::new(self.subject.canonical_id()) {
            Ok(id) => subject.with_id(id),
            Err(_) => subject,
        };
        let mut diagnostic = Diagnostic::new(
            severity,
            DiagnosticCode::from_static_unchecked(self.code.as_str()),
            DiagnosticDomain::from_static_unchecked("ui_composition"),
            DiagnosticMessage::new(self.message.clone()),
        )
        .with_subject(subject);
        if let Ok(stage_key) = DiagnosticMetadataKey::from_static("stage") {
            diagnostic.push_metadata(DiagnosticMetadataEntry::new(
                stage_key,
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
        CompositionDiagnosticStage,
        CompositionDiagnosticSeverity,
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

impl PartialOrd for CompositionDiagnosticRecord {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for CompositionDiagnosticRecord {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.sort_key().cmp(&other.sort_key())
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CompositionRejection {
    diagnostics: Vec<CompositionDiagnosticRecord>,
}

impl CompositionRejection {
    pub fn new(mut diagnostics: Vec<CompositionDiagnosticRecord>) -> Self {
        diagnostics.sort();
        diagnostics.dedup();
        Self { diagnostics }
    }

    pub fn single(diagnostic: CompositionDiagnosticRecord) -> Self {
        Self::new(vec![diagnostic])
    }
    pub fn diagnostics(&self) -> &[CompositionDiagnosticRecord] {
        &self.diagnostics
    }
}

impl core::fmt::Display for CompositionRejection {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(
            formatter,
            "composition rejected with {} diagnostic(s)",
            self.diagnostics.len()
        )
    }
}

impl std::error::Error for CompositionRejection {}

fn deserialize_context<'de, D>(deserializer: D) -> Result<BTreeMap<String, String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let context = BTreeMap::<String, String>::deserialize(deserializer)?;
    for key in context.keys() {
        diagnostics::DiagnosticMetadataKey::new(key.clone()).map_err(|error| {
            serde::de::Error::custom(format_args!(
                "invalid composition diagnostic context key {key:?}: {error:?}"
            ))
        })?;
    }
    Ok(context)
}
