use core::fmt;
use std::rc::Rc;

use crate::{
    EditIntent, EditableContribution, EditableContributionError, EditingSessionPolicy, ElementId,
    EventContext, EventPhase, Focusability, HitContribution, HitContributionContext, IntoElementId,
    KeyboardCompositionState, KeyboardPhase, LogicalKey, SemanticAction, SemanticContribution,
    SemanticContributionContext, SemanticEditable, SemanticEditableMode, SemanticInvalidState,
    SemanticNodeContribution, SemanticRange, SemanticReference, SemanticRelationship,
    SemanticRelationshipKind, SemanticRole, SemanticState, TextDocumentSnapshot, TextNewlinePolicy,
    TextSelection, TextSensitivity, UiEvent, WidgetInvalidation, WidgetMeasure, WidgetMeasureInput,
    WidgetTextInput, WidgetUpdateContext, ChildBearingWidget,
    SemanticAutocomplete, SemanticPopupKind,
    element::{
        AuthoringDiagnostic, CommonNodeAuthoring, Element, View, common_node_builder_methods,
    },
    widget_erasure::WidgetAdapter,
    widget_protocol::{Widget, WidgetActivation},
};

#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TextFieldError {
    InvalidSelection(EditableContributionError),
    /// The application supplied a durable newline to a single-line source.
    SingleLineSourceContainsNewline,
    /// Secret/password authoring cannot use the multiline semantic contract.
    SecretRequiresSingleLine,
}

impl fmt::Display for TextFieldError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidSelection(error) => write!(f, "{error}"),
            Self::SingleLineSourceContainsNewline => {
                f.write_str("single-line text field source contains a line break")
            }
            Self::SecretRequiresSingleLine => {
                f.write_str("secret text field requires single-line mode")
            }
        }
    }
}
impl std::error::Error for TextFieldError {}

/// Host-neutral public text-input facade. M10 remains the only editing authority.
///
/// Public single-/multiline inputs and explicitly classified single-line
/// passwords share one widget, M10 editor and M8 retained shaping authority.
pub struct TextField<Action> {
    snapshot: TextDocumentSnapshot,
    content: String,
    selection: TextSelection,
    mode: SemanticEditableMode,
    sensitivity: TextSensitivity,
    placeholder: Option<String>,
    labelled_by: Option<ElementId>,
    described_by: Option<ElementId>,
    error_message: Option<ElementId>,
    invalid: Option<SemanticInvalidState>,
    required: bool,
    read_only: bool,
    disabled: bool,
    mapper: Rc<dyn Fn(EditIntent) -> Action>,
    on_submit: Option<Box<dyn FnMut() -> Action>>,
    spin: Option<SpinEditing<Action>>,
    combo: Option<crate::combo_box::ComboWidget<Action>>,
    combo_popup: Option<Element<Action>>,
    combo_autocomplete: Option<SemanticAutocomplete>,
    common: CommonNodeAuthoring,
}

impl<Action> fmt::Debug for TextField<Action> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut debug = f.debug_struct("TextField");
        debug
            .field("snapshot", &self.snapshot)
            .field("mode", &self.mode)
            .field("sensitivity", &self.sensitivity)
            .field("read_only", &self.read_only)
            .field("disabled", &self.disabled);
        if self.sensitivity == TextSensitivity::Public {
            debug
                .field("content_bytes", &self.content.len())
                .field("selection", &self.selection);
        }
        debug.finish_non_exhaustive()
    }
}

impl<Action> TextField<Action> {
    /// Creates a checked, application-owned input field.
    ///
    /// # Errors
    ///
    /// Returns an error for a foreign/invalid selection, or a pre-existing
    /// newline in a single-line application source. All later incoming line
    /// breaks are normalized once in M10 before application edit proposals.
    pub fn new(
        snapshot: TextDocumentSnapshot,
        content: impl Into<String>,
        selection: TextSelection,
        mode: SemanticEditableMode,
        on_edit: impl Fn(EditIntent) -> Action + 'static,
    ) -> Result<Self, TextFieldError> {
        let content = content.into();
        if mode == SemanticEditableMode::SingleLine
            && content.chars().any(TextNewlinePolicy::is_hard_line_break)
        {
            return Err(TextFieldError::SingleLineSourceContainsNewline);
        }
        EditableContribution::<()>::new_read_only(
            snapshot,
            content.as_str(),
            selection,
            TextSensitivity::Public,
            false,
            EditingSessionPolicy::PreserveExact,
        )
        .map_err(TextFieldError::InvalidSelection)?;
        Ok(Self {
            snapshot,
            content,
            selection,
            mode,
            sensitivity: TextSensitivity::Public,
            placeholder: None,
            labelled_by: None,
            described_by: None,
            error_message: None,
            invalid: None,
            required: false,
            read_only: false,
            disabled: false,
            mapper: Rc::new(on_edit),
            on_submit: None,
            spin: None,
            combo: None,
            combo_popup: None,
            combo_autocomplete: None,
            common: CommonNodeAuthoring::default(),
        })
    }

