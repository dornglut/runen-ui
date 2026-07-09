use serde::{Deserialize, Serialize};

use crate::{
    CanonicalExtensionPayload, CompositionBundleCandidate, CompositionCompatibility,
    CompositionDefinitionId, CompositionDefinitionV1, CompositionLayoutScope,
    CompositionPersistenceDiagnosticCode as Code, CompositionPersistenceDiagnosticStage as Stage,
    CompositionPersistenceDiagnosticSubject as Subject, CompositionPersistenceRejection,
    CompositionState, StateRevision,
};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct LayoutDisplayName(String);

impl LayoutDisplayName {
    pub fn new(value: impl Into<String>) -> Result<Self, CompositionPersistenceRejection> {
        let value = value.into();
        let trimmed = value.trim();
        if trimmed.is_empty() || trimmed.len() > 160 || trimmed.chars().any(char::is_control) {
            return Err(
                crate::persistence::prelude::CompositionPersistenceRejection::single(
                    crate::persistence::prelude::CompositionPersistenceDiagnosticRecord::error(
                        Code::SharedMetadataMismatch,
                        Stage::Promotion,
                        Subject::General("layout_display_name".to_owned()),
                        "Use a non-empty layout name of at most 160 non-control characters.",
                    ),
                ),
            );
        }
        Ok(Self(trimmed.to_owned()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl TryFrom<String> for LayoutDisplayName {
    type Error = CompositionPersistenceRejection;

    fn try_from(value: String) -> Result<Self, Self::Error> {
        Self::new(value)
    }
}

impl From<LayoutDisplayName> for String {
    fn from(value: LayoutDisplayName) -> Self {
        value.0
    }
}

pub trait CompositionExtensionSnapshotPort {
    fn snapshot_extensions(
        &self,
        layout_id: CompositionDefinitionId,
        source_revision: StateRevision,
    ) -> Result<Vec<CanonicalExtensionPayload>, CompositionPersistenceRejection>;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LayoutPromotion {
    source_revision: StateRevision,
    candidate: CompositionDefinitionV1,
    display_name: LayoutDisplayName,
    scope: CompositionLayoutScope,
    compatibility: CompositionCompatibility,
}

impl LayoutPromotion {
    pub const fn source_revision(&self) -> StateRevision {
        self.source_revision
    }
    pub fn candidate(&self) -> &CompositionDefinitionV1 {
        &self.candidate
    }
    pub fn into_candidate(self) -> CompositionDefinitionV1 {
        self.candidate
    }

    pub fn display_name(&self) -> &LayoutDisplayName {
        &self.display_name
    }

    pub const fn scope(&self) -> CompositionLayoutScope {
        self.scope
    }

    pub fn compatibility(&self) -> &CompositionCompatibility {
        &self.compatibility
    }

    pub fn form_bundle(
        &self,
        extensions: Vec<CanonicalExtensionPayload>,
    ) -> Result<CompositionBundleCandidate, CompositionPersistenceRejection> {
        CompositionBundleCandidate::form(
            self.candidate.clone(),
            self.compatibility.clone(),
            extensions,
        )
    }

    pub fn snapshot_bundle(
        &self,
        snapshots: &dyn CompositionExtensionSnapshotPort,
    ) -> Result<CompositionBundleCandidate, CompositionPersistenceRejection> {
        let extensions =
            snapshots.snapshot_extensions(self.candidate.id(), self.source_revision)?;
        self.form_bundle(extensions)
    }
}

impl CompositionState {
    pub fn promote_definition(
        &self,
        id: CompositionDefinitionId,
        display_name: LayoutDisplayName,
        scope: CompositionLayoutScope,
        compatibility: CompositionCompatibility,
    ) -> LayoutPromotion {
        let mut candidate = self.definition.clone();
        candidate.set_id(id);
        candidate.set_revision(crate::DefinitionRevision::new(self.revision.raw()));
        LayoutPromotion {
            source_revision: self.revision,
            candidate,
            display_name,
            scope,
            compatibility,
        }
    }
}
