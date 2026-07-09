use ui_controls::ControlEditableTextSelectionPolicy;
use ui_input::{
    FocusInputFact, FocusTargetId, NormalizedInputFact, NormalizedInputSample, TextCompositionFact,
    TextEditFact, TextPosition, TextRange, TextSelectionFact,
};
use ui_runtime::{
    TextEditingLifecycleState, TextEditingReplayScript, TextEditingReplayStep, TextEditingReport,
    WidgetId, base_controls_text_editing_fixture, base_controls_text_editing_positive_script,
    base_controls_text_editing_proof_frame, base_controls_text_editing_suppression_script,
    replay_text_editing,
};

#[test]
fn text_editing_replay_reports_caret_selection_composition_and_boundaries() {
    let fixture = base_controls_text_editing_fixture();
    let script = base_controls_text_editing_positive_script();
    let report = replay_text_editing(&fixture, &script);

    assert_eq!(report.descriptor_evidence.len(), 1);
    assert!(report.input_steps.len() >= 12);
    assert!(
        report
            .accepted_edit_intents
            .iter()
            .any(|intent| intent.intent == "insert-text")
    );
    assert!(
        report
            .accepted_edit_intents
            .iter()
            .any(|intent| intent.intent == "replace-selection")
    );
    assert!(
        report
            .suppressed_edit_intents
            .iter()
            .any(|intent| intent.intent == "paste"
                && intent.reason == "host_owned_source.unsupported_by_descriptor")
    );
    assert!(!report.caret_evidence.is_empty());
    assert!(
        report
            .selection_evidence
            .iter()
            .any(|selection| !selection.collapsed && selection.accepted)
    );
    assert!(
        report
            .composition_evidence
            .iter()
            .any(|composition| composition.kind == "composition-start")
    );
    assert!(
        report
            .composition_evidence
            .iter()
            .any(|composition| composition.kind == "composition-commit")
    );
    assert!(
        report
            .value_evidence
            .iter()
            .any(|value| value.rendered_value.contains("[e composing]"))
    );
    assert!(
        report
            .value_evidence
            .iter()
            .any(|value| value.committed_text == "cafe")
    );
    assert!(report.lifecycle_transitions.iter().any(|transition| {
        transition.to == TextEditingLifecycleState::Composing
            && transition.reason == "composition-start"
    }));
    let proof_frame = base_controls_text_editing_proof_frame(report.clone());
    assert!(proof_frame.summary.value_rows >= 1);
    assert!(proof_frame.summary.no_bypass_proven);
    assert!(report.boundary_assertions.no_bypass_evidence());
    assert_eq!(report.boundary_assertions.host_commands_executed, 0);
    assert_eq!(report.boundary_assertions.product_mutations, 0);
    assert_eq!(report.boundary_assertions.authored_ui_edits, 0);
    assert_eq!(report.boundary_assertions.product_undo_redo_operations, 0);
    assert_eq!(report.boundary_assertions.plugin_framework_operations, 0);
}

#[test]
fn text_editing_replay_suppresses_no_target_without_bypass() {
    let fixture = base_controls_text_editing_fixture();
    let script = base_controls_text_editing_suppression_script();
    let report = replay_text_editing(&fixture, &script);

    assert!(report.accepted_edit_intents.is_empty());
    assert_eq!(report.suppressed_edit_intents.len(), 1);
    assert_eq!(report.suppressed_edit_intents[0].reason, "target.missing");
    assert!(report.boundary_assertions.no_bypass_evidence());
}

#[test]
fn runtime_suppresses_disabled_target() {
    let mut fixture = base_controls_text_editing_fixture();
    let target_id = fixture.controls[0].target_id.clone();
    fixture.controls[0].enabled = false;
    let report = replay_text_editing(
        &fixture,
        &script_with_step(text_step(
            "disabled.insert",
            TextEditFact::insert_text("x").with_target(target_id),
        )),
    );

    assert_suppressed_without_bypass(&report, "insert-text", "target.disabled");
}

#[test]
fn runtime_suppresses_read_only_mutation() {
    let mut fixture = base_controls_text_editing_fixture();
    let target_id = fixture.controls[0].target_id.clone();
    fixture.controls[0].read_only = true;
    let report = replay_text_editing(
        &fixture,
        &script_with_step(text_step(
            "read-only.insert",
            TextEditFact::insert_text("x").with_target(target_id),
        )),
    );

    assert_suppressed_without_bypass(&report, "insert-text", "target.read_only");
}