    common_node_builder_methods!();

    pub(crate) fn with_spin_button(
        mut self,
        label: String,
        range: SemanticRange,
        on_adjust: Option<Box<dyn FnMut(crate::SpinButtonRequest) -> Action>>,
    ) -> Self {
        self.spin = Some(SpinEditing {
            label,
            range,
            on_adjust,
        });
        self
    }

    pub(crate) fn permits_combo_box(&self) -> bool {
        self.mode == SemanticEditableMode::SingleLine
            && self.sensitivity == TextSensitivity::Public
            && self.spin.is_none()
            && self.combo.is_none()
    }
    pub(crate) fn combo_owner_id(&self) -> Option<&ElementId> {
        self.common.id.as_ref()
    }
    pub(crate) const fn combo_editor_enabled(&self) -> bool {
        !self.disabled
    }
    pub(crate) fn with_combo_box(
        mut self,
        widget: crate::combo_box::ComboWidget<Action>,
        popup: Option<Element<Action>>,
        autocomplete: SemanticAutocomplete,
    ) -> Self {
        self.combo = Some(widget);
        self.combo_popup = popup;
        self.combo_autocomplete = Some(autocomplete);
        self
    }

    pub(crate) fn with_common_node(mut self, common: CommonNodeAuthoring) -> Self {
        self.common = common;
        self
    }

    /// Marks a checked single-line field as a secret/password presentation.
    /// The M10 sensitivity authority disables ordinary clipboard disclosure,
    /// while M8 shapes the already accepted grapheme-masked projection.
    ///
    /// # Errors
    ///
    /// A multiline field cannot claim secret sensitivity.
    pub fn password(mut self) -> Result<Self, TextFieldError> {
        if self.mode != SemanticEditableMode::SingleLine {
            return Err(TextFieldError::SecretRequiresSingleLine);
        }
        self.sensitivity = TextSensitivity::Secret;
        Ok(self)
    }

    #[must_use]
    pub fn placeholder(mut self, value: impl Into<String>) -> Self {
        self.placeholder = Some(value.into());
        self
    }

    #[must_use]
    pub const fn read_only(mut self, read_only: bool) -> Self {
        self.read_only = read_only;
        self
    }

    #[must_use]
    pub const fn disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self
    }

    #[must_use]
    pub const fn required(mut self, required: bool) -> Self {
        self.required = required;
        self
    }

    #[must_use]
    pub const fn invalid(mut self, invalid: SemanticInvalidState) -> Self {
        self.invalid = Some(invalid);
        self
    }

    #[must_use]
    pub fn labelled_by(mut self, target: impl IntoElementId) -> Self {
        self.labelled_by = self.relationship_target("labelled_by", target);
        self
    }

    #[must_use]
    pub fn described_by(mut self, target: impl IntoElementId) -> Self {
        self.described_by = self.relationship_target("described_by", target);
        self
    }

    #[must_use]
    pub fn error_message(mut self, target: impl IntoElementId) -> Self {
        self.error_message = self.relationship_target("error_message", target);
        self
    }

    #[must_use]
    pub fn on_submit(mut self, callback: impl FnMut() -> Action + 'static) -> Self {
        self.on_submit = Some(Box::new(callback));
        self
    }

    fn relationship_target(
        &mut self,
        field: &'static str,
        target: impl IntoElementId,
    ) -> Option<ElementId> {
        match target.into_element_id() {
            Ok(id) => Some(id),
            Err((value, error)) => {
                self.common.diagnostics.push(AuthoringDiagnostic {
                    field,
                    value,
                    error,
                });
                None
            }
        }
    }
}

