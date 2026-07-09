use ui_runtime::{
    BASE_CONTROLS_EXECUTABLE_OVERLAY_LAYERING_STORY_ID, base_controls_overlay_layering_fixture,
    base_controls_overlay_layering_positive_script, replay_overlay_layering,
};

#[test]
fn executable_overlay_layering_story_replays_deterministically() {
    let fixture = base_controls_overlay_layering_fixture();
    let script = base_controls_overlay_layering_positive_script();

    let first = replay_overlay_layering(&fixture, &script);
    let second = replay_overlay_layering(&fixture, &script);

    assert_eq!(first.input_steps, second.input_steps);
    assert_eq!(first.open_intents, second.open_intents);
    assert_eq!(first.stack_entries, second.stack_entries);
    assert_eq!(first.dismissal_evidence, second.dismissal_evidence);
    assert_eq!(first.boundary_assertions, second.boundary_assertions);
    assert!(BASE_CONTROLS_EXECUTABLE_OVERLAY_LAYERING_STORY_ID.contains("overlay-layering.story"));
}