#[test]
fn runtime_suppresses_range_selection_when_not_supported() {
    let mut fixture = base_controls_text_editing_fixture();
    let target_id = fixture.controls[0].target_id.clone();
    fixture.controls[0].descriptor.selection_policy = ControlEditableTextSelectionPolicy::CaretOnly;
    let report = replay_text_editing(
        &fixture,
        &script_with_step(selection_step(
            "range.unsupported",
            TextSelectionFact::range(TextRange::new(
                TextPosition::grapheme(0),
                TextPosition::grapheme(2),
            ))
            .with_target(target_id),
        )),
    );

    assert_suppressed_without_bypass(&report, "selection", "selection.range_not_supported");
}

#[test]
fn runtime_suppresses_composition_when_not_supported() {
    let mut fixture = base_controls_text_editing_fixture();
    let target_id = fixture.controls[0].target_id.clone();
    fixture.controls[0].descriptor = fixture.controls[0].descriptor.clone().without_composition();
    let report = replay_text_editing(
        &fixture,
        &script_with_step(composition_step(
            "composition.unsupported",
            TextCompositionFact::start("e").with_target(target_id),
        )),
    );

    assert_suppressed_without_bypass(
        &report,
        "composition-start",
        "intent.unsupported_by_descriptor",
    );
}

#[test]
fn runtime_suppresses_focus_for_non_editable_widget() {
    let fixture = base_controls_text_editing_fixture();
    let report = replay_text_editing(
        &fixture,
        &script_with_step(focus_step("focus.non-editable", WidgetId(404))),
    );

    assert_suppressed_without_bypass(&report, "focus", "target.not_editable");
}

#[test]
fn runtime_suppresses_target_not_declared() {
    let fixture = base_controls_text_editing_fixture();
    let report = replay_text_editing(
        &fixture,
        &script_with_step(text_step(
            "missing-target.insert",
            TextEditFact::insert_text("x").with_target("runenwerk.ui.missing"),
        )),
    );

    assert_suppressed_without_bypass(&report, "insert-text", "target.not_declared");
}

fn script_with_step(step: TextEditingReplayStep) -> TextEditingReplayScript {
    TextEditingReplayScript::new("text-editing.report.test").with_step(step)
}

fn focus_step(id: &str, widget_id: WidgetId) -> TextEditingReplayStep {
    TextEditingReplayStep::new(
        id,
        sample(id).with_fact(NormalizedInputFact::Focus(FocusInputFact::target(
            FocusTargetId(widget_id.0),
        ))),
    )
}

fn text_step(id: &str, fact: TextEditFact) -> TextEditingReplayStep {
    TextEditingReplayStep::new(
        id,
        sample(id).with_fact(NormalizedInputFact::TextEdit(fact)),
    )
}

fn selection_step(id: &str, fact: TextSelectionFact) -> TextEditingReplayStep {
    TextEditingReplayStep::new(
        id,
        sample(id).with_fact(NormalizedInputFact::TextSelection(fact)),
    )
}

fn composition_step(id: &str, fact: TextCompositionFact) -> TextEditingReplayStep {
    TextEditingReplayStep::new(
        id,
        sample(id).with_fact(NormalizedInputFact::TextComposition(fact)),
    )
}

fn sample(id: &str) -> NormalizedInputSample {
    NormalizedInputSample::new(format!("sample.{id}"))
}

fn assert_suppressed_without_bypass(report: &TextEditingReport, intent: &str, reason: &str) {
    assert!(report.accepted_edit_intents.is_empty());
    assert!(
        report
            .suppressed_edit_intents
            .iter()
            .any(|suppression| suppression.intent == intent && suppression.reason == reason),
        "expected suppression {intent}/{reason}, got {:?}",
        report.suppressed_edit_intents
    );
    assert!(report.boundary_assertions.no_bypass_evidence());
    assert_eq!(report.boundary_assertions.host_commands_executed, 0);
    assert_eq!(report.boundary_assertions.product_mutations, 0);
    assert_eq!(report.boundary_assertions.authored_ui_edits, 0);
    assert_eq!(report.boundary_assertions.product_undo_redo_operations, 0);
    assert_eq!(report.boundary_assertions.plugin_framework_operations, 0);
}
