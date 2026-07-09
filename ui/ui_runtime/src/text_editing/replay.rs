use std::collections::BTreeMap;

use ui_controls::{ControlEditableTextIntent, ControlEditableTextSelectionPolicy};
use ui_input::{
    FocusChange, Key, KeyState, NormalizedInputFact, TextCompositionFact, TextCompositionKind,
    TextEditFact, TextEditIntent, TextPosition, TextRange, TextSelectionFact, TextSelectionReason,
};

use crate::WidgetId;

use super::{
    MountedEditableTextControl, MountedTextEditingFixture, TextEditingBoundaryAssertions,
    TextEditingCaretEvidence, TextEditingCompositionEvidence, TextEditingDescriptorEvidence,
    TextEditingEditIntentEvidence, TextEditingLifecycleState, TextEditingLifecycleTransition,
    TextEditingReplayScript, TextEditingReplayStep, TextEditingReport,
    TextEditingSelectionEvidence, TextEditingSuppressionEvidence, TextEditingValueEvidence,
};

pub fn replay_text_editing(
    fixture: &MountedTextEditingFixture,
    script: &TextEditingReplayScript,
) -> TextEditingReport {
    let mut state = TextEditingReplayState::new(fixture);
    let mut report = empty_report(fixture, script);
    for step in &script.steps {
        report.input_steps.push(step.step_id.clone());
        for fact in &step.sample.facts {
            apply_fact(step, fact, &mut state, &mut report);
        }
    }
    report
}

struct TextEditingReplayState {
    controls: Vec<MountedEditableTextControl>,
    focused_target_id: Option<String>,
    lifecycle: BTreeMap<String, TextEditingLifecycleState>,
}

impl TextEditingReplayState {
    fn new(fixture: &MountedTextEditingFixture) -> Self {
        let lifecycle = fixture
            .controls
            .iter()
            .map(|control| {
                (
                    control.target_id.clone(),
                    TextEditingLifecycleState::Unfocused,
                )
            })
            .collect();
        Self {
            controls: fixture.controls.clone(),
            focused_target_id: None,
            lifecycle,
        }
    }

    fn control_index_by_target(&self, target_id: &str) -> Option<usize> {
        self.controls
            .iter()
            .position(|control| control.target_id == target_id)
    }

    fn control_index_by_widget(&self, widget_id: WidgetId) -> Option<usize> {
        self.controls
            .iter()
            .position(|control| control.widget_id == widget_id)
    }

    fn resolved_target(&self, explicit: Option<&str>) -> Option<String> {
        explicit
            .map(str::to_owned)
            .or_else(|| self.focused_target_id.clone())
    }
}

fn empty_report(
    fixture: &MountedTextEditingFixture,
    script: &TextEditingReplayScript,
) -> TextEditingReport {
    TextEditingReport {
        replay_id: script.replay_id.clone(),
        fixture_id: fixture.fixture_id.clone(),
        input_steps: Vec::new(),
        descriptor_evidence: descriptor_evidence(fixture),
        lifecycle_transitions: Vec::new(),
        caret_evidence: Vec::new(),
        selection_evidence: Vec::new(),
        composition_evidence: Vec::new(),
        value_evidence: Vec::new(),
        accepted_edit_intents: Vec::new(),
        suppressed_edit_intents: Vec::new(),
        boundary_assertions: TextEditingBoundaryAssertions::default(),
    }
}

fn descriptor_evidence(fixture: &MountedTextEditingFixture) -> Vec<TextEditingDescriptorEvidence> {
    fixture
        .controls
        .iter()
        .map(|control| TextEditingDescriptorEvidence {
            target_id: control.target_id.clone(),
            widget_id: control.widget_id,
            control_kind_id: control.descriptor.control_kind_id.as_str().to_owned(),
            mode: control.descriptor.mode.as_str().to_owned(),
            supported_intents: control
                .descriptor
                .supported_intents
                .iter()
                .map(|intent| intent.as_str().to_owned())
                .collect(),
            selection_policy: control.descriptor.selection_policy.as_str().to_owned(),
            composition_policy: control.descriptor.composition_policy.as_str().to_owned(),
            host_owned_mutation: control.descriptor.host_owned_mutation,
            proof_required: control.descriptor.proof_required,
        })
        .collect()
}

