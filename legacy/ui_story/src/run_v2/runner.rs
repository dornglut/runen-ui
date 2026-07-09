use serde::{Deserialize, Serialize};

use crate::diagnostic::{
    UI_STORY_RUN_UNKNOWN_STORY, UI_STORY_WORKFLOW_PROFILE_UNKNOWN, UiStoryDiagnostic,
    UiStoryDiagnosticOrigin, UiStoryDiagnosticSubject,
};
use crate::identity::{UiStoryId, UiStoryWorkflowProfileId};
use crate::registry_v2::ValidatedUiStoryRegistryV2;
use crate::workflow::{UiStoryBuiltinWorkflowProfile, UiStoryWorkflowGraph};

use super::{UiStoryWorkflowRunResultV2, UiStoryWorkflowRunV2};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct UiStoryRunRequestV2 {
    pub story_id: UiStoryId,
}

impl UiStoryRunRequestV2 {
    pub fn new(story_id: UiStoryId) -> Self {
        Self { story_id }
    }

    pub fn story_id(&self) -> &UiStoryId {
        &self.story_id
    }
}

impl From<UiStoryId> for UiStoryRunRequestV2 {
    fn from(story_id: UiStoryId) -> Self {
        Self::new(story_id)
    }
}

impl From<&str> for UiStoryRunRequestV2 {
    fn from(story_id: &str) -> Self {
        Self::new(UiStoryId::new(story_id))
    }
}

impl From<String> for UiStoryRunRequestV2 {
    fn from(story_id: String) -> Self {
        Self::new(UiStoryId::new(story_id))
    }
}

#[derive(Clone, Debug)]
pub struct UiStoryRunnerV2<'registry> {
    registry: &'registry ValidatedUiStoryRegistryV2,
}

impl<'registry> UiStoryRunnerV2<'registry> {
    pub fn new(registry: &'registry ValidatedUiStoryRegistryV2) -> Self {
        Self { registry }
    }

    pub fn begin(
        &self,
        request: impl Into<UiStoryRunRequestV2>,
    ) -> Result<UiStoryWorkflowRunV2, Box<UiStoryWorkflowRunResultV2>> {
        let request = request.into();
        let story_id = request.story_id;
        let Some(story) = self.registry.get(&story_id) else {
            return Err(Box::new(UiStoryWorkflowRunResultV2::failed_seed(
                story_id.clone(),
                None,
                UiStoryDiagnostic::error(
                    UI_STORY_RUN_UNKNOWN_STORY,
                    UiStoryDiagnosticOrigin::Runner,
                    UiStoryDiagnosticSubject::Story(story_id.clone()),
                    format!("unknown ui story {}", story_id.as_str()),
                ),
            )));
        };

        let Some(workflow_graph) = resolve_builtin_workflow_graph(&story.workflow_profile_id)
        else {
            return Err(Box::new(UiStoryWorkflowRunResultV2::failed_seed(
                story_id.clone(),
                None,
                UiStoryDiagnostic::error(
                    UI_STORY_WORKFLOW_PROFILE_UNKNOWN,
                    UiStoryDiagnosticOrigin::Runner,
                    UiStoryDiagnosticSubject::WorkflowProfile(story.workflow_profile_id.clone()),
                    format!(
                        "unknown ui story workflow profile {}",
                        story.workflow_profile_id.as_str()
                    ),
                ),
            )));
        };

        Ok(UiStoryWorkflowRunV2::new(story_id, workflow_graph))
    }
}

fn resolve_builtin_workflow_graph(
    profile_id: &UiStoryWorkflowProfileId,
) -> Option<UiStoryWorkflowGraph> {
    UiStoryBuiltinWorkflowProfile::all()
        .find(|profile| profile.profile_id() == *profile_id)
        .map(UiStoryBuiltinWorkflowProfile::graph)
}
