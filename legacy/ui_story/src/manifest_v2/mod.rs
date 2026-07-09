//! Manifest V2 for semantic UI story workflow proof runs.
//!
//! Manifest V2 selects a workflow profile instead of enumerating flat proof
//! stages. App-owned proof producers later attach evidence to workflow nodes.

mod builder;
mod expected;
mod schema;
mod source;
mod viewport;

use serde::{Deserialize, Serialize};

use crate::diagnostic::{
    UI_STORY_MANIFEST_FIELD_MISSING, UI_STORY_MANIFEST_SCHEMA_UNSUPPORTED,
    UI_STORY_MANIFEST_SOURCE_INVALID, UiStoryDiagnostic, UiStoryDiagnosticOrigin,
    UiStoryDiagnosticSubject,
};
use crate::identity::{
    UiStoryCategoryId, UiStoryHostProfileId, UiStoryId, UiStoryProgramId, UiStoryRevision,
    UiStoryThemeProfileId, UiStoryWorkflowProfileId,
};

pub use builder::UiStoryManifestBuilder;
pub use expected::{UiStoryExpectedOutcomeV2, UiStoryMountPolicyV2};
pub use schema::{UI_STORY_MANIFEST_V2_SCHEMA_VERSION, UiStoryManifestV2ParseError};
pub use source::{UiStorySourceKindV2, UiStorySourceRef};
pub use viewport::{UiStoryViewportMatrix, UiStoryViewportProfileV2};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct UiStoryManifestV2 {
    pub schema_version: u32,
    pub story_id: UiStoryId,
    pub story_revision: UiStoryRevision,
    pub title: String,
    pub category_id: UiStoryCategoryId,
    pub source: UiStorySourceRef,
    pub program_id: UiStoryProgramId,
    pub host_profile_id: UiStoryHostProfileId,
    pub theme_profile_id: UiStoryThemeProfileId,
    pub viewport_matrix: UiStoryViewportMatrix,
    pub workflow_profile_id: UiStoryWorkflowProfileId,
    pub expected_outcome: UiStoryExpectedOutcomeV2,
    pub mount_policy: UiStoryMountPolicyV2,
}

impl UiStoryManifestV2 {
    pub fn builder(story_id: impl Into<String>) -> UiStoryManifestBuilder {
        UiStoryManifestBuilder::new(story_id)
    }

    pub fn validate(&self) -> Vec<UiStoryDiagnostic> {
        let mut diagnostics = Vec::new();

        if self.schema_version != UI_STORY_MANIFEST_V2_SCHEMA_VERSION {
            diagnostics.push(
                UiStoryDiagnostic::error(
                    UI_STORY_MANIFEST_SCHEMA_UNSUPPORTED,
                    UiStoryDiagnosticOrigin::Manifest,
                    UiStoryDiagnosticSubject::Story(self.story_id.clone()),
                    format!(
                        "unsupported ui story manifest schema version {}; expected {}",
                        self.schema_version, UI_STORY_MANIFEST_V2_SCHEMA_VERSION
                    ),
                )
                .with_context("schema_version", self.schema_version.to_string()),
            );
        }

        push_invalid_id(
            &mut diagnostics,
            self.story_id.is_valid(),
            UiStoryDiagnosticSubject::Story(self.story_id.clone()),
            "story_id",
        );
        push_missing(
            &mut diagnostics,
            !self.title.trim().is_empty() && self.title.trim() == self.title,
            UiStoryDiagnosticSubject::Story(self.story_id.clone()),
            "title",
        );
        push_invalid_id(
            &mut diagnostics,
            self.category_id.is_valid(),
            UiStoryDiagnosticSubject::Story(self.story_id.clone()),
            "category_id",
        );
        push_invalid_id(
            &mut diagnostics,
            self.program_id.is_valid(),
            UiStoryDiagnosticSubject::Story(self.story_id.clone()),
            "program_id",
        );
        push_invalid_id(
            &mut diagnostics,
            self.host_profile_id.is_valid(),
            UiStoryDiagnosticSubject::Story(self.story_id.clone()),
            "host_profile_id",
        );
        push_invalid_id(
            &mut diagnostics,
            self.theme_profile_id.is_valid(),
            UiStoryDiagnosticSubject::Story(self.story_id.clone()),
            "theme_profile_id",
        );
        push_invalid_id(
            &mut diagnostics,
            self.workflow_profile_id.is_valid(),
            UiStoryDiagnosticSubject::WorkflowProfile(self.workflow_profile_id.clone()),
            "workflow_profile_id",
        );

        if !self.source.source_id.is_valid() {
            diagnostics.push(
                UiStoryDiagnostic::error(
                    UI_STORY_MANIFEST_SOURCE_INVALID,
                    UiStoryDiagnosticOrigin::Manifest,
                    UiStoryDiagnosticSubject::ManifestSource(self.source.source_id.clone()),
                    "source_id must be non-empty and must not contain surrounding whitespace",
                )
                .with_context("field", "source.source_id"),
            );
        }
        if !self.source.path_is_valid() {
            diagnostics.push(
                UiStoryDiagnostic::error(
                    UI_STORY_MANIFEST_SOURCE_INVALID,
                    UiStoryDiagnosticOrigin::Manifest,
                    UiStoryDiagnosticSubject::ManifestSource(self.source.source_id.clone()),
                    "source path must be non-empty and must not contain surrounding whitespace",
                )
                .with_context("field", "source.path")
                .with_context("path", self.source.path.clone()),
            );
        }

        diagnostics.extend(self.viewport_matrix.validate(&self.story_id));

        diagnostics
    }

