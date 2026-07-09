//! Stable diagnostics for the UI Story V2 proof model.
//!
//! V2 diagnostics are attached to story, manifest, workflow, evidence, run,
//! report, and mount subjects instead of the old flat stage enum. App-owned
//! proof producers may appear only through `ExternalProducer` origins.

use serde::{Deserialize, Serialize};

use crate::identity::{
    UiStoryEvidenceKey, UiStoryEvidenceProducerId, UiStoryId, UiStoryManifestSourceId,
    UiStoryRunId, UiStoryWorkflowNodeId, UiStoryWorkflowProfileId,
};

pub const UI_STORY_MANIFEST_SCHEMA_UNSUPPORTED: &str = "ui.story.manifest.schema_unsupported";
pub const UI_STORY_MANIFEST_FIELD_MISSING: &str = "ui.story.manifest.field_missing";
pub const UI_STORY_MANIFEST_SOURCE_INVALID: &str = "ui.story.manifest.source_invalid";
pub const UI_STORY_REGISTRY_DUPLICATE_STORY: &str = "ui.story.registry.duplicate_story";
pub const UI_STORY_REGISTRY_INVALID_MANIFEST: &str = "ui.story.registry.invalid_manifest";
pub const UI_STORY_WORKFLOW_PROFILE_UNKNOWN: &str = "ui.story.workflow.profile_unknown";
pub const UI_STORY_WORKFLOW_NODE_DUPLICATE: &str = "ui.story.workflow.node_duplicate";
pub const UI_STORY_WORKFLOW_NODE_MISSING: &str = "ui.story.workflow.node_missing";
pub const UI_STORY_WORKFLOW_EDGE_ENDPOINT_UNKNOWN: &str = "ui.story.workflow.edge_endpoint_unknown";
pub const UI_STORY_WORKFLOW_CYCLE: &str = "ui.story.workflow.cycle";
pub const UI_STORY_RUN_UNKNOWN_STORY: &str = "ui.story.run.unknown_story";
pub const UI_STORY_RUN_MISSING_REQUIRED_EVIDENCE: &str = "ui.story.run.missing_required_evidence";
pub const UI_STORY_RUN_BLOCKED_DEPENDENCY: &str = "ui.story.run.blocked_dependency";
pub const UI_STORY_RUN_DUPLICATE_EVIDENCE: &str = "ui.story.run.duplicate_evidence";
pub const UI_STORY_EXPECTED_FAILURE_TARGET_MISSING: &str =
    "ui.story.expected_failure.target_missing";
pub const UI_STORY_EXPECTED_FAILURE_DIAGNOSTIC_MISSING: &str =
    "ui.story.expected_failure.diagnostic_missing";
pub const UI_STORY_EXPECTED_FAILURE_UNEXPECTED_ERROR: &str =
    "ui.story.expected_failure.unexpected_error";
pub const UI_STORY_MOUNT_BLOCKED_OUTCOME: &str = "ui.story.mount.blocked_outcome";
pub const UI_STORY_MOUNT_BLOCKED_POLICY: &str = "ui.story.mount.blocked_policy";
pub const UI_STORY_MOUNT_BLOCKED_MISSING_PREVIEW: &str = "ui.story.mount.blocked_missing_preview";

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct UiStoryDiagnosticCode(String);

impl UiStoryDiagnosticCode {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn is_empty(&self) -> bool {
        self.0.trim().is_empty()
    }