fn apply_fact(
    step: &TextEditingReplayStep,
    fact: &NormalizedInputFact,
    state: &mut TextEditingReplayState,
    report: &mut TextEditingReport,
) {
    match fact {
        NormalizedInputFact::Focus(focus) => {
            if let FocusChange::Set(target) = focus.change {
                focus_target(step, WidgetId(target.0), state, report);
            }
        }
        NormalizedInputFact::Keyboard(keyboard) if keyboard.state == KeyState::Pressed => {
            apply_keyboard(step, &keyboard.key, state, report)
        }
        NormalizedInputFact::TextEdit(fact) => apply_text_edit(step, fact, state, report),
        NormalizedInputFact::TextSelection(fact) => apply_selection(step, fact, state, report),
        NormalizedInputFact::TextComposition(fact) => apply_composition(step, fact, state, report),
        NormalizedInputFact::Pointer(_)
        | NormalizedInputFact::Keyboard(_)
        | NormalizedInputFact::Semantic(_)
        | NormalizedInputFact::TextIntent(_) => {}
    }
}

fn focus_target(
    step: &TextEditingReplayStep,
    widget_id: WidgetId,
    state: &mut TextEditingReplayState,
    report: &mut TextEditingReport,
) {
    let Some(index) = state.control_index_by_widget(widget_id) else {
        suppress(step, None, "focus", "target.not_editable", report);
        return;
    };
    let target_id = state.controls[index].target_id.clone();
    state.focused_target_id = Some(target_id.clone());
    transition(
        step,
        &target_id,
        TextEditingLifecycleState::Focused,
        "focus.gained",
        state,
        report,
    );
}

fn apply_keyboard(
    step: &TextEditingReplayStep,
    key: &Key,
    state: &mut TextEditingReplayState,
    report: &mut TextEditingReport,
) {
    match key {
        Key::Backspace => {
            request_intent(
                step,
                None,
                ControlEditableTextIntent::DeleteBackward,
                "",
                None,
                state,
                report,
            );
        }
        Key::Delete => {
            request_intent(
                step,
                None,
                ControlEditableTextIntent::DeleteForward,
                "",
                None,
                state,
                report,
            );
        }
        Key::Left | Key::Right | Key::Home | Key::End => {
            let accepted = request_intent(
                step,
                None,
                ControlEditableTextIntent::MoveCaret,
                "",
                None,
                state,
                report,
            );
            if accepted
                && let Some(target_id) = state.focused_target_id.clone()
                && let Some(index) = state.control_index_by_target(&target_id)
            {
                move_caret_for_key(&mut state.controls[index], key);
                let position = caret_label(&state.controls[index]);
                report.caret_evidence.push(TextEditingCaretEvidence {
                    step_id: step.step_id.clone(),
                    target_id: target_id.clone(),
                    position,
                    reason: "keyboard-navigation".to_owned(),
                    accepted: true,
                });
                report.boundary_assertions.caret_moves += 1;
                record_value(step, &target_id, "caret-moved", state, report);
            }
        }
        Key::Enter => {
            request_intent(
                step,
                None,
                ControlEditableTextIntent::Submit,
                "",
                None,
                state,
                report,
            );
        }
        Key::Escape => {
            request_intent(
                step,
                None,
                ControlEditableTextIntent::Cancel,
                "",
                None,
                state,
                report,
            );
        }
        _ => {}
    }
}

fn apply_text_edit(
    step: &TextEditingReplayStep,
    fact: &TextEditFact,
    state: &mut TextEditingReplayState,
    report: &mut TextEditingReport,
) {
    let intent = text_edit_intent(fact.intent);
    let accepted = request_intent(
        step,
        fact.target_id.as_deref(),
        intent,
        &fact.text,
        fact.host_owned_source.clone(),
        state,
        report,
    );
    if !accepted {
        return;
    }
    let Some(target_id) = state.resolved_target(fact.target_id.as_deref()) else {
        return;
    };
    let Some(index) = state.control_index_by_target(&target_id) else {
        return;
    };
    match fact.intent {
        TextEditIntent::InsertText => {
            replace_current_selection(&mut state.controls[index], &fact.text);
            record_value(step, &target_id, "insert-text", state, report);
        }
        TextEditIntent::ReplaceSelection => {
            replace_current_selection(&mut state.controls[index], &fact.text);
            record_value(step, &target_id, "replace-selection", state, report);
        }
        TextEditIntent::DeleteBackward => {
            delete_backward(&mut state.controls[index]);
            record_value(step, &target_id, "delete-backward", state, report);
        }
        TextEditIntent::DeleteForward => {
            delete_forward(&mut state.controls[index]);
            record_value(step, &target_id, "delete-forward", state, report);
        }
        TextEditIntent::MoveCaret
        | TextEditIntent::ExtendSelection
        | TextEditIntent::Submit
        | TextEditIntent::Cancel
        | TextEditIntent::SourceInsert
        | TextEditIntent::Copy
        | TextEditIntent::Cut => {}
    }
}