    pub fn from_ron_str(source: &str) -> Result<Self, UiStoryManifestV2ParseError> {
        let manifest = ron::from_str::<Self>(source).map_err(|error| {
            UiStoryManifestV2ParseError::parse_failed(format!(
                "failed to parse ui story manifest v2: {error}"
            ))
        })?;

        if let Some(diagnostic) = manifest
            .validate()
            .into_iter()
            .find(|diagnostic| diagnostic.code.as_str() == UI_STORY_MANIFEST_SCHEMA_UNSUPPORTED)
        {
            let code = diagnostic.code.as_str().to_owned();
            let message = diagnostic.message;
            return Err(UiStoryManifestV2ParseError::new(code, message));
        }

        Ok(manifest)
    }

    pub fn to_ron_string_pretty(&self) -> Result<String, UiStoryManifestV2ParseError> {
        ron::ser::to_string_pretty(self, ron::ser::PrettyConfig::default()).map_err(|error| {
            UiStoryManifestV2ParseError::parse_failed(format!(
                "failed to serialize ui story manifest v2: {error}"
            ))
        })
    }

    pub fn is_valid(&self) -> bool {
        self.validate().is_empty()
    }
}

fn push_missing(
    diagnostics: &mut Vec<UiStoryDiagnostic>,
    valid: bool,
    subject: UiStoryDiagnosticSubject,
    field: &str,
) {
    if !valid {
        diagnostics.push(
            UiStoryDiagnostic::error(
                UI_STORY_MANIFEST_FIELD_MISSING,
                UiStoryDiagnosticOrigin::Manifest,
                subject,
                format!("{field} is required"),
            )
            .with_context("field", field),
        );
    }
}

