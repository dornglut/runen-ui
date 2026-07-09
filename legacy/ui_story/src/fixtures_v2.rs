//! Checked-in UI Story V2 manifest fixture sources.
//!
//! This module exposes deterministic in-memory manifest sources. It does not own
//! gallery semantics, discover files, or execute app/editor behavior.

use crate::evidence::UiStoryDiagnosticExpectation;
use crate::registry_v2::{
    UiStoryManifestSourceV2, UiStoryRegistryBuildReportV2, UiStoryRegistryBuilderV2,
    ValidatedUiStoryRegistryV2,
};
use crate::workflow::NODE_SOURCE_LOAD;

pub const STORY_BUTTON_BASIC: &str = "ui.gallery.button.basic";
pub const STORY_BUTTON_SELECTED: &str = "ui.gallery.button.selected";
pub const STORY_BUTTON_MISSING_SOURCE: &str = "ui.gallery.button.missing_source";

pub const SOURCE_LOAD_PRODUCER_ID: &str = "runenwerk_editor.ui_gallery.source_loader";
pub const SOURCE_LOAD_EVIDENCE_KEY: &str = "ui.gallery.source_load";
pub const SOURCE_LOAD_READ_FAILED_DIAGNOSTIC: &str = "ui_gallery.story.source.read_failed";

const BUTTON_BASIC_MANIFEST_SOURCE_ID: &str = "ui.story.fixture.button.basic.manifest";
const BUTTON_SELECTED_MANIFEST_SOURCE_ID: &str = "ui.story.fixture.button.selected.manifest";
const BUTTON_MISSING_SOURCE_MANIFEST_SOURCE_ID: &str =
    "ui.story.fixture.button.missing_source.manifest";

const BUTTON_BASIC_MANIFEST_PATH: &str = "checked-in/ui_story/v2/button/basic.story.ron";
const BUTTON_SELECTED_MANIFEST_PATH: &str = "checked-in/ui_story/v2/button/selected.story.ron";
const BUTTON_MISSING_SOURCE_MANIFEST_PATH: &str =
    "checked-in/ui_story/v2/button/missing_source.story.ron";

const BUTTON_BASIC_MANIFEST_RON: &str = r#"(
    schema_version: 2,
    story_id: "ui.gallery.button.basic",
    story_revision: 0,
    title: "Button / Basic",
    category_id: "controls.button",
    source: (
        source_id: "ui.gallery.button.basic.source",
        path: "assets/ui_gallery/button/basic.ron",
        kind: Node,
    ),
    program_id: "ui.gallery.button.basic.program",
    host_profile_id: "editor.gallery",
    theme_profile_id: "editor.dark",
    viewport_matrix: (
        profiles: [
            (
                viewport_id: "default",
                width: 320,
                height: 128,
                scale: 1.0,
            ),
        ],
    ),
    workflow_profile_id: "ui_story.workflow.static_preview",
    expected_outcome: Pass,
    mount_policy: EligibleWhenPassed,
)"#;

const BUTTON_SELECTED_MANIFEST_RON: &str = r#"(
    schema_version: 2,
    story_id: "ui.gallery.button.selected",
    story_revision: 0,
    title: "Button / Selected",
    category_id: "controls.button",
    source: (
        source_id: "ui.gallery.button.selected.source",
        path: "assets/ui_gallery/button/selected.ron",
        kind: Node,
    ),
    program_id: "ui.gallery.button.selected.program",
    host_profile_id: "editor.gallery",
    theme_profile_id: "editor.dark",
    viewport_matrix: (
        profiles: [
            (
                viewport_id: "default",
                width: 320,
                height: 128,
                scale: 1.0,
            ),
        ],
    ),
    workflow_profile_id: "ui_story.workflow.static_preview",
    expected_outcome: Pass,
    mount_policy: EligibleWhenPassed,
)"#;

const BUTTON_MISSING_SOURCE_MANIFEST_RON: &str = r#"(
    schema_version: 2,
    story_id: "ui.gallery.button.missing_source",
    story_revision: 0,
    title: "Button / Missing Source",
    category_id: "controls.button",
    source: (
        source_id: "ui.gallery.button.missing_source.source",
        path: "assets/ui_gallery/stories/controls/button/missing_source.ron",
        kind: Node,
    ),
    program_id: "ui.gallery.button.missing_source.program",
    host_profile_id: "editor.gallery",
    theme_profile_id: "editor.dark",
    viewport_matrix: (
        profiles: [
            (
                viewport_id: "default",
                width: 320,
                height: 128,
                scale: 1.0,
            ),
        ],
    ),
    workflow_profile_id: "ui_story.workflow.source_load_only",
    expected_outcome: ExpectedFailure(
        expectation: (
            workflow_node_id: "source_load",
            producer_id: "runenwerk_editor.ui_gallery.source_loader",
            evidence_key: "ui.gallery.source_load",
            code: "ui_gallery.story.source.read_failed",
            severity: Error,
        ),
    ),
    mount_policy: Never,
)"#;