#[derive(Clone, PartialEq)]
struct TextFieldState {
    snapshot: TextDocumentSnapshot,
    content: String,
    selection: TextSelection,
    mode: SemanticEditableMode,
    sensitivity: TextSensitivity,
    placeholder: Option<String>,
    labelled_by: Option<ElementId>,
    described_by: Option<ElementId>,
    error_message: Option<ElementId>,
    invalid: Option<SemanticInvalidState>,
    required: bool,
    read_only: bool,
    disabled: bool,
    spin: Option<SpinFacts>,
    combo: Option<crate::combo_box::ComboState>,
    combo_autocomplete: Option<SemanticAutocomplete>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct SpinFacts {
    label: String,
    range: SemanticRange,
    actionable: bool,
}

struct SpinEditing<Action> {
    label: String,
    range: SemanticRange,
    on_adjust: Option<Box<dyn FnMut(crate::SpinButtonRequest) -> Action>>,
}

impl fmt::Debug for TextFieldState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut debug = f.debug_struct("TextFieldState");
        debug
            .field("snapshot", &self.snapshot)
            .field("mode", &self.mode)
            .field("sensitivity", &self.sensitivity)
            .field("read_only", &self.read_only)
            .field("disabled", &self.disabled);
        if self.sensitivity == TextSensitivity::Public {
            debug
                .field("content_bytes", &self.content.len())
                .field("selection", &self.selection);
        }
        debug.finish_non_exhaustive()
    }
}

struct TextFieldWidget<Action> {
    state: TextFieldState,
    mapper: Rc<dyn Fn(EditIntent) -> Action>,
    on_submit: Option<Box<dyn FnMut() -> Action>>,
    on_adjust: Option<Box<dyn FnMut(crate::SpinButtonRequest) -> Action>>,
    combo: Option<crate::combo_box::ComboWidget<Action>>,
}

impl<Action> fmt::Debug for TextFieldWidget<Action> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TextFieldWidget")
            .field("state", &self.state)
            .finish_non_exhaustive()
    }
}