fn apply_selection(
    step: &TextEditingReplayStep,
    fact: &TextSelectionFact,
    state: &mut TextEditingReplayState,
    report: &mut TextEditingReport,
) {
    let Some(target_id) = state.resolved_target(fact.target_id.as_deref()) else {
        suppress(step, None, "selection", "target.missing", report);
        return;
    };
    let Some(index) = state.control_index_by_target(&target_id) else {
        suppress(
            step,
            Some(target_id),
            "selection",
            "target.not_declared",
            report,
        );
        return;
    };
    if !fact.range.is_collapsed()
        && state.controls[index].descriptor.selection_policy
            != ControlEditableTextSelectionPolicy::RangeSelection
    {
        suppress(
            step,
            Some(target_id),
            "selection",
            "selection.range_not_supported",
            report,
        );
        return;
    }
    let reason = fact.reason.as_str().to_owned();
    state.controls[index].selection = fact.range;
    report
        .selection_evidence
        .push(TextEditingSelectionEvidence {
            step_id: step.step_id.clone(),
            target_id: target_id.clone(),
            anchor: position_label(fact.range.anchor),
            extent: position_label(fact.range.extent),
            reason,
            collapsed: fact.range.is_collapsed(),
            accepted: true,
        });
    report.boundary_assertions.selection_changes += 1;
    if fact.reason == TextSelectionReason::CaretMove || fact.range.is_collapsed() {
        report.caret_evidence.push(TextEditingCaretEvidence {
            step_id: step.step_id.clone(),
            target_id: target_id.clone(),
            position: position_label(fact.range.extent),
            reason: fact.reason.as_str().to_owned(),
            accepted: true,
        });
        report.boundary_assertions.caret_moves += 1;
    }
    transition(
        step,
        &target_id,
        if fact.range.is_collapsed() {
            TextEditingLifecycleState::Focused
        } else {
            TextEditingLifecycleState::Selecting
        },
        fact.reason.as_str(),
        state,
        report,
    );
    record_value(step, &target_id, fact.reason.as_str(), state, report);
}

fn apply_composition(
    step: &TextEditingReplayStep,
    fact: &TextCompositionFact,
    state: &mut TextEditingReplayState,
    report: &mut TextEditingReport,
) {
    let intent = composition_intent(fact.kind);
    let accepted = request_intent(
        step,
        fact.target_id.as_deref(),
        intent,
        &fact.text,
        None,
        state,
        report,
    );
    if !accepted {
        return;
    }
    let Some(target_id) = state.resolved_target(fact.target_id.as_deref()) else {
        return;
    };
    if let Some(index) = state.control_index_by_target(&target_id) {
        match fact.kind {
            TextCompositionKind::Start | TextCompositionKind::Update => {
                state.controls[index].composition_text = Some(fact.text.clone());
            }
            TextCompositionKind::Commit => {
                let committed = if fact.text.is_empty() {
                    state.controls[index]
                        .composition_text
                        .clone()
                        .unwrap_or_default()
                } else {
                    fact.text.clone()
                };
                replace_current_selection(&mut state.controls[index], &committed);
                state.controls[index].composition_text = None;
            }
            TextCompositionKind::Cancel => {
                state.controls[index].composition_text = None;
            }
        }
    }
    report
        .composition_evidence
        .push(TextEditingCompositionEvidence {
            step_id: step.step_id.clone(),
            target_id: target_id.clone(),
            kind: fact.kind.as_str().to_owned(),
            text: fact.text.clone(),
            accepted: true,
        });
    report.boundary_assertions.composition_events += 1;
    transition(
        step,
        &target_id,
        match fact.kind {
            TextCompositionKind::Start | TextCompositionKind::Update => {
                TextEditingLifecycleState::Composing
            }
            TextCompositionKind::Commit => TextEditingLifecycleState::Editing,
            TextCompositionKind::Cancel => TextEditingLifecycleState::Focused,
        },
        fact.kind.as_str(),
        state,
        report,
    );
    record_value(step, &target_id, fact.kind.as_str(), state, report);
}

