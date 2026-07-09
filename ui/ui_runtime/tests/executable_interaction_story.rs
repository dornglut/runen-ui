use ui_controls::BaseControlsPlugin;
use ui_runtime::{
    InteractionStoryExecutionMode, InteractionVisibleState, WidgetId,
    base_controls_executable_interaction_expected_evidence,
    base_controls_executable_interaction_story_session,
    base_controls_generic_interaction_positive_script,
};

#[test]
fn base_controls_executable_interaction_story_replay_produces_expected_evidence() {
    let compiled = BaseControlsPlugin::new().compile();
    let script = base_controls_generic_interaction_positive_script();
    let expected = base_controls_executable_interaction_expected_evidence();
    let mut session = base_controls_executable_interaction_story_session(
        &compiled,
        InteractionStoryExecutionMode::Replay,
    );

    let report = session.run_script_with_expected(&script, &expected);

    assert!(report.evidence_result.passed());
    assert_eq!(report.input_log.len(), script.steps.len());
    assert!(report.boundary_assertions.no_bypass_evidence());
    assert!(report.render_summary.has_main_inspector_and_report);

    let button = report.visual_proof.main_view.control(WidgetId(1)).unwrap();
    assert!(button.has_marker(InteractionVisibleState::Pressed));
    assert!(button.has_marker(InteractionVisibleState::Captured));
    assert!(button.has_marker(InteractionVisibleState::ActivationRequested));
    assert!(!button.has_current_state(InteractionVisibleState::Pressed));
}

#[test]
fn base_controls_executable_interaction_story_live_log_replays_with_semantic_parity() {
    let compiled = BaseControlsPlugin::new().compile();
    let script = base_controls_generic_interaction_positive_script();
    let expected = base_controls_executable_interaction_expected_evidence();
    let mut live = base_controls_executable_interaction_story_session(
        &compiled,
        InteractionStoryExecutionMode::Live,
    );

    for step in &script.steps {
        live.apply_step(step.clone());
    }

    let report = live.run_report_with_expected(&expected);
    assert!(report.evidence_result.passed());

    let parity = live.replay_live_parity_report();
    assert!(parity.passed());
    assert_eq!(
        parity.live_report.input_log,
        parity.replayed_live_log_report.input_log
    );
    assert!(parity.equivalent_target_resolution);
    assert!(parity.equivalent_focus_resolution);
    assert!(parity.equivalent_state_transitions);
    assert!(parity.equivalent_runtime_facts);
    assert!(parity.equivalent_runtime_events);
    assert!(parity.equivalent_semantic_outcomes);
    assert!(parity.equivalent_suppression);
    assert!(parity.equivalent_no_target);
    assert!(parity.equivalent_observed_markers);
    assert!(parity.equivalent_final_current_states);
    assert!(parity.equivalent_boundaries);
}