impl<Action: 'static> Widget<Action> for TextFieldWidget<Action> {
    type State = TextFieldState;

    fn create_state(&self) -> Self::State {
        self.state.clone()
    }

    fn update(&self, state: &mut Self::State, context: &mut WidgetUpdateContext<Action>) {
        if state.content != self.state.content
            || state.mode != self.state.mode
            || state.sensitivity != self.state.sensitivity
        {
            // Mode changes wrapping and thus the retained M8 text geometry.
            context.invalidate(
                WidgetInvalidation::LAYOUT
                    | WidgetInvalidation::PAINT
                    | WidgetInvalidation::SEMANTICS,
            );
        } else {
            if state.snapshot != self.state.snapshot || state.selection != self.state.selection {
                context.invalidate(WidgetInvalidation::PAINT | WidgetInvalidation::SEMANTICS);
            }
            if state.content.is_empty() && state.placeholder != self.state.placeholder {
                context.invalidate(WidgetInvalidation::LAYOUT | WidgetInvalidation::PAINT);
            }
            if state.placeholder != self.state.placeholder
                || state.labelled_by != self.state.labelled_by
                || state.described_by != self.state.described_by
                || state.error_message != self.state.error_message
                || state.invalid != self.state.invalid
                || state.required != self.state.required
            {
                context.invalidate(WidgetInvalidation::SEMANTICS);
            }
        }
        if state.spin != self.state.spin {
            context.invalidate(WidgetInvalidation::SEMANTICS | WidgetInvalidation::INTERACTION);
        }
        if state.combo_autocomplete != self.state.combo_autocomplete {
            context.invalidate(WidgetInvalidation::SEMANTICS);
        }
        if state.combo.is_some() != self.state.combo.is_some() {
            context.invalidate(WidgetInvalidation::SEMANTICS | WidgetInvalidation::INTERACTION);
        }
        if state.read_only != self.state.read_only || state.disabled != self.state.disabled {
            context.invalidate(
                WidgetInvalidation::INTERACTION
                    | WidgetInvalidation::PAINT
                    | WidgetInvalidation::SEMANTICS,
            );
        }
        let mut next = self.state.clone();
        if let (Some(combo), Some(current)) = (self.combo.as_ref(), state.combo.as_mut()) {
            combo.update(current, context);
            next.combo = Some(current.clone());
        }
        state.clone_from(&next);
    }

    fn activation(&self, state: &Self::State) -> WidgetActivation {
        if self.state.disabled {
            WidgetActivation::disabled()
        } else if let (Some(combo), Some(current)) = (self.combo.as_ref(), state.combo.as_ref()) {
            combo.activation(current)
        } else {
            WidgetActivation::NONE
        }
    }

    fn activate(
        &mut self,
        state: &mut Self::State,
        context: &mut crate::WidgetActivationContext<Action>,
    ) -> crate::WidgetActivationOutput<Action> {
        if let (Some(combo), Some(current)) = (self.combo.as_mut(), state.combo.as_mut()) {
            combo.activate(current, context)
        } else {
            crate::WidgetActivationOutput::none()
        }
    }

    fn text_input(&self, _: &Self::State) -> WidgetTextInput {
        let enabled = !self.state.disabled && !self.state.read_only
            && self.combo.as_ref().is_none_or(|combo| combo.invalid.is_none());
        WidgetTextInput::new(enabled, enabled)
    }

    fn event(
        &mut self,
        state: &mut Self::State,
        event: &UiEvent,
        context: &mut EventContext<'_, Action>,
    ) -> crate::WidgetEventOutput {
        if let (Some(combo), Some(current)) = (self.combo.as_mut(), state.combo.as_mut()) {
            combo.proxy_event(current, event, context, true);
        }
        if context.phase() == EventPhase::Target
            && !context.default_is_prevented()
            && self.state.mode == SemanticEditableMode::SingleLine
            && !self.state.disabled
            && let Some(key) = event.as_keyboard()
            && key.phase() == KeyboardPhase::Down
            && key.logical_key() == &LogicalKey::Enter
            && key.modifiers() == crate::KeyModifiers::NONE
            && key.composition_state() == KeyboardCompositionState::Inactive
        {
            context.prevent_default();
            if !key.is_repeat()
                && let Some(submit) = self.on_submit.as_mut()
            {
                context.emit(submit());
            }
        }
        if context.phase() == EventPhase::Target
            && !context.default_is_prevented()
            && !self.state.disabled
            && !self.state.read_only
            && let Some(spin) = self.state.spin.as_ref()
            && spin.actionable
            && let Some(callback) = self.on_adjust.as_mut()
        {
            let request = event.as_semantic_command().map_or_else(
                || {
                    event
                        .as_keyboard()
                        .and_then(|key| crate::spin_button::keyboard_request(&spin.range, key))
                },
                |command| crate::spin_button::semantic_request(&spin.range, command.command()),
            );
            if let Some(request) = request {
                context.emit(callback(request));
                context.prevent_default();
                context.stop_propagation();
            }
        }
        crate::WidgetEventOutput::none()
    }

    fn editable(&self, _: &Self::State) -> Option<EditableContribution<Action>> {
        if self.combo.as_ref().is_some_and(|combo| combo.invalid.is_some()) {
            return None;
        }
        let mapper = Rc::clone(&self.mapper);
        let newline = if self.state.mode == SemanticEditableMode::SingleLine {
            TextNewlinePolicy::ReplaceWithSpace
        } else {
            TextNewlinePolicy::Preserve
        };
        EditableContribution::new(
            self.state.snapshot,
            self.state.content.as_str(),
            self.state.selection,
            self.state.sensitivity,
            self.state.read_only,
            self.state.disabled,
            EditingSessionPolicy::PreserveExact,
            move |intent| mapper(intent),
        )
        .ok()
        .map(|contribution| contribution.with_newline_policy(newline))
    }

    fn measure(&self, _: &Self::State, _: WidgetMeasureInput) -> WidgetMeasure {
        let wrap = if self.state.mode == SemanticEditableMode::SingleLine {
            crate::TextLeafWrap::NoWrap
        } else {
            crate::TextLeafWrap::Wrap
        };
        let descriptor =
            crate::TextLeafMeasure::new(self.state.content.clone()).with_wrap_mode(wrap);
        let descriptor = if self.state.content.is_empty()
            && let Some(hint) = self
                .state
                .placeholder
                .as_deref()
                .filter(|hint| !hint.is_empty())
        {
            descriptor.with_visual_placeholder(hint)
        } else {
            descriptor
        };
        WidgetMeasure::Text(descriptor)
    }

    fn hit_test(&self, _: &Self::State, context: HitContributionContext) -> HitContribution {
        let size = context.local_size();
        let rect = crate::LogicalRect::try_new(0.0, 0.0, size.width(), size.height())
            .unwrap_or_else(|_| unreachable!("validated local size yields a finite rectangle"));
        HitContribution::single_rect(rect)
    }

    fn semantics(&self, state: &Self::State, context: SemanticContributionContext) -> SemanticContribution {
        let Some(editable) = SemanticEditable::new(
            self.state.snapshot,
            &self.state.content,
            self.state.selection,
            self.state.sensitivity,
            self.state.read_only,
        ) else {
            return SemanticContribution::empty();
        };
        let mut state = SemanticState::ENABLED
            .with_disabled(self.state.disabled)
            .with_read_only(self.state.read_only)
            .with_required(self.state.required);
        if let Some(invalid) = self.state.invalid {
            state = state.with_invalid(invalid);
        }
        let role = if state.combo.is_some() {
            SemanticRole::ComboBox
        } else if self.state.spin.is_some() {
            SemanticRole::SpinButton
        } else {
            SemanticRole::EditableText
        };
        let mut node = SemanticNodeContribution::primary(role)
            .with_state(state)
            .with_editable(editable)
            .with_editable_mode(self.state.mode)
            .with_action(SemanticAction::MoveBackward)
            .with_action(SemanticAction::MoveForward)
            .with_action(SemanticAction::ExtendBackward)
            .with_action(SemanticAction::ExtendForward)
            .with_action(SemanticAction::SelectAll)
            .with_action(SemanticAction::SetSelection);
        if !self.state.read_only {
            node = node
                .with_action(SemanticAction::DeleteBackward)
                .with_action(SemanticAction::DeleteForward)
                .with_action(SemanticAction::Undo)
                .with_action(SemanticAction::Redo)
                .with_action(SemanticAction::ReplaceSelection);
        }
        if let Some(placeholder) = &self.state.placeholder {
            node = node.with_placeholder(placeholder.clone());
        }
        if let Some(combo) = state.combo.as_ref() {
            if combo.invalid.is_some() {
                return SemanticContribution::empty();
            }
            node = node
                .with_name(combo.label.clone())
                .with_popup(SemanticPopupKind::ListBox)
                .with_autocomplete(state.combo_autocomplete.unwrap_or(SemanticAutocomplete::List))
                .with_state(SemanticState::ENABLED
                    .with_disabled(state.disabled)
                    .with_read_only(state.read_only)
                    .with_required(state.required)
                    .with_expanded(combo.open))
                .with_action(SemanticAction::RequestFocus);
            if combo.enabled {
                node = node.with_action(SemanticAction::Activate)
                    .with_action(if combo.open {
                        SemanticAction::Collapse
                    } else {
                        SemanticAction::Expand
                    });
            }
            if combo.open {
                node = node.with_relationship(SemanticRelationship::new(
                    SemanticRelationshipKind::Controls,
                    SemanticReference::Authored {
                        element_id: combo.popup_id.clone(),
                        semantic_key: None,
                    },
                ));
                if let Some(id) = combo.active.as_ref() {
                    node = node.with_relationship(SemanticRelationship::new(
                        SemanticRelationshipKind::ActiveDescendant,
                        SemanticReference::Authored {
                            element_id: id.clone(),
                            semantic_key: None,
                        },
                    ));
                }
            }
            if context.has_mounted_children() {
                node = node.with_mounted_children();
            }
        }
        if let Some(spin) = &self.state.spin {
            node = node
                .with_name(spin.label.clone())
                .with_range(spin.range.clone());
            if spin.actionable && !self.state.disabled && !self.state.read_only {
                if crate::spin_button::step_actions_available(&spin.range) {
                    node = node
                        .with_action(SemanticAction::Increment)
                        .with_action(SemanticAction::Decrement);
                }
                node = node.with_action(SemanticAction::SetValue);
            }
        }
        for (target, kind) in [
            (
                &self.state.labelled_by,
                SemanticRelationshipKind::LabelledBy,
            ),
            (
                &self.state.described_by,
                SemanticRelationshipKind::DescribedBy,
            ),
            (
                &self.state.error_message,
                SemanticRelationshipKind::ErrorMessage,
            ),
        ] {
            if let Some(element_id) = target {
                node = node.with_relationship(SemanticRelationship::new(
                    kind,
                    SemanticReference::Authored {
                        element_id: element_id.clone(),
                        semantic_key: None,
                    },
                ));
            }
        }
        SemanticContribution::single(node)
    }
}