fn push_invalid_id(
    diagnostics: &mut Vec<UiStoryDiagnostic>,
    valid: bool,
    subject: UiStoryDiagnosticSubject,
    field: &str,
) {
    if !valid {
        diagnostics.push(
            UiStoryDiagnostic::error(
                UI_STORY_MANIFEST_FIELD_MISSING,
                UiStoryDiagnosticOrigin::Manifest,
                subject,
                format!("{field} must be non-empty and must not contain surrounding whitespace"),
            )
            .with_context("field", field),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostic::{UI_STORY_MANIFEST_FIELD_MISSING, UiStoryDiagnosticSeverity};
    use crate::evidence::UiStoryDiagnosticExpectation;
    use crate::workflow::{NODE_SOURCE_LOAD, WORKFLOW_STATIC_PREVIEW};

    const SOURCE_PRODUCER: &str = "runenwerk_editor.ui_gallery.source_loader";
    const SOURCE_KEY: &str = "ui.gallery.source_load";
    const SOURCE_DIAGNOSTIC: &str = "ui_gallery.story.source.read_failed";

    fn valid_manifest() -> UiStoryManifestV2 {
        UiStoryManifestV2::builder("ui.gallery.button.basic")
            .title("Button / Basic")
            .category("controls.button")
            .source_node(
                "assets/ui_gallery/stories/controls/button/basic.ron",
                "ui.gallery.button.basic.source",
            )
            .program_id("ui.gallery.button.basic.program")
            .host_profile("editor.gallery")
            .theme_profile("editor.dark")
            .viewport("default", 320, 128, 1.0)
            .workflow_profile(WORKFLOW_STATIC_PREVIEW)
            .expected_pass()
            .mount_policy(UiStoryMountPolicyV2::EligibleWhenPassed)
            .build()
    }

    fn expected_failure() -> UiStoryDiagnosticExpectation {
        UiStoryDiagnosticExpectation::from_strings(
            NODE_SOURCE_LOAD,
            SOURCE_PRODUCER,
            SOURCE_KEY,
            SOURCE_DIAGNOSTIC,
            UiStoryDiagnosticSeverity::Error,
        )
    }

    #[test]
    fn manifest_v2_valid_static_preview_manifest_has_no_diagnostics() {
        let manifest = valid_manifest();

        assert!(manifest.validate().is_empty());
        assert_eq!(
            manifest.workflow_profile_id.as_str(),
            WORKFLOW_STATIC_PREVIEW
        );
    }

    #[test]
    fn manifest_v2_rejects_empty_story_id() {
        let manifest = UiStoryManifestV2::builder("")
            .title("Button / Basic")
            .category("controls.button")
            .source_node("basic.ron", "ui.gallery.button.basic.source")
            .program_id("ui.gallery.button.basic.program")
            .host_profile("editor.gallery")
            .theme_profile("editor.dark")
            .viewport("default", 320, 128, 1.0)
            .workflow_profile(WORKFLOW_STATIC_PREVIEW)
            .expected_pass()
            .mount_policy(UiStoryMountPolicyV2::EligibleWhenPassed)
            .build();

        assert!(
            manifest
                .validate()
                .iter()
                .any(|diagnostic| diagnostic.code.as_str() == UI_STORY_MANIFEST_FIELD_MISSING)
        );
    }

    #[test]
    fn manifest_v2_rejects_empty_title() {
        let manifest = UiStoryManifestV2::builder("ui.gallery.button.basic")
            .title("")
            .category("controls.button")
            .source_node("basic.ron", "ui.gallery.button.basic.source")
            .program_id("ui.gallery.button.basic.program")
            .host_profile("editor.gallery")
            .theme_profile("editor.dark")
            .viewport("default", 320, 128, 1.0)
            .workflow_profile(WORKFLOW_STATIC_PREVIEW)
            .expected_pass()
            .mount_policy(UiStoryMountPolicyV2::EligibleWhenPassed)
            .build();

        assert!(
            manifest
                .validate()
                .iter()
                .any(|diagnostic| diagnostic.message.contains("title"))
        );
    }

    #[test]
    fn manifest_v2_rejects_missing_source_path() {
        let manifest = UiStoryManifestV2::builder("ui.gallery.button.basic")
            .title("Button / Basic")
            .category("controls.button")
            .source_node("", "ui.gallery.button.basic.source")
            .program_id("ui.gallery.button.basic.program")
            .host_profile("editor.gallery")
            .theme_profile("editor.dark")
            .viewport("default", 320, 128, 1.0)
            .workflow_profile(WORKFLOW_STATIC_PREVIEW)
            .expected_pass()
            .mount_policy(UiStoryMountPolicyV2::EligibleWhenPassed)
            .build();

        assert!(
            manifest
                .validate()
                .iter()
                .any(|diagnostic| diagnostic.message.contains("source path"))
        );
    }

    #[test]
    fn manifest_v2_rejects_empty_viewport_matrix() {
        let mut manifest = valid_manifest();
        manifest.viewport_matrix =
            UiStoryViewportMatrix::new(Vec::<UiStoryViewportProfileV2>::new());

        assert!(
            manifest
                .validate()
                .iter()
                .any(|diagnostic| diagnostic.message.contains("viewport_matrix"))
        );
    }

    #[test]
    fn manifest_v2_expected_failure_preserves_exact_diagnostic_expectation() {
        let expectation = expected_failure();
        let manifest = valid_manifest();
        let manifest = UiStoryManifestV2 {
            expected_outcome: UiStoryExpectedOutcomeV2::expected_failure(expectation.clone()),
            mount_policy: UiStoryMountPolicyV2::Never,
            ..manifest
        };

        match manifest.expected_outcome {
            UiStoryExpectedOutcomeV2::ExpectedFailure {
                expectation: actual,
            } => {
                assert_eq!(actual, expectation);
            }
            UiStoryExpectedOutcomeV2::Pass => panic!("expected failure should be preserved"),
        }
    }

    #[test]
    fn manifest_v2_round_trips_pretty_ron() {
        let manifest = valid_manifest();
        let ron = manifest
            .to_ron_string_pretty()
            .expect("manifest should serialize");
        let parsed = UiStoryManifestV2::from_ron_str(&ron).expect("manifest should parse");

        assert_eq!(parsed, manifest);
    }

    #[test]
    fn manifest_v2_builder_uses_workflow_profile_not_required_stages() {
        let manifest = valid_manifest();
        let ron = manifest
            .to_ron_string_pretty()
            .expect("manifest should serialize");

        assert_eq!(
            manifest.workflow_profile_id.as_str(),
            WORKFLOW_STATIC_PREVIEW
        );
        assert!(ron.contains("workflow_profile_id"));
        assert!(!ron.contains("required_stages"));
        assert!(!ron.contains("required_stage"));
    }
}
