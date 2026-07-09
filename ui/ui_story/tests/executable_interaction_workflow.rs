use ui_story::{UiStoryBuiltinWorkflowProfile, WORKFLOW_EXECUTABLE_INTERACTION_PROOF};

#[test]
fn executable_interaction_workflow_profile_is_available() {
    let profile = UiStoryBuiltinWorkflowProfile::ExecutableInteractionProof;
    assert_eq!(
        profile.profile_id().as_str(),
        WORKFLOW_EXECUTABLE_INTERACTION_PROOF
    );
    assert!(profile.graph().validate().is_empty());
}