impl<Action> ChildBearingWidget<Action> for TextFieldWidget<Action> {}

impl<Action: 'static> View<Action> for TextField<Action> {
    fn into_element(self) -> Element<Action> {
        let invalid_combo = self.combo.as_ref().is_some_and(|combo| combo.invalid.is_some());
        let combo_state = self.combo.as_ref().map(Widget::create_state);
        let (fields, diagnostics) = self.common.into_authored_fields(
            if self.disabled || invalid_combo {
                Focusability::NotFocusable
            } else {
                Focusability::Focusable
            },
            None,
        );
        let spin_state = self.spin.as_ref().map(|spin| SpinFacts {
            label: spin.label.clone(),
            range: spin.range.clone(),
            actionable: spin.on_adjust.is_some(),
        });
        let on_adjust = self.spin.and_then(|spin| spin.on_adjust);
        Element::from_authored_parts(
            fields,
            Box::new(WidgetAdapter(TextFieldWidget {
                state: TextFieldState {
                    snapshot: self.snapshot,
                    content: self.content,
                    selection: self.selection,
                    mode: self.mode,
                    sensitivity: self.sensitivity,
                    placeholder: self.placeholder,
                    labelled_by: self.labelled_by,
                    described_by: self.described_by,
                    error_message: self.error_message,
                    invalid: self.invalid,
                    required: self.required,
                    read_only: self.read_only,
                    disabled: self.disabled,
                    spin: spin_state,
                    combo: combo_state,
                    combo_autocomplete: self.combo_autocomplete,
                },
                mapper: self.mapper,
                on_submit: self.on_submit,
                on_adjust,
                combo: self.combo,
            })),
            self.combo_popup.into_iter().collect(),
            diagnostics,
        )
    }
}

