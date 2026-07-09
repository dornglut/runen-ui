//! Registry V2 for Manifest V2 story sources.
//!
//! Registry V2 parses in-memory manifest sources into a deterministic validated
//! registry. It does not discover files, read directories, or own app/editor IO.

mod builder;
mod source;
mod validated;

pub use builder::{UiStoryRegistryBuildReportV2, UiStoryRegistryBuilderV2};
pub use source::UiStoryManifestSourceV2;
pub use validated::ValidatedUiStoryRegistryV2;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostic::{UI_STORY_MANIFEST_FIELD_MISSING, UI_STORY_REGISTRY_DUPLICATE_STORY};
    use crate::manifest_v2::{UiStoryManifestV2, UiStoryMountPolicyV2};
    use crate::workflow::WORKFLOW_STATIC_PREVIEW;

    fn valid_manifest(story_id: &str) -> UiStoryManifestV2 {
        UiStoryManifestV2::builder(story_id)
            .title(format!("Story {story_id}"))
            .category("controls.button")
            .source_node(
                format!("assets/ui_gallery/stories/{story_id}.ron"),
                format!("{story_id}.source"),
            )
            .program_id(format!("{story_id}.program"))
            .host_profile("editor.gallery")
            .theme_profile("editor.dark")
            .viewport("default", 320, 128, 1.0)
            .workflow_profile(WORKFLOW_STATIC_PREVIEW)
            .expected_pass()
            .mount_policy(UiStoryMountPolicyV2::EligibleWhenPassed)
            .build()
    }

    fn valid_source(source_id: &str, path: &str, story_id: &str) -> UiStoryManifestSourceV2 {
        UiStoryManifestSourceV2::new(
            source_id,
            path,
            valid_manifest(story_id)
                .to_ron_string_pretty()
                .expect("valid manifest should serialize"),
        )
    }

    fn invalid_title_source() -> UiStoryManifestSourceV2 {
        let manifest = UiStoryManifestV2::builder("ui.gallery.invalid")
            .title("")
            .category("controls.button")
            .source_node(
                "assets/ui_gallery/stories/invalid.ron",
                "ui.gallery.invalid.source",
            )
            .program_id("ui.gallery.invalid.program")
            .host_profile("editor.gallery")
            .theme_profile("editor.dark")
            .viewport("default", 320, 128, 1.0)
            .workflow_profile(WORKFLOW_STATIC_PREVIEW)
            .expected_pass()
            .mount_policy(UiStoryMountPolicyV2::EligibleWhenPassed)
            .build();

        UiStoryManifestSourceV2::new(
            "manifest.invalid",
            "virtual/invalid.story.ron",
            manifest
                .to_ron_string_pretty()
                .expect("invalid semantic manifest should still serialize"),
        )
    }

    #[test]
    fn registry_v2_builds_validated_registry_from_valid_sources() {
        let registry = UiStoryRegistryBuilderV2::new()
            .add_source(valid_source(
                "manifest.basic",
                "virtual/basic.story.ron",
                "ui.gallery.button.basic",
            ))
            .build()
            .expect("valid sources should build a validated registry");

        assert_eq!(registry.len(), 1);
        assert!(registry.contains(&crate::identity::UiStoryId::new("ui.gallery.button.basic")));
    }

    #[test]
    fn registry_v2_rejects_invalid_manifest() {
        let report = UiStoryRegistryBuilderV2::new()
            .add_source(invalid_title_source())
            .build()
            .expect_err("invalid manifest should reject registry build");

        assert!(!report.is_valid());
        assert_eq!(report.parsed_count, 1);
        assert_eq!(report.accepted_count, 0);
        assert_eq!(report.rejected_count, 1);
        assert!(
            report
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code.as_str() == UI_STORY_MANIFEST_FIELD_MISSING)
        );
    }

    #[test]
    fn registry_v2_rejects_duplicate_story_id() {
        let report = UiStoryRegistryBuilderV2::new()
            .add_source(valid_source(
                "manifest.basic.one",
                "virtual/basic-one.story.ron",
                "ui.gallery.button.basic",
            ))
            .add_source(valid_source(
                "manifest.basic.two",
                "virtual/basic-two.story.ron",
                "ui.gallery.button.basic",
            ))
            .build()
            .expect_err("duplicate story ids should reject registry build");

        assert_eq!(report.parsed_count, 2);
        assert_eq!(report.accepted_count, 1);
        assert_eq!(report.rejected_count, 1);
        assert_eq!(report.duplicate_count, 1);
        assert!(
            report
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code.as_str() == UI_STORY_REGISTRY_DUPLICATE_STORY)
        );
    }

    #[test]
    fn registry_v2_orders_stories_deterministically_by_story_id() {
        let registry = UiStoryRegistryBuilderV2::new()
            .add_sources([
                valid_source("manifest.z", "virtual/z.story.ron", "ui.gallery.z"),
                valid_source("manifest.a", "virtual/a.story.ron", "ui.gallery.a"),
                valid_source("manifest.m", "virtual/m.story.ron", "ui.gallery.m"),
            ])
            .build()
            .expect("valid sources should build registry");

        assert_eq!(
            registry
                .story_ids()
                .map(crate::identity::UiStoryId::as_str)
                .collect::<Vec<_>>(),
            vec!["ui.gallery.a", "ui.gallery.m", "ui.gallery.z"]
        );
    }

    #[test]
    fn registry_v2_preserves_manifest_validation_diagnostics() {
        let report = UiStoryRegistryBuilderV2::new()
            .add_source(invalid_title_source())
            .build()
            .expect_err("invalid manifest should reject registry build");

        let diagnostic = report
            .diagnostics
            .iter()
            .find(|diagnostic| diagnostic.code.as_str() == UI_STORY_MANIFEST_FIELD_MISSING)
            .expect("manifest diagnostic should be preserved");

        assert!(
            diagnostic
                .context
                .iter()
                .any(|(key, value)| key == "manifest_source_id" && value == "manifest.invalid")
        );
        assert!(
            diagnostic
                .context
                .iter()
                .any(|(key, value)| key == "manifest_source_path"
                    && value == "virtual/invalid.story.ron")
        );
    }

    #[test]
    fn registry_v2_does_not_perform_filesystem_discovery() {
        let registry = UiStoryRegistryBuilderV2::new()
            .add_source(valid_source(
                "manifest.virtual",
                "/definitely/not/on/disk/story.ron",
                "ui.gallery.virtual",
            ))
            .build()
            .expect("registry should consume supplied contents without reading filesystem");

        assert_eq!(registry.len(), 1);
        assert!(registry.contains(&crate::identity::UiStoryId::new("ui.gallery.virtual")));
    }

    #[test]
    fn registry_v2_build_report_counts_are_stable() {
        let report = UiStoryRegistryBuilderV2::new()
            .add_source(valid_source(
                "manifest.valid",
                "virtual/valid.story.ron",
                "ui.gallery.valid",
            ))
            .add_source(invalid_title_source())
            .add_source(valid_source(
                "manifest.valid.duplicate",
                "virtual/valid-duplicate.story.ron",
                "ui.gallery.valid",
            ))
            .build()
            .expect_err("invalid plus duplicate input should reject registry build");

        assert_eq!(report.parsed_count, 3);
        assert_eq!(report.accepted_count, 1);
        assert_eq!(report.rejected_count, 2);
        assert_eq!(report.duplicate_count, 1);
        assert_eq!(report.diagnostics.len(), 2);
    }
}