    pub fn is_valid(&self) -> bool {
        !self.is_empty() && self.0.trim() == self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum UiStoryDiagnosticSeverity {
    Info,
    Warning,
    Error,
    Fatal,
}

impl UiStoryDiagnosticSeverity {
    pub const fn is_blocking(self) -> bool {
        matches!(self, Self::Error | Self::Fatal)
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum UiStoryDiagnosticSubject {
    Story(UiStoryId),
    ManifestSource(UiStoryManifestSourceId),
    WorkflowProfile(UiStoryWorkflowProfileId),
    WorkflowNode(UiStoryWorkflowNodeId),
    Evidence(UiStoryEvidenceKey),
    Producer(UiStoryEvidenceProducerId),
    Run(UiStoryRunId),
    General(String),
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum UiStoryDiagnosticOrigin {
    Manifest,
    Registry,
    Workflow,
    Evidence,
    Runner,
    Report,
    Mount,
    Cli,
    ExternalProducer(UiStoryEvidenceProducerId),
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct UiStoryDiagnostic {
    pub code: UiStoryDiagnosticCode,
    pub severity: UiStoryDiagnosticSeverity,
    pub origin: UiStoryDiagnosticOrigin,
    pub subject: UiStoryDiagnosticSubject,
    pub message: String,
    #[serde(default)]
    pub context: Vec<(String, String)>,
}

impl UiStoryDiagnostic {
    pub fn new(
        code: impl Into<String>,
        severity: UiStoryDiagnosticSeverity,
        origin: UiStoryDiagnosticOrigin,
        subject: UiStoryDiagnosticSubject,
        message: impl Into<String>,
    ) -> Self {
        Self {
            code: UiStoryDiagnosticCode::new(code),
            severity,
            origin,
            subject,
            message: message.into(),
            context: Vec::new(),
        }
    }

    pub fn info(
        code: impl Into<String>,
        origin: UiStoryDiagnosticOrigin,
        subject: UiStoryDiagnosticSubject,
        message: impl Into<String>,
    ) -> Self {
        Self::new(
            code,
            UiStoryDiagnosticSeverity::Info,
            origin,
            subject,
            message,
        )
    }

    pub fn warning(
        code: impl Into<String>,
        origin: UiStoryDiagnosticOrigin,
        subject: UiStoryDiagnosticSubject,
        message: impl Into<String>,
    ) -> Self {
        Self::new(
            code,
            UiStoryDiagnosticSeverity::Warning,
            origin,
            subject,
            message,
        )
    }

    pub fn error(
        code: impl Into<String>,
        origin: UiStoryDiagnosticOrigin,
        subject: UiStoryDiagnosticSubject,
        message: impl Into<String>,
    ) -> Self {
        Self::new(
            code,
            UiStoryDiagnosticSeverity::Error,
            origin,
            subject,
            message,
        )
    }

    pub fn fatal(
        code: impl Into<String>,
        origin: UiStoryDiagnosticOrigin,
        subject: UiStoryDiagnosticSubject,
        message: impl Into<String>,
    ) -> Self {
        Self::new(
            code,
            UiStoryDiagnosticSeverity::Fatal,
            origin,
            subject,
            message,
        )
    }

    pub fn with_context(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.context.push((key.into(), value.into()));
        self
    }

    pub fn is_blocking(&self) -> bool {
        self.severity.is_blocking()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_and_fatal_are_blocking() {
        assert!(UiStoryDiagnosticSeverity::Error.is_blocking());
        assert!(UiStoryDiagnosticSeverity::Fatal.is_blocking());
    }

    #[test]
    fn info_and_warning_are_not_blocking() {
        assert!(!UiStoryDiagnosticSeverity::Info.is_blocking());
        assert!(!UiStoryDiagnosticSeverity::Warning.is_blocking());
    }

    #[test]
    fn diagnostics_sort_deterministically_by_origin_subject_code() {
        let mut diagnostics = vec![
            UiStoryDiagnostic::error(
                UI_STORY_RUN_MISSING_REQUIRED_EVIDENCE,
                UiStoryDiagnosticOrigin::Runner,
                UiStoryDiagnosticSubject::WorkflowNode(UiStoryWorkflowNodeId::new("source_parse")),
                "missing source parse evidence",
            ),
            UiStoryDiagnostic::error(
                UI_STORY_MANIFEST_FIELD_MISSING,
                UiStoryDiagnosticOrigin::Manifest,
                UiStoryDiagnosticSubject::Story(UiStoryId::new("ui.gallery.button.basic")),
                "missing source",
            ),
            UiStoryDiagnostic::warning(
                UI_STORY_MOUNT_BLOCKED_POLICY,
                UiStoryDiagnosticOrigin::Mount,
                UiStoryDiagnosticSubject::Story(UiStoryId::new("ui.gallery.button.basic")),
                "gallery-only story is not product mountable",
            ),
        ];

        diagnostics.sort();

        assert_eq!(
            diagnostics
                .iter()
                .map(|diagnostic| diagnostic.code.as_str())
                .collect::<Vec<_>>(),
            vec![
                UI_STORY_MANIFEST_FIELD_MISSING,
                UI_STORY_MOUNT_BLOCKED_POLICY,
                UI_STORY_RUN_MISSING_REQUIRED_EVIDENCE,
            ]
        );
    }

    #[test]
    fn diagnostic_context_is_preserved_in_order() {
        let diagnostic = UiStoryDiagnostic::error(
            UI_STORY_WORKFLOW_NODE_MISSING,
            UiStoryDiagnosticOrigin::Workflow,
            UiStoryDiagnosticSubject::WorkflowNode(UiStoryWorkflowNodeId::new("source_load")),
            "workflow node is missing",
        )
        .with_context("profile", "ui_story.workflow.static_preview")
        .with_context("node", "source_load");

        assert_eq!(
            diagnostic.context,
            vec![
                (
                    "profile".to_string(),
                    "ui_story.workflow.static_preview".to_string()
                ),
                ("node".to_string(), "source_load".to_string()),
            ]
        );
    }

    #[test]
    fn diagnostic_codes_are_stable_strings() {
        let code = UiStoryDiagnosticCode::new(UI_STORY_WORKFLOW_CYCLE);

        assert_eq!(code.as_str(), "ui.story.workflow.cycle");
        assert!(code.is_valid());
        assert!(!UiStoryDiagnosticCode::new(" ui.story.workflow.cycle ").is_valid());
    }

    #[test]
    fn diagnostic_constructor_sets_blocking_state() {
        let diagnostic = UiStoryDiagnostic::fatal(
            UI_STORY_REGISTRY_INVALID_MANIFEST,
            UiStoryDiagnosticOrigin::Registry,
            UiStoryDiagnosticSubject::ManifestSource(UiStoryManifestSourceId::new(
                "assets/ui_gallery/stories/basic.story.ron",
            )),
            "manifest source failed validation",
        );

        assert_eq!(diagnostic.severity, UiStoryDiagnosticSeverity::Fatal);
        assert!(diagnostic.is_blocking());
    }
}
