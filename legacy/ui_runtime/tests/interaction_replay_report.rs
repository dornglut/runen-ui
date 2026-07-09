use ui_controls::{BUTTON_CONTROL_KIND_ID, BaseControlsPlugin};
use ui_input::{
    FocusDirection, FocusInputFact, FocusTargetId, Key, KeyState, KeyboardInputFact,
    NormalizedInputFact, NormalizedInputSample, PointerButton, PointerEventKind, PointerInputFact,
    TextIntentFact,
};
use ui_math::UiPoint;
use ui_runtime::{
    InteractionReplayScript, InteractionReplayStep, InteractionVisibleState,
    InteractionVisualProof, MountedInteractionFixture, WidgetId,
    base_controls_generic_interaction_fixture, base_controls_generic_interaction_positive_script,
    replay_interactions,
};

#[test]
fn mounted_interaction_replay_reports_facts_events_outcomes_and_boundaries() {
    let fixture = base_controls_fixture();
    let script = base_controls_positive_script();
    let report = replay_interactions(&fixture, &script);
    assert!(report.boundary_assertions.no_bypass_evidence());
    assert_eq!(report.boundary_assertions.host_commands_executed, 0);
    assert_eq!(report.boundary_assertions.product_mutations, 0);
    assert_eq!(report.boundary_assertions.overlay_events, 0);
    assert_eq!(report.boundary_assertions.text_edit_transactions, 0);
    assert!(
        report
            .runtime_facts
            .iter()
            .any(|fact| fact.fact == "hovered" && fact.target == WidgetId(1))
    );
    assert!(
        report
            .state_transitions
            .iter()
            .any(|transition| transition.state == "pressed"
                && transition.active
                && transition.target == WidgetId(1))
    );
    assert!(
        report
            .runtime_events
            .iter()
            .any(|event| event.event == "pointer-activate"
                && event.step_id == "release_button"
                && event.target == WidgetId(1))
    );
    assert!(
        !report
            .semantic_outcomes
            .iter()
            .any(|outcome| outcome.step_id == "press_button"
                && outcome.outcome == "activation-requested")
    );
    assert!(
        report
            .semantic_outcomes
            .iter()
            .any(|outcome| outcome.outcome == "activation-requested"
                && outcome.target == WidgetId(1)
                && outcome.step_id == "release_button")
    );
    assert_outcome(&report, WidgetId(2), "action-intent");
    assert_outcome(&report, WidgetId(3), "text-intent-seen");
    assert_outcome(&report, WidgetId(4), "active-item-intent");
    assert_outcome(&report, WidgetId(5), "node-intent");
    assert_outcome(&report, WidgetId(6), "cell-or-row-intent");
    assert!(
        report
            .suppressed_events
            .iter()
            .any(|event| event.reason == "control.disabled" && event.target == WidgetId(7))
    );
    assert!(
        report
            .no_target_events
            .iter()
            .any(|event| event.reason == "pointer.no_target")
    );
}

#[test]
fn visual_proof_exposes_main_inspector_and_report_views() {
    let fixture = base_controls_fixture();
    let report = replay_interactions(&fixture, &base_controls_positive_script());
    let proof = InteractionVisualProof::from_fixture_report(
        "runenwerk.ui.controls.base.generic_interaction",
        &fixture,
        &report,
        WidgetId(1),
    );
    assert_eq!(proof.main_view.controls.len(), fixture.controls.len());
    assert_eq!(proof.inspector_view.selected_widget, Some(WidgetId(1)));
    assert_eq!(
        proof.inspector_view.control_kind_id.as_deref(),
        Some(BUTTON_CONTROL_KIND_ID)
    );
    assert!(
        proof
            .inspector_view
            .declared_requirements
            .iter()
            .any(|requirement| requirement.starts_with("pointer-activate"))
    );
    assert!(
        proof
            .inspector_view
            .observed_reusable_interaction_states
            .contains(&InteractionVisibleState::FocusVisible)
    );
    assert!(
        !proof
            .inspector_view
            .current_reusable_interaction_state_set
            .contains(&InteractionVisibleState::Pressed)
    );
    assert_marker(&proof, WidgetId(1), InteractionVisibleState::Hovered);
    assert_marker(&proof, WidgetId(1), InteractionVisibleState::Pressed);
    assert_marker(&proof, WidgetId(1), InteractionVisibleState::Focused);
    assert_marker(&proof, WidgetId(1), InteractionVisibleState::FocusVisible);
    assert_marker(
        &proof,
        WidgetId(1),
        InteractionVisibleState::ActivationRequested,
    );
    assert_marker(&proof, WidgetId(7), InteractionVisibleState::Disabled);
    assert_marker(&proof, WidgetId(7), InteractionVisibleState::Suppressed);
    assert_marker(
        &proof,
        WidgetId(4),
        InteractionVisibleState::ListActiveItemIntent,
    );
    assert_marker(&proof, WidgetId(5), InteractionVisibleState::TreeNodeIntent);
    assert_marker(
        &proof,
        WidgetId(6),
        InteractionVisibleState::TableCellOrRowIntent,
    );
    assert_marker(
        &proof,
        WidgetId(3),
        InteractionVisibleState::TextIntentProbe,
    );
    assert!(proof.report_view.boundary_assertions.no_bypass_evidence());
    assert!(
        proof
            .report_view
            .runtime_events
            .iter()
            .any(|event| event.contains("pointer-activate"))
    );
    assert!(
        proof
            .report_view
            .suppressed_events
            .iter()
            .any(|event| event.contains("control.disabled"))
    );
    assert!(
        proof
            .report_view
            .no_target_events
            .iter()
            .any(|event| event.contains("pointer.no_target"))
    );
}

