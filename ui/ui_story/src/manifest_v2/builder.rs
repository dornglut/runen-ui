use crate::evidence::UiStoryDiagnosticExpectation;
use crate::identity::{
    UiStoryCategoryId, UiStoryHostProfileId, UiStoryId, UiStoryProgramId, UiStoryRevision,
    UiStoryThemeProfileId, UiStoryWorkflowProfileId,
};

use super::{
    UI_STORY_MANIFEST_V2_SCHEMA_VERSION, UiStoryExpectedOutcomeV2, UiStoryManifestV2,
    UiStoryMountPolicyV2, UiStorySourceRef, UiStoryViewportMatrix,
};

#[derive(Clone, Debug)]
pub struct UiStoryManifestBuilder {
    story_id: UiStoryId,
    story_revision: UiStoryRevision,
    title: String,
    category_id: UiStoryCategoryId,
    source: UiStorySourceRef,
    program_id: UiStoryProgramId,
    host_profile_id: UiStoryHostProfileId,
    theme_profile_id: UiStoryThemeProfileId,
    viewport_matrix: UiStoryViewportMatrix,
    workflow_profile_id: UiStoryWorkflowProfileId,
    expected_outcome: UiStoryExpectedOutcomeV2,
    mount_policy: UiStoryMountPolicyV2,
}

impl UiStoryManifestBuilder {
    pub fn new(story_id: impl Into<String>) -> Self {
        Self {
            story_id: UiStoryId::new(story_id),
            story_revision: UiStoryRevision::new(0),
            title: String::new(),
            category_id: UiStoryCategoryId::new(""),
            source: UiStorySourceRef::node("", ""),
            program_id: UiStoryProgramId::new(""),
            host_profile_id: UiStoryHostProfileId::new(""),
            theme_profile_id: UiStoryThemeProfileId::new(""),
            viewport_matrix: UiStoryViewportMatrix::default(),
            workflow_profile_id: UiStoryWorkflowProfileId::new(""),
            expected_outcome: UiStoryExpectedOutcomeV2::Pass,
            mount_policy: UiStoryMountPolicyV2::GalleryOnly,
        }
    }

    pub fn story_revision(mut self, story_revision: UiStoryRevision) -> Self {
        self.story_revision = story_revision;
        self
    }

    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = title.into();
        self
    }

    pub fn category(mut self, category_id: impl Into<String>) -> Self {
        self.category_id = UiStoryCategoryId::new(category_id);
        self
    }

    pub fn source_node(mut self, path: impl Into<String>, source_id: impl Into<String>) -> Self {
        self.source = UiStorySourceRef::node(path, source_id);
        self
    }

    pub fn source_template(
        mut self,
        path: impl Into<String>,
        source_id: impl Into<String>,
    ) -> Self {
        self.source = UiStorySourceRef::template(path, source_id);
        self
    }

    pub fn program_id(mut self, program_id: impl Into<String>) -> Self {
        self.program_id = UiStoryProgramId::new(program_id);
        self
    }

    pub fn host_profile(mut self, host_profile_id: impl Into<String>) -> Self {
        self.host_profile_id = UiStoryHostProfileId::new(host_profile_id);
        self
    }

    pub fn theme_profile(mut self, theme_profile_id: impl Into<String>) -> Self {
        self.theme_profile_id = UiStoryThemeProfileId::new(theme_profile_id);
        self
    }

    pub fn viewport(
        mut self,
        viewport_id: impl Into<String>,
        width: u32,
        height: u32,
        scale: f32,
    ) -> Self {
        self.viewport_matrix = UiStoryViewportMatrix::single(viewport_id, width, height, scale);
        self
    }

    pub fn viewport_matrix(mut self, viewport_matrix: UiStoryViewportMatrix) -> Self {
        self.viewport_matrix = viewport_matrix;
        self
    }

    pub fn workflow_profile(mut self, workflow_profile_id: impl Into<String>) -> Self {
        self.workflow_profile_id = UiStoryWorkflowProfileId::new(workflow_profile_id);
        self
    }

    pub fn expected_pass(mut self) -> Self {
        self.expected_outcome = UiStoryExpectedOutcomeV2::Pass;
        self
    }

    pub fn expected_failure(mut self, expectation: UiStoryDiagnosticExpectation) -> Self {
        self.expected_outcome = UiStoryExpectedOutcomeV2::ExpectedFailure { expectation };
        self
    }

    pub fn mount_policy(mut self, mount_policy: UiStoryMountPolicyV2) -> Self {
        self.mount_policy = mount_policy;
        self
    }

    pub fn build(self) -> UiStoryManifestV2 {
        UiStoryManifestV2 {
            schema_version: UI_STORY_MANIFEST_V2_SCHEMA_VERSION,
            story_id: self.story_id,
            story_revision: self.story_revision,
            title: self.title,
            category_id: self.category_id,
            source: self.source,
            program_id: self.program_id,
            host_profile_id: self.host_profile_id,
            theme_profile_id: self.theme_profile_id,
            viewport_matrix: self.viewport_matrix,
            workflow_profile_id: self.workflow_profile_id,
            expected_outcome: self.expected_outcome,
            mount_policy: self.mount_policy,
        }
    }
}