#[allow(clippy::too_many_arguments)]
fn request_intent(
    step: &TextEditingReplayStep,
    explicit_target_id: Option<&str>,
    intent: ControlEditableTextIntent,
    text: &str,
    host_owned_source: Option<String>,
    state: &mut TextEditingReplayState,
    report: &mut TextEditingReport,
) -> bool {
    let Some(target_id) = state.resolved_target(explicit_target_id) else {
        suppress(step, None, intent.as_str(), "target.missing", report);
        return false;
    };
    let Some(index) = state.control_index_by_target(&target_id) else {
        suppress(
            step,
            Some(target_id),
            intent.as_str(),
            "target.not_declared",
            report,
        );
        return false;
    };
    let control = &state.controls[index];
    if !control.enabled {
        suppress(
            step,
            Some(target_id),
            intent.as_str(),
            "target.disabled",
            report,
        );
        return false;
    }
    if control.read_only && intent.mutates_transient_text() {
        suppress(
            step,
            Some(target_id),
            intent.as_str(),
            "target.read_only",
            report,
        );
        return false;
    }
    if !control.descriptor.supported_intents.contains(&intent) {
        let reason = if intent == ControlEditableTextIntent::Paste && host_owned_source.is_some() {
            "host_owned_source.unsupported_by_descriptor"
        } else {
            "intent.unsupported_by_descriptor"
        };
        suppress(step, Some(target_id), intent.as_str(), reason, report);
        return false;
    }

    report
        .accepted_edit_intents
        .push(TextEditingEditIntentEvidence {
            step_id: step.step_id.clone(),
            input_sample_id: step.sample.sample_id.clone(),
            target_id: target_id.clone(),
            intent: intent.as_str().to_owned(),
            text: text.to_owned(),
            host_owned_source,
            reason: "descriptor.accepted".to_owned(),
        });
    report.boundary_assertions.accepted_edit_intents += 1;
    transition(
        step,
        &target_id,
        match intent {
            ControlEditableTextIntent::Submit => TextEditingLifecycleState::Submitting,
            ControlEditableTextIntent::Cancel => TextEditingLifecycleState::Cancelled,
            ControlEditableTextIntent::ExtendSelection
            | ControlEditableTextIntent::ReplaceSelection => TextEditingLifecycleState::Selecting,
            _ => TextEditingLifecycleState::Editing,
        },
        intent.as_str(),
        state,
        report,
    );
    true
}

fn transition(
    step: &TextEditingReplayStep,
    target_id: &str,
    to: TextEditingLifecycleState,
    reason: &str,
    state: &mut TextEditingReplayState,
    report: &mut TextEditingReport,
) {
    let from = state
        .lifecycle
        .get(target_id)
        .copied()
        .unwrap_or(TextEditingLifecycleState::Unfocused);
    if from == to && reason != "focus.gained" {
        return;
    }
    state.lifecycle.insert(target_id.to_owned(), to);
    report
        .lifecycle_transitions
        .push(TextEditingLifecycleTransition {
            step_id: step.step_id.clone(),
            target_id: target_id.to_owned(),
            from,
            to,
            reason: reason.to_owned(),
        });
    report.boundary_assertions.lifecycle_transitions += 1;
}

fn suppress(
    step: &TextEditingReplayStep,
    target_id: Option<String>,
    intent: &str,
    reason: &str,
    report: &mut TextEditingReport,
) {
    report
        .suppressed_edit_intents
        .push(TextEditingSuppressionEvidence {
            step_id: step.step_id.clone(),
            input_sample_id: step.sample.sample_id.clone(),
            target_id,
            intent: intent.to_owned(),
            reason: reason.to_owned(),
            host_commands_executed: 0,
            product_mutations: 0,
            authored_ui_edits: 0,
            product_undo_redo_operations: 0,
            plugin_framework_operations: 0,
        });
    report.boundary_assertions.suppressed_edit_intents += 1;
}

fn record_value(
    step: &TextEditingReplayStep,
    target_id: &str,
    reason: &str,
    state: &TextEditingReplayState,
    report: &mut TextEditingReport,
) {
    let Some(index) = state.control_index_by_target(target_id) else {
        return;
    };
    let control = &state.controls[index];
    report.value_evidence.push(TextEditingValueEvidence {
        step_id: step.step_id.clone(),
        target_id: target_id.to_owned(),
        committed_text: control.committed_text.clone(),
        composition_text: control.composition_text.clone(),
        rendered_value: rendered_value(control),
        caret: caret_label(control),
        selection: range_label(control.selection),
        reason: reason.to_owned(),
    });
}

fn replace_current_selection(control: &mut MountedEditableTextControl, text: &str) {
    let len = control.committed_text.chars().count();
    let (start, end) = range_indices(control.selection, len);
    control.committed_text = replace_char_range(&control.committed_text, start, end, text);
    set_caret(control, start + text.chars().count());
}