pub fn checked_in_story_manifest_sources_v2() -> Vec<UiStoryManifestSourceV2> {
    vec![
        UiStoryManifestSourceV2::new(
            BUTTON_BASIC_MANIFEST_SOURCE_ID,
            BUTTON_BASIC_MANIFEST_PATH,
            BUTTON_BASIC_MANIFEST_RON,
        ),
        UiStoryManifestSourceV2::new(
            BUTTON_MISSING_SOURCE_MANIFEST_SOURCE_ID,
            BUTTON_MISSING_SOURCE_MANIFEST_PATH,
            BUTTON_MISSING_SOURCE_MANIFEST_RON,
        ),
        UiStoryManifestSourceV2::new(
            BUTTON_SELECTED_MANIFEST_SOURCE_ID,
            BUTTON_SELECTED_MANIFEST_PATH,
            BUTTON_SELECTED_MANIFEST_RON,
        ),
    ]
}

pub fn checked_in_story_registry_v2()
-> Result<ValidatedUiStoryRegistryV2, UiStoryRegistryBuildReportV2> {
    UiStoryRegistryBuilderV2::new()
        .add_sources(checked_in_story_manifest_sources_v2())
        .build()
}

pub fn source_load_read_failure_expectation_v2() -> UiStoryDiagnosticExpectation {
    UiStoryDiagnosticExpectation::error_from_strings(
        NODE_SOURCE_LOAD,
        SOURCE_LOAD_PRODUCER_ID,
        SOURCE_LOAD_EVIDENCE_KEY,
        SOURCE_LOAD_READ_FAILED_DIAGNOSTIC,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::identity::UiStoryId;
    use crate::manifest_v2::{UiStoryExpectedOutcomeV2, UiStoryManifestV2, UiStoryMountPolicyV2};
    use crate::workflow::{WORKFLOW_SOURCE_LOAD_ONLY, WORKFLOW_STATIC_PREVIEW};

    fn manifest(story_id: &str) -> UiStoryManifestV2 {
        checked_in_story_registry_v2()
            .expect("checked-in registry should validate")
            .get(&UiStoryId::new(story_id))
            .expect("fixture story should exist")
            .clone()
    }

    #[test]
    fn fixtures_v2_sources_are_deterministic() {
        let sources = checked_in_story_manifest_sources_v2();
        let source_ids = sources
            .iter()
            .map(|source| source.source_id.as_str())
            .collect::<Vec<_>>();

        assert_eq!(
            source_ids,
            vec![
                BUTTON_BASIC_MANIFEST_SOURCE_ID,
                BUTTON_MISSING_SOURCE_MANIFEST_SOURCE_ID,
                BUTTON_SELECTED_MANIFEST_SOURCE_ID,
            ]
        );
    }

    #[test]
    fn fixtures_v2_registry_builds_validated_registry() {
        let registry = checked_in_story_registry_v2().expect("checked-in registry should build");

        assert_eq!(registry.len(), 3);
        assert!(registry.contains(&UiStoryId::new(STORY_BUTTON_BASIC)));
        assert!(registry.contains(&UiStoryId::new(STORY_BUTTON_SELECTED)));
        assert!(registry.contains(&UiStoryId::new(STORY_BUTTON_MISSING_SOURCE)));
    }

    #[test]
    fn fixtures_v2_passing_stories_use_static_preview_workflow() {
        for story_id in [STORY_BUTTON_BASIC, STORY_BUTTON_SELECTED] {
            let manifest = manifest(story_id);

            assert_eq!(
                manifest.workflow_profile_id.as_str(),
                WORKFLOW_STATIC_PREVIEW
            );
            assert_eq!(manifest.expected_outcome, UiStoryExpectedOutcomeV2::Pass);
            assert_eq!(
                manifest.mount_policy,
                UiStoryMountPolicyV2::EligibleWhenPassed
            );
        }
    }

    #[test]
    fn fixtures_v2_missing_source_is_expected_failure() {
        let manifest = manifest(STORY_BUTTON_MISSING_SOURCE);

        assert_eq!(
            manifest.workflow_profile_id.as_str(),
            WORKFLOW_SOURCE_LOAD_ONLY
        );
        assert_eq!(manifest.mount_policy, UiStoryMountPolicyV2::Never);
        assert_eq!(
            manifest.expected_outcome,
            UiStoryExpectedOutcomeV2::ExpectedFailure {
                expectation: source_load_read_failure_expectation_v2(),
            }
        );
    }

    #[test]
    fn fixtures_v2_manifest_ron_does_not_contain_required_stages() {
        for source in checked_in_story_manifest_sources_v2() {
            assert!(!source.contents.contains("required_stages"));
            assert!(!source.contents.contains("required_stage"));
            assert!(source.contents.contains("workflow_profile_id"));
        }
    }

    #[test]
    fn fixtures_v2_does_not_perform_filesystem_discovery() {
        let sources = checked_in_story_manifest_sources_v2();

        assert!(sources.iter().all(|source| !source.contents.is_empty()));
        assert!(
            sources
                .iter()
                .all(|source| source.path.starts_with("checked-in/"))
        );
        assert!(checked_in_story_registry_v2().is_ok());
    }
}
