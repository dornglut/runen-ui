//! Workflow run recording for UI Story V2.
//!
//! Runner V2 binds a validated story registry to a semantic workflow graph and
//! records app-supplied evidence. It does not execute filesystems, compilers,
//! renderers, static mounting, or editor behavior.

mod recording;
mod runner;

pub use recording::{UiStoryWorkflowRunResultV2, UiStoryWorkflowRunV2};
pub use runner::{UiStoryRunRequestV2, UiStoryRunnerV2};

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostic::{
        UI_STORY_RUN_DUPLICATE_EVIDENCE, UI_STORY_RUN_MISSING_REQUIRED_EVIDENCE,
        UI_STORY_RUN_UNKNOWN_STORY, UI_STORY_WORKFLOW_PROFILE_UNKNOWN, UiStoryDiagnostic,
        UiStoryDiagnosticOrigin, UiStoryDiagnosticSubject,
    };
    use crate::evidence::UiStoryEvidence;
    use crate::identity::{UiStoryId, UiStoryWorkflowNodeId};
    use crate::manifest_v2::{UiStoryManifestV2, UiStoryMountPolicyV2};
    use crate::registry_v2::{
        UiStoryManifestSourceV2, UiStoryRegistryBuilderV2, ValidatedUiStoryRegistryV2,
    };
    use crate::workflow::{
        NODE_COMPILER, NODE_PREVIEW_FRAME, NODE_PROGRAM_FORMATION, NODE_RENDER_DATA,
        NODE_RENDER_PRIMITIVES, NODE_RUNTIME_VIEW, NODE_SOURCE_LOAD, NODE_SOURCE_PARSE,
        NODE_STATIC_MOUNT, WORKFLOW_SOURCE_LOAD_ONLY, WORKFLOW_STATIC_PREVIEW,
    };

    const STORY_ID: &str = "ui.gallery.button.basic";
    const PRODUCER_ID: &str = "runenwerk_editor.ui_gallery.source_loader";
    const EVIDENCE_KEY: &str = "ui.gallery.source_load";

    fn manifest(story_id: &str, workflow_profile: &str, source_path: &str) -> UiStoryManifestV2 {
        UiStoryManifestV2::builder(story_id)
            .title(format!("Story {story_id}"))
            .category("controls.button")
            .source_node(source_path, format!("{story_id}.source"))
            .program_id(format!("{story_id}.program"))
            .host_profile("editor.gallery")
            .theme_profile("editor.dark")
            .viewport("default", 320, 128, 1.0)
            .workflow_profile(workflow_profile)
            .expected_pass()
            .mount_policy(UiStoryMountPolicyV2::EligibleWhenPassed)
            .build()
    }

    fn registry_for_manifest(manifest: UiStoryManifestV2) -> ValidatedUiStoryRegistryV2 {
        UiStoryRegistryBuilderV2::new()
            .add_source(UiStoryManifestSourceV2::new(
                "manifest.basic",
                "virtual/basic.story.ron",
                manifest
                    .to_ron_string_pretty()
                    .expect("manifest should serialize"),
            ))
            .build()
            .expect("manifest should build a validated registry")
    }

    fn source_load_passed() -> UiStoryEvidence {
        UiStoryEvidence::passed(NODE_SOURCE_LOAD, PRODUCER_ID, EVIDENCE_KEY)
    }

    fn source_load_failed() -> UiStoryEvidence {
        UiStoryEvidence::failed(
            NODE_SOURCE_LOAD,
            PRODUCER_ID,
            EVIDENCE_KEY,
            vec![UiStoryDiagnostic::error(
                "ui_gallery.story.source.read_failed",
                UiStoryDiagnosticOrigin::ExternalProducer(
                    crate::identity::UiStoryEvidenceProducerId::new(PRODUCER_ID),
                ),
                UiStoryDiagnosticSubject::WorkflowNode(UiStoryWorkflowNodeId::new(
                    NODE_SOURCE_LOAD,
                )),
                "source load failed",
            )],
        )
    }

    fn passed_node_evidence(node_id: &str) -> UiStoryEvidence {
        UiStoryEvidence::passed(
            node_id,
            "runenwerk_editor.ui_gallery.test_producer",
            format!("ui.gallery.{node_id}"),
        )
    }

    #[test]
    fn runner_v2_begins_run_for_valid_story() {
        let registry = registry_for_manifest(manifest(
            STORY_ID,
            WORKFLOW_SOURCE_LOAD_ONLY,
            "assets/ui_gallery/stories/basic.ron",
        ));
        let runner = UiStoryRunnerV2::new(&registry);

        let run = runner
            .begin(STORY_ID)
            .expect("valid story should begin a workflow run");

        assert_eq!(run.story_id.as_str(), STORY_ID);
        assert_eq!(
            run.workflow_graph.profile_id.as_str(),
            WORKFLOW_SOURCE_LOAD_ONLY
        );
        assert!(run.recorded_evidence.is_empty());
    }

    #[test]
    fn runner_v2_rejects_unknown_story() {
        let registry = registry_for_manifest(manifest(
            STORY_ID,
            WORKFLOW_SOURCE_LOAD_ONLY,
            "assets/ui_gallery/stories/basic.ron",
        ));
        let runner = UiStoryRunnerV2::new(&registry);

        let result = runner
            .begin("ui.gallery.unknown")
            .expect_err("unknown story should fail closed");

        assert!(result.has_blockers());
        assert!(result.workflow_graph.is_none());
        assert!(
            result
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code.as_str() == UI_STORY_RUN_UNKNOWN_STORY)
        );
    }

    #[test]
    fn runner_v2_rejects_unknown_workflow_profile() {
        let registry = registry_for_manifest(manifest(
            STORY_ID,
            "ui_story.workflow.custom_unknown",
            "assets/ui_gallery/stories/basic.ron",
        ));
        let runner = UiStoryRunnerV2::new(&registry);

        let result = runner
            .begin(STORY_ID)
            .expect_err("unknown workflow profile should fail closed");

        assert!(result.has_blockers());
        assert!(result.workflow_graph.is_none());
        assert!(
            result
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code.as_str() == UI_STORY_WORKFLOW_PROFILE_UNKNOWN)
        );
    }

    #[test]
    fn workflow_run_v2_records_evidence() {
        let registry = registry_for_manifest(manifest(
            STORY_ID,
            WORKFLOW_SOURCE_LOAD_ONLY,
            "assets/ui_gallery/stories/basic.ron",
        ));
        let runner = UiStoryRunnerV2::new(&registry);
        let mut run = runner.begin(STORY_ID).expect("story should begin");

        run.record(source_load_passed());
        let result = run.finish();

        assert!(!result.has_blockers());
        assert_eq!(result.evidence.len(), 1);
        assert!(result.missing_required_nodes.is_empty());
    }

    #[test]
    fn workflow_run_v2_missing_required_node_blocks_result() {
        let registry = registry_for_manifest(manifest(
            STORY_ID,
            WORKFLOW_SOURCE_LOAD_ONLY,
            "assets/ui_gallery/stories/basic.ron",
        ));
        let runner = UiStoryRunnerV2::new(&registry);
        let run = runner.begin(STORY_ID).expect("story should begin");

        let result = run.finish();

        assert!(result.has_blockers());
        assert_eq!(
            result
                .missing_required_nodes
                .iter()
                .map(UiStoryWorkflowNodeId::as_str)
                .collect::<Vec<_>>(),
            vec![NODE_SOURCE_LOAD]
        );
        assert!(
            result.diagnostics.iter().any(
                |diagnostic| diagnostic.code.as_str() == UI_STORY_RUN_MISSING_REQUIRED_EVIDENCE
            )
        );
    }

    #[test]
    fn workflow_run_v2_duplicate_evidence_blocks_result() {
        let registry = registry_for_manifest(manifest(
            STORY_ID,
            WORKFLOW_SOURCE_LOAD_ONLY,
            "assets/ui_gallery/stories/basic.ron",
        ));
        let runner = UiStoryRunnerV2::new(&registry);
        let mut run = runner.begin(STORY_ID).expect("story should begin");

        run.record(source_load_passed());
        run.record(source_load_passed());
        let result = run.finish();

        assert!(result.has_blockers());
        assert_eq!(
            result
                .duplicate_evidence_keys
                .iter()
                .map(crate::identity::UiStoryEvidenceKey::as_str)
                .collect::<Vec<_>>(),
            vec![EVIDENCE_KEY]
        );
        assert!(
            result
                .diagnostics
                .iter()
                .any(|diagnostic| diagnostic.code.as_str() == UI_STORY_RUN_DUPLICATE_EVIDENCE)
        );
    }

    #[test]
    fn workflow_run_v2_failed_upstream_blocks_downstream() {
        let registry = registry_for_manifest(manifest(
            STORY_ID,
            WORKFLOW_STATIC_PREVIEW,
            "assets/ui_gallery/stories/basic.ron",
        ));
        let runner = UiStoryRunnerV2::new(&registry);
        let mut run = runner.begin(STORY_ID).expect("story should begin");

        run.record(source_load_failed());
        run.record_many([
            passed_node_evidence(NODE_SOURCE_PARSE),
            passed_node_evidence(NODE_PROGRAM_FORMATION),
            passed_node_evidence(NODE_COMPILER),
            passed_node_evidence(NODE_RUNTIME_VIEW),
            passed_node_evidence(NODE_RENDER_PRIMITIVES),
            passed_node_evidence(NODE_RENDER_DATA),
            passed_node_evidence(NODE_STATIC_MOUNT),
            passed_node_evidence(NODE_PREVIEW_FRAME),
        ]);
        let result = run.finish();

        assert!(result.has_blockers());
        assert!(result.missing_required_nodes.is_empty());
        assert!(
            result
                .blocked_nodes
                .iter()
                .any(|node_id| node_id.as_str() == NODE_SOURCE_PARSE)
        );
        assert!(
            result
                .blocked_nodes
                .iter()
                .any(|node_id| node_id.as_str() == NODE_COMPILER)
        );
    }

    #[test]
    fn workflow_run_v2_does_not_execute_app_behavior() {
        let registry = registry_for_manifest(manifest(
            STORY_ID,
            WORKFLOW_SOURCE_LOAD_ONLY,
            "/definitely/not/on/disk/story.ron",
        ));
        let runner = UiStoryRunnerV2::new(&registry);

        let run = runner
            .begin(UiStoryRunRequestV2::new(UiStoryId::new(STORY_ID)))
            .expect("runner should not read story source paths");
        let result = run.finish();

        assert_eq!(result.story_id.as_str(), STORY_ID);
        assert!(result.has_blockers());
        assert_eq!(result.missing_required_nodes.len(), 1);
    }
}
