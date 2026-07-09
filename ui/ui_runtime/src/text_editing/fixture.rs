//! Text-editing fixture and replay script helpers.

use ui_controls::{ControlEditableTextDescriptor, ControlPackageDescriptor};
use ui_input::{
    FocusInputFact, FocusTargetId, Key, KeyState, KeyboardInputFact, NormalizedInputFact,
    NormalizedInputSample, TextCompositionFact, TextEditFact, TextPosition, TextRange,
    TextSelectionFact,
};
use ui_math::UiRect;

use crate::WidgetId;

#[derive(Debug, Clone, PartialEq)]
pub struct MountedTextEditingFixture {
    pub fixture_id: String,
    pub controls: Vec<MountedEditableTextControl>,
}

impl MountedTextEditingFixture {
    pub fn new(fixture_id: impl Into<String>) -> Self {
        Self {
            fixture_id: fixture_id.into(),
            controls: Vec::new(),
        }
    }

    pub fn with_control(mut self, control: MountedEditableTextControl) -> Self {
        self.controls.push(control);
        self
    }

    pub fn control_by_target(&self, target_id: &str) -> Option<&MountedEditableTextControl> {
        self.controls
            .iter()
            .find(|control| control.target_id == target_id)
    }

    pub fn control_by_widget(&self, widget_id: WidgetId) -> Option<&MountedEditableTextControl> {
        self.controls
            .iter()
            .find(|control| control.widget_id == widget_id)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct MountedEditableTextControl {
    pub widget_id: WidgetId,
    pub target_id: String,
    pub label: String,
    pub bounds: UiRect,
    pub descriptor: ControlEditableTextDescriptor,
    pub enabled: bool,
    pub read_only: bool,
    pub committed_text: String,
    pub selection: TextRange,
    pub composition_text: Option<String>,
}

impl MountedEditableTextControl {
    pub fn new(
        widget_id: WidgetId,
        target_id: impl Into<String>,
        label: impl Into<String>,
        bounds: UiRect,
        descriptor: ControlEditableTextDescriptor,
    ) -> Self {
        Self {
            widget_id,
            target_id: target_id.into(),
            label: label.into(),
            bounds,
            descriptor,
            enabled: true,
            read_only: false,
            committed_text: String::new(),
            selection: TextRange::collapsed(TextPosition::grapheme(0)),
            composition_text: None,
        }
    }

    pub fn disabled(mut self) -> Self {
        self.enabled = false;
        self
    }

    pub fn read_only(mut self) -> Self {
        self.read_only = true;
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TextEditingReplayScript {
    pub replay_id: String,
    pub steps: Vec<TextEditingReplayStep>,
}

impl TextEditingReplayScript {
    pub fn new(replay_id: impl Into<String>) -> Self {
        Self {
            replay_id: replay_id.into(),
            steps: Vec::new(),
        }
    }

    pub fn with_step(mut self, step: TextEditingReplayStep) -> Self {
        self.steps.push(step);
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TextEditingReplayStep {
    pub step_id: String,
    pub sample: NormalizedInputSample,
}

impl TextEditingReplayStep {
    pub fn new(step_id: impl Into<String>, sample: NormalizedInputSample) -> Self {
        Self {
            step_id: step_id.into(),
            sample,
        }
    }
}

pub fn base_controls_text_editing_fixture_from_package(
    package: &ControlPackageDescriptor,
) -> MountedTextEditingFixture {
    let mut fixture = MountedTextEditingFixture::new("base-controls.text-editing.package-fixture");
    for (index, descriptor) in package.editable_text_descriptors.iter().enumerate() {
        let target_id = descriptor.control_kind_id.as_str().to_owned();
        fixture = fixture.with_control(MountedEditableTextControl::new(
            WidgetId(301 + index as u64),
            target_id.clone(),
            target_id,
            UiRect::new(32.0, 40.0 + index as f32 * 44.0, 320.0, 32.0),
            descriptor.clone(),
        ));
    }
    fixture
}

pub fn base_controls_text_editing_fixture() -> MountedTextEditingFixture {
    base_controls_text_editing_fixture_from_package(&ui_controls::runenwerk_control_package())
}

pub fn base_controls_text_editing_positive_script() -> TextEditingReplayScript {
    let target_id = ui_controls::INSPECTOR_FIELD_CONTROL_KIND_ID;
    TextEditingReplayScript::new("base-controls.text-editing.positive")
        .with_step(focus_step("step.focus.inspector", WidgetId(301)))
        .with_step(selection_step(
            "step.place.caret",
            TextSelectionFact::caret(TextPosition::grapheme(0)).with_target(target_id),
        ))
        .with_step(text_step(
            "step.insert.text",
            TextEditFact::insert_text("caf").with_target(target_id),
        ))
        .with_step(key_step("step.move.left", Key::Left))
        .with_step(key_step("step.move.home", Key::Home))
        .with_step(key_step("step.move.end", Key::End))
        .with_step(selection_step(
            "step.select.range",
            TextSelectionFact::range(TextRange::new(
                TextPosition::grapheme(0),
                TextPosition::grapheme(3),
            ))
            .with_target(target_id),
        ))
        .with_step(text_step(
            "step.replace.selection",
            TextEditFact::replace_selection("cafe").with_target(target_id),
        ))
        .with_step(composition_step(
            "step.composition.start",
            TextCompositionFact::start("e").with_target(target_id),
        ))
        .with_step(composition_step(
            "step.composition.update",
            TextCompositionFact::update("e").with_target(target_id),
        ))
        .with_step(composition_step(
            "step.composition.commit",
            TextCompositionFact::commit("e").with_target(target_id),
        ))
        .with_step(composition_step(
            "step.composition.cancel",
            TextCompositionFact::cancel().with_target(target_id),
        ))
        .with_step(key_step("step.delete.backward", Key::Backspace))
        .with_step(key_step("step.delete.forward", Key::Delete))
        .with_step(text_step(
            "step.paste.host-owned-source",
            TextEditFact::source_insert("host.clipboard.plain").with_target(target_id),
        ))
        .with_step(key_step("step.submit.enter", Key::Enter))
        .with_step(key_step("step.cancel.escape", Key::Escape))
}

pub fn base_controls_text_editing_suppression_script() -> TextEditingReplayScript {
    TextEditingReplayScript::new("base-controls.text-editing.suppressed").with_step(text_step(
        "step.no-target.insert",
        TextEditFact::insert_text("blocked"),
    ))
}

fn focus_step(id: &str, widget_id: WidgetId) -> TextEditingReplayStep {
    TextEditingReplayStep::new(
        id,
        NormalizedInputSample::new(format!("sample.{id}")).with_fact(NormalizedInputFact::Focus(
            FocusInputFact::target(FocusTargetId(widget_id.0)),
        )),
    )
}

fn key_step(id: &str, key: Key) -> TextEditingReplayStep {
    TextEditingReplayStep::new(
        id,
        NormalizedInputSample::new(format!("sample.{id}")).with_fact(
            NormalizedInputFact::Keyboard(KeyboardInputFact::new(key, KeyState::Pressed)),
        ),
    )
}

fn text_step(id: &str, fact: TextEditFact) -> TextEditingReplayStep {
    TextEditingReplayStep::new(
        id,
        NormalizedInputSample::new(format!("sample.{id}"))
            .with_fact(NormalizedInputFact::TextEdit(fact)),
    )
}

fn selection_step(id: &str, fact: TextSelectionFact) -> TextEditingReplayStep {
    TextEditingReplayStep::new(
        id,
        NormalizedInputSample::new(format!("sample.{id}"))
            .with_fact(NormalizedInputFact::TextSelection(fact)),
    )
}

fn composition_step(id: &str, fact: TextCompositionFact) -> TextEditingReplayStep {
    TextEditingReplayStep::new(
        id,
        NormalizedInputSample::new(format!("sample.{id}"))
            .with_fact(NormalizedInputFact::TextComposition(fact)),
    )
}