/// Constructs the public single-/multiline standard field over one M10 document.
///
/// # Errors
///
/// Returns an error for invalid revision-scoped coordinates or an already
/// multiline source bound to the single-line policy.
pub fn text_field<Action>(
    snapshot: TextDocumentSnapshot,
    content: impl Into<String>,
    selection: TextSelection,
    mode: SemanticEditableMode,
    on_edit: impl Fn(EditIntent) -> Action + 'static,
) -> Result<TextField<Action>, TextFieldError> {
    TextField::new(snapshot, content, selection, mode, on_edit)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{TextAffinity, TextDocumentId, TextDocumentRevision, TextPosition};

    fn widget(mode: SemanticEditableMode) -> TextFieldWidget<()> {
        let content = "a line without hard breaks".to_owned();
        let snapshot =
            TextDocumentSnapshot::new(TextDocumentId::new(91), TextDocumentRevision::new(1));
        let position = TextPosition::new(snapshot, &content, content.len(), TextAffinity::Upstream)
            .unwrap_or_else(|_| unreachable!("fixture position is valid"));
        TextFieldWidget {
            state: TextFieldState {
                snapshot,
                content,
                selection: TextSelection::collapsed(position),
                mode,
                sensitivity: TextSensitivity::Public,
                placeholder: None,
                labelled_by: None,
                described_by: None,
                error_message: None,
                invalid: None,
                required: false,
                read_only: false,
                disabled: false,
                spin: None,
                combo: None,
                combo_autocomplete: None,
            },
            mapper: Rc::new(|_| ()),
            on_submit: None,
            on_adjust: None,
            combo: None,
        }
    }

    #[test]
    fn secret_password_builder_rejects_multiline_and_redacts_widget_state_debug() {
        let source = "vault-secret-é漢";
        let snapshot =
            TextDocumentSnapshot::new(TextDocumentId::new(92), TextDocumentRevision::new(1));
        let active = TextPosition::new(snapshot, source, "vault".len(), TextAffinity::Downstream)
            .unwrap_or_else(|_| unreachable!("source selection is valid"));
        let make = |mode| {
            TextField::<()>::new(
                snapshot,
                source,
                TextSelection::collapsed(active),
                mode,
                |_| (),
            )
            .unwrap_or_else(|_| unreachable!("checked constructor succeeds"))
        };
        assert!(matches!(
            make(SemanticEditableMode::Multiline).password(),
            Err(TextFieldError::SecretRequiresSingleLine)
        ));
        let field = make(SemanticEditableMode::SingleLine)
            .password()
            .unwrap_or_else(|_| unreachable!("single-line password is valid"));
        assert_eq!(field.sensitivity, TextSensitivity::Secret);
        let facade_debug = format!("{field:?}");
        for forbidden in [source, "content_bytes", "selection"] {
            assert!(!facade_debug.contains(forbidden));
        }
        let mut widget = widget(SemanticEditableMode::SingleLine);
        widget.state.content = source.to_owned();
        widget.state.selection = TextSelection::collapsed(
            TextPosition::new(
                widget.state.snapshot,
                source,
                "vault".len(),
                TextAffinity::Downstream,
            )
            .unwrap_or_else(|_| unreachable!("updated selection validates")),
        );
        widget.state.sensitivity = TextSensitivity::Secret;
        for debug in [format!("{:?}", widget.state), format!("{widget:?}")] {
            for forbidden in [source, "content_bytes", "selection"] {
                assert!(
                    !debug.contains(forbidden),
                    "secret metadata appeared in Debug"
                );
            }
        }
        assert_eq!(
            widget
                .editable(&widget.state)
                .unwrap_or_else(|| unreachable!("secret document binds"))
                .sensitivity(),
            TextSensitivity::Secret
        );
        let public = make(SemanticEditableMode::SingleLine);
        assert!(format!("{public:?}").contains("content_bytes"));
    }

    #[test]
    fn secret_classification_rebuild_invalidates_retained_layout_and_semantics() {
        let before = widget(SemanticEditableMode::SingleLine);
        let mut after = widget(SemanticEditableMode::SingleLine);
        after.state.sensitivity = TextSensitivity::Secret;
        let mut retained = before.create_state();
        let mut context = WidgetUpdateContext::<()>::__runtime_new();
        after.update(&mut retained, &mut context);
        let flags = context.__runtime_take_invalidation();
        assert!(flags.contains(WidgetInvalidation::LAYOUT));
        assert!(flags.contains(WidgetInvalidation::PAINT));
        assert!(flags.contains(WidgetInvalidation::SEMANTICS));
        assert_eq!(retained.sensitivity, TextSensitivity::Secret);
    }

    #[test]
    fn mode_rebuild_invalidates_retained_text_geometry_and_semantics() {
        let before = widget(SemanticEditableMode::Multiline);
        let after = widget(SemanticEditableMode::SingleLine);
        let mut retained = before.create_state();
        let mut context = WidgetUpdateContext::<()>::__runtime_new();
        after.update(&mut retained, &mut context);
        let flags = context.__runtime_take_invalidation();
        assert!(flags.contains(WidgetInvalidation::LAYOUT));
        assert!(flags.contains(WidgetInvalidation::PAINT));
        assert!(flags.contains(WidgetInvalidation::SEMANTICS));
        assert_eq!(retained.mode, SemanticEditableMode::SingleLine);
    }

    #[test]
    fn relationship_rebuild_invalidates_semantics_without_layout_or_paint() {
        let before = widget(SemanticEditableMode::SingleLine);
        let mut after = widget(SemanticEditableMode::SingleLine);
        after.state.labelled_by = Some(
            ElementId::from_static("form.label")
                .unwrap_or_else(|_| unreachable!("fixture id is valid")),
        );
        after.state.described_by = Some(
            ElementId::from_static("form.help")
                .unwrap_or_else(|_| unreachable!("fixture id is valid")),
        );
        after.state.error_message = Some(
            ElementId::from_static("form.error")
                .unwrap_or_else(|_| unreachable!("fixture id is valid")),
        );
        let mut retained = before.create_state();
        let mut context = WidgetUpdateContext::<()>::__runtime_new();
        after.update(&mut retained, &mut context);
        let flags = context.__runtime_take_invalidation();
        assert!(flags.contains(WidgetInvalidation::SEMANTICS));
        assert!(!flags.contains(WidgetInvalidation::LAYOUT));
        assert!(!flags.contains(WidgetInvalidation::PAINT));
        assert_eq!(retained.labelled_by, after.state.labelled_by);
        assert_eq!(retained.described_by, after.state.described_by);
        assert_eq!(retained.error_message, after.state.error_message);
    }

    #[test]
    fn readonly_and_disabled_changes_refresh_interaction_authority() {
        let before = widget(SemanticEditableMode::SingleLine);
        let mut after = widget(SemanticEditableMode::SingleLine);
        after.state.read_only = true;
        after.state.disabled = true;
        let mut retained = before.create_state();
        let mut context = WidgetUpdateContext::<()>::__runtime_new();
        after.update(&mut retained, &mut context);
        let flags = context.__runtime_take_invalidation();
        assert!(flags.contains(WidgetInvalidation::INTERACTION));
        assert!(flags.contains(WidgetInvalidation::SEMANTICS));
        assert!(!flags.contains(WidgetInvalidation::LAYOUT));
    }
    #[derive(Clone, Copy, Debug)]
    enum EnterCase {
        Ordinary,
        Shift,
        Control,
        Repeat,
        Composition,
        PreviouslyPrevented,
        ReadOnly,
        Disabled,
    }

    #[test]
    fn enter_honors_prevention_modifiers_repeat_composition_and_read_only() {
        for case in [
            EnterCase::Ordinary,
            EnterCase::Shift,
            EnterCase::Control,
            EnterCase::Repeat,
            EnterCase::Composition,
            EnterCase::PreviouslyPrevented,
            EnterCase::ReadOnly,
            EnterCase::Disabled,
        ] {
            assert_enter_case(case);
        }
    }

    fn assert_enter_case(case: EnterCase) {
        use crate::{
            __runtime::{RoutedEventOutput, RuntimeNamespace},
            CommandOrigin, KeyLocation, KeyModifiers, KeyboardEvent, MonotonicInstant, PhysicalKey,
            WorkSequence,
        };
        use core::num::NonZeroU64;

        let namespace = RuntimeNamespace::__runtime_new();
        let owner = namespace.__runtime_mounted_id(1, 1);
        let mut field = widget(SemanticEditableMode::SingleLine);
        field.state.read_only = matches!(case, EnterCase::ReadOnly);
        field.state.disabled = matches!(case, EnterCase::Disabled);
        field.on_submit = Some(Box::new(|| ()));
        let mut state = field.create_state();

        let modifiers = match case {
            EnterCase::Shift => KeyModifiers::SHIFT,
            EnterCase::Control => KeyModifiers::NONE.with_control(),
            _ => KeyModifiers::NONE,
        };
        let composition = if matches!(case, EnterCase::Composition) {
            KeyboardCompositionState::Active
        } else {
            KeyboardCompositionState::Inactive
        };
        let event = UiEvent::Keyboard(KeyboardEvent::new(
            KeyboardPhase::Down,
            PhysicalKey::Code("Enter".to_owned()),
            LogicalKey::Enter,
            modifiers,
            matches!(case, EnterCase::Repeat),
            KeyLocation::Standard,
            composition,
            None,
        ));
        let mut context = EventContext::new(
            EventPhase::Target,
            &owner,
            &owner,
            None,
            CommandOrigin::programmatic(),
            WorkSequence::__runtime_new(
                NonZeroU64::new(1).unwrap_or_else(|| unreachable!("valid work sequence")),
            ),
            MonotonicInstant::__runtime_from_nanos(0),
            None,
            true,
            matches!(case, EnterCase::PreviouslyPrevented),
            false,
            4,
        );
        let _ = field.event(&mut state, &event, &mut context);
        let output = context.into_output();
        let ignored = matches!(
            case,
            EnterCase::Shift
                | EnterCase::Control
                | EnterCase::Composition
                | EnterCase::PreviouslyPrevented
                | EnterCase::Disabled
        );
        let emits = matches!(case, EnterCase::Ordinary | EnterCase::ReadOnly);
        assert_eq!(
            output.default_prevented,
            !ignored || matches!(case, EnterCase::PreviouslyPrevented),
            "{case:?}"
        );
        assert_eq!(output.ordered.len(), usize::from(emits), "{case:?}");
        if emits {
            assert!(matches!(output.ordered[0], RoutedEventOutput::Action(())));
        }
    }

    #[test]
    fn combined_mode_and_disabled_rebuild_invalidates_both_layout_and_interaction() {
        let before = widget(SemanticEditableMode::Multiline);
        let mut after = widget(SemanticEditableMode::SingleLine);
        after.state.disabled = true;
        let mut retained = before.create_state();
        let mut context = WidgetUpdateContext::<()>::__runtime_new();
        after.update(&mut retained, &mut context);
        let flags = context.__runtime_take_invalidation();
        assert!(flags.contains(WidgetInvalidation::LAYOUT));
        assert!(flags.contains(WidgetInvalidation::INTERACTION));
        assert!(flags.contains(WidgetInvalidation::PAINT));
        assert!(flags.contains(WidgetInvalidation::SEMANTICS));
    }
}