#[test]
fn pointer_release_outside_clears_capture_and_does_not_activate() {
    let fixture = base_controls_fixture();
    let script = InteractionReplayScript::new(
        "runenwerk.ui.controls.base.generic_interaction.release_outside",
    )
    .with_step(pointer_step(
        "press_button",
        PointerEventKind::Down,
        12.0,
        12.0,
    ))
    .with_step(pointer_step(
        "leave_button",
        PointerEventKind::Leave,
        120.0,
        12.0,
    ))
    .with_step(pointer_step(
        "release_outside",
        PointerEventKind::Up,
        260.0,
        260.0,
    ));
    let report = replay_interactions(&fixture, &script);
    assert!(
        report
            .runtime_events
            .iter()
            .any(|event| event.event == "pointer_leave_kept_capture")
    );
    assert!(
        report
            .state_transitions
            .iter()
            .any(|transition| transition.state == "captured"
                && !transition.active
                && transition.target == WidgetId(1))
    );
    assert!(
        report
            .suppressed_events
            .iter()
            .any(|event| event.reason == "pointer.release_outside")
    );
    assert!(
        !report
            .semantic_outcomes
            .iter()
            .any(|outcome| outcome.outcome == "activation-requested")
    );
    assert!(report.boundary_assertions.no_bypass_evidence());
}

#[test]
fn focus_resolution_validates_target_state_and_declarations() {
    let fixture = base_controls_fixture();
    let script = InteractionReplayScript::new(
        "runenwerk.ui.controls.base.generic_interaction.focus_validation",
    )
    .with_step(focus_target_step("focus_button", WidgetId(1)))
    .with_step(focus_target_step("focus_missing", WidgetId(404)))
    .with_step(focus_target_step("focus_disabled", WidgetId(7)))
    .with_step(focus_target_step("focus_label_without_focus", WidgetId(9)))
    .with_step(focus_target_step("focus_inert", WidgetId(10)))
    .with_step(focus_next_step("focus_traversal"));
    let report = replay_interactions(&fixture, &script);
    assert_focus_reason(&report, "focus_button", "focus.target_resolved");
    assert_focus_reason(&report, "focus_missing", "focus.target_missing");
    assert_focus_reason(&report, "focus_disabled", "focus.target_disabled");
    assert_focus_reason(
        &report,
        "focus_label_without_focus",
        "focus.target_does_not_declare_focus",
    );
    assert_focus_reason(&report, "focus_inert", "focus.target_not_focusable");
    assert_focus_reason(&report, "focus_traversal", "focus.target_resolved");
    assert!(
        report
            .no_target_events
            .iter()
            .any(|event| event.reason == "focus.target_missing")
    );
    assert!(
        report
            .suppressed_events
            .iter()
            .any(|event| event.reason == "focus.target_disabled")
    );
    assert!(
        report
            .suppressed_events
            .iter()
            .any(|event| event.reason == "focus.target_not_focusable")
    );
    assert!(
        report
            .suppressed_events
            .iter()
            .any(|event| event.reason == "focus.target_does_not_declare_focus")
    );
}

#[test]
fn read_only_text_intent_is_observed_as_probe_without_text_editing() {
    let fixture = base_controls_fixture();
    let script = InteractionReplayScript::new(
        "runenwerk.ui.controls.base.generic_interaction.read_only_text",
    )
    .with_step(focus_target_step("focus_read_only_inspector", WidgetId(8)))
    .with_step(text_intent_step("text_read_only_probe"));
    let report = replay_interactions(&fixture, &script);
    let proof = InteractionVisualProof::from_fixture_report(
        "runenwerk.ui.controls.base.generic_interaction.read_only_text",
        &fixture,
        &report,
        WidgetId(8),
    );
    assert_outcome(&report, WidgetId(8), "text-intent-seen");
    assert!(
        report
            .runtime_facts
            .iter()
            .any(|fact| fact.target == WidgetId(8) && fact.fact == "text-intent-read-only-probe")
    );
    assert_eq!(report.boundary_assertions.text_edit_transactions, 0);
    assert_eq!(report.boundary_assertions.product_mutations, 0);
    assert_eq!(report.boundary_assertions.host_commands_executed, 0);
    assert_marker(
        &proof,
        WidgetId(8),
        InteractionVisibleState::ReadOnlyTextIntentProbe,
    );
    assert!(proof.inspector_view.read_only);
    assert!(proof.inspector_view.text_intent_probe);
}