fn delete_backward(control: &mut MountedEditableTextControl) {
    let len = control.committed_text.chars().count();
    let (start, end) = range_indices(control.selection, len);
    if start != end {
        control.committed_text = replace_char_range(&control.committed_text, start, end, "");
        set_caret(control, start);
        return;
    }
    if start == 0 {
        set_caret(control, 0);
        return;
    }
    control.committed_text = replace_char_range(&control.committed_text, start - 1, start, "");
    set_caret(control, start - 1);
}

fn delete_forward(control: &mut MountedEditableTextControl) {
    let len = control.committed_text.chars().count();
    let (start, end) = range_indices(control.selection, len);
    if start != end {
        control.committed_text = replace_char_range(&control.committed_text, start, end, "");
        set_caret(control, start);
        return;
    }
    if start >= len {
        set_caret(control, len);
        return;
    }
    control.committed_text = replace_char_range(&control.committed_text, start, start + 1, "");
    set_caret(control, start);
}

fn move_caret_for_key(control: &mut MountedEditableTextControl, key: &Key) {
    let len = control.committed_text.chars().count();
    let (_, end) = range_indices(control.selection, len);
    let next = match key {
        Key::Left => end.saturating_sub(1),
        Key::Right => (end + 1).min(len),
        Key::Home => 0,
        Key::End => len,
        _ => end,
    };
    set_caret(control, next);
}

fn set_caret(control: &mut MountedEditableTextControl, ordinal: usize) {
    let position = TextPosition::grapheme(ordinal as u32);
    control.selection = TextRange::collapsed(position);
}

fn range_indices(range: TextRange, text_len: usize) -> (usize, usize) {
    let anchor = range.anchor.ordinal as usize;
    let extent = range.extent.ordinal as usize;
    let start = anchor.min(extent).min(text_len);
    let end = anchor.max(extent).min(text_len);
    (start, end)
}

fn replace_char_range(value: &str, start: usize, end: usize, replacement: &str) -> String {
    let prefix = value.chars().take(start);
    let suffix = value.chars().skip(end);
    prefix.chain(replacement.chars()).chain(suffix).collect()
}

fn rendered_value(control: &MountedEditableTextControl) -> String {
    let Some(composition) = &control.composition_text else {
        return control.committed_text.clone();
    };
    let len = control.committed_text.chars().count();
    let (_, caret) = range_indices(control.selection, len);
    let marker = format!("[{composition} composing]");
    replace_char_range(&control.committed_text, caret, caret, &marker)
}

fn caret_label(control: &MountedEditableTextControl) -> String {
    position_label(control.selection.extent)
}

fn range_label(range: TextRange) -> String {
    format!(
        "{}..{}",
        position_label(range.anchor),
        position_label(range.extent)
    )
}

fn text_edit_intent(intent: TextEditIntent) -> ControlEditableTextIntent {
    match intent {
        TextEditIntent::InsertText => ControlEditableTextIntent::InsertText,
        TextEditIntent::DeleteBackward => ControlEditableTextIntent::DeleteBackward,
        TextEditIntent::DeleteForward => ControlEditableTextIntent::DeleteForward,
        TextEditIntent::ReplaceSelection => ControlEditableTextIntent::ReplaceSelection,
        TextEditIntent::MoveCaret => ControlEditableTextIntent::MoveCaret,
        TextEditIntent::ExtendSelection => ControlEditableTextIntent::ExtendSelection,
        TextEditIntent::Submit => ControlEditableTextIntent::Submit,
        TextEditIntent::Cancel => ControlEditableTextIntent::Cancel,
        TextEditIntent::SourceInsert => ControlEditableTextIntent::Paste,
        TextEditIntent::Copy => ControlEditableTextIntent::Copy,
        TextEditIntent::Cut => ControlEditableTextIntent::Cut,
    }
}

fn composition_intent(kind: TextCompositionKind) -> ControlEditableTextIntent {
    match kind {
        TextCompositionKind::Start => ControlEditableTextIntent::CompositionStart,
        TextCompositionKind::Update => ControlEditableTextIntent::CompositionUpdate,
        TextCompositionKind::Commit => ControlEditableTextIntent::CompositionCommit,
        TextCompositionKind::Cancel => ControlEditableTextIntent::CompositionCancel,
    }
}

fn position_label(position: TextPosition) -> String {
    match (position.line, position.column) {
        (Some(line), Some(column)) => {
            format!("{}:{line}:{column}", position.unit.as_str())
        }
        _ => format!("{}:{}", position.unit.as_str(), position.ordinal),
    }
}