#[test]
fn replay_reports_disabled_no_target_and_keyboard_without_focus_cases() {
    let fixture = base_controls_fixture();
    let script =
        InteractionReplayScript::new("runenwerk.ui.controls.base.generic_interaction.negative")
            .with_step(key_step("keyboard_without_focus", Key::Enter))
            .with_step(pointer_step(
                "disabled_button",
                PointerEventKind::Down,
                12.0,
                132.0,
            ))
            .with_step(pointer_step(
                "outside",
                PointerEventKind::Down,
                260.0,
                260.0,
            ));
    let report = replay_interactions(&fixture, &script);
    assert!(
        report
            .no_target_events
            .iter()
            .any(|event| event.reason == "keyboard.no_focus")
    );
    assert!(
        report
            .suppressed_events
            .iter()
            .any(|event| event.reason == "control.disabled")
    );
    assert!(
        report
            .no_target_events
            .iter()
            .any(|event| event.reason == "pointer.no_target")
    );
    assert!(report.semantic_outcomes.is_empty());
    assert!(report.boundary_assertions.no_bypass_evidence());
}

#[test]
fn text_intent_against_non_text_probe_is_suppressed_without_text_editing() {
    let fixture = base_controls_fixture();
    let script = InteractionReplayScript::new(
        "runenwerk.ui.controls.base.generic_interaction.text_negative",
    )
    .with_step(focus_target_step("focus_action", WidgetId(2)))
    .with_step(text_intent_step("text_against_action"));
    let report = replay_interactions(&fixture, &script);
    assert!(
        report
            .suppressed_events
            .iter()
            .any(|event| event.reason == "text_intent.not_declared")
    );
    assert_eq!(report.boundary_assertions.text_edit_transactions, 0);
    assert_eq!(report.boundary_assertions.product_mutations, 0);
    assert_eq!(report.boundary_assertions.host_commands_executed, 0);
}

fn base_controls_positive_script() -> InteractionReplayScript {
    base_controls_generic_interaction_positive_script()
}

fn base_controls_fixture() -> MountedInteractionFixture {
    let compiled = BaseControlsPlugin::new().compile();
    base_controls_generic_interaction_fixture(&compiled)
}

fn pointer_step(id: &str, kind: PointerEventKind, x: f32, y: f32) -> InteractionReplayStep {
    InteractionReplayStep::new(
        id,
        NormalizedInputSample::new(id).with_fact(NormalizedInputFact::Pointer(
            PointerInputFact::new(kind, UiPoint::new(x, y)).with_button(PointerButton::Primary),
        )),
    )
}

fn focus_next_step(id: &str) -> InteractionReplayStep {
    InteractionReplayStep::new(
        id,
        NormalizedInputSample::new(id).with_fact(NormalizedInputFact::Focus(
            FocusInputFact::traversal(FocusDirection::Next),
        )),
    )
}

fn focus_target_step(id: &str, widget_id: WidgetId) -> InteractionReplayStep {
    InteractionReplayStep::new(
        id,
        NormalizedInputSample::new(id).with_fact(NormalizedInputFact::Focus(
            FocusInputFact::target(FocusTargetId(widget_id.0)),
        )),
    )
}

fn key_step(id: &str, key: Key) -> InteractionReplayStep {
    InteractionReplayStep::new(
        id,
        NormalizedInputSample::new(id).with_fact(NormalizedInputFact::Keyboard(
            KeyboardInputFact::new(key, KeyState::Pressed),
        )),
    )
}

fn text_intent_step(id: &str) -> InteractionReplayStep {
    InteractionReplayStep::new(
        id,
        NormalizedInputSample::new(id).with_fact(NormalizedInputFact::TextIntent(
            TextIntentFact::insert_text("probe"),
        )),
    )
}

fn assert_outcome(
    report: &ui_runtime::InteractionFormationReport,
    target: WidgetId,
    outcome: &str,
) {
    assert!(
        report
            .semantic_outcomes
            .iter()
            .any(|entry| entry.target == target && entry.outcome == outcome)
    );
}

fn assert_focus_reason(
    report: &ui_runtime::InteractionFormationReport,
    step_id: &str,
    reason: &str,
) {
    assert!(
        report
            .focus_resolution
            .iter()
            .any(|entry| entry.step_id == step_id && entry.reason == reason)
    );
}

fn assert_marker(
    proof: &InteractionVisualProof,
    widget_id: WidgetId,
    state: InteractionVisibleState,
) {
    let control = proof
        .main_view
        .control(widget_id)
        .expect("visual proof should contain control");
    assert!(control.has_marker(state));
}
