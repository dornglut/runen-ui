use core::fmt;
use std::rc::Rc;

use crate::{
    EditIntent, EditableContribution, EditableContributionError, EditingSessionPolicy, ElementId,
    EventContext, EventPhase, Focusability, HitContribution, HitContributionContext, IntoElementId,
    KeyboardCompositionState, KeyboardPhase, LogicalKey, SemanticAction, SemanticContribution,
    SemanticContributionContext, SemanticEditable, SemanticEditableMode, SemanticInvalidState,
    SemanticNodeContribution, SemanticReference, SemanticRelationship, SemanticRelationshipKind,
    SemanticRole, SemanticState, TextDocumentSnapshot, TextNewlinePolicy, TextSelection,
    TextSensitivity, UiEvent, WidgetInvalidation, WidgetMeasure, WidgetMeasureInput,
    WidgetTextInput, WidgetUpdateContext,
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
}

impl fmt::Display for TextFieldError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidSelection(error) => write!(f, "{error}"),
            Self::SingleLineSourceContainsNewline => {
                f.write_str("single-line text field source contains a line break")
            }
        }
    }
}
impl std::error::Error for TextFieldError {}

/// Host-neutral public text-input facade. M10 remains the only editing authority.
///
/// The currently implemented entry modes are public single-line and multiline.
/// A secret/password presentation must not be exposed without correlated masked
/// text shaping and caret mapping.
pub struct TextField<Action> {
    snapshot: TextDocumentSnapshot,
    content: String,
    selection: TextSelection,
    mode: SemanticEditableMode,
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
    common: CommonNodeAuthoring,
}

impl<Action> fmt::Debug for TextField<Action> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("TextField")
            .field("snapshot", &self.snapshot)
            .field("content_bytes", &self.content.len())
            .field("selection", &self.selection)
            .field("mode", &self.mode)
            .field("read_only", &self.read_only)
            .field("disabled", &self.disabled)
            .finish_non_exhaustive()
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
            && (content.contains('\r') || content.contains('\n'))
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
            common: CommonNodeAuthoring::default(),
        })
    }

    common_node_builder_methods!();

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

#[derive(Clone, Debug, PartialEq)]
struct TextFieldState {
    snapshot: TextDocumentSnapshot,
    content: String,
    selection: TextSelection,
    mode: SemanticEditableMode,
    placeholder: Option<String>,
    invalid: Option<SemanticInvalidState>,
    required: bool,
    read_only: bool,
    disabled: bool,
}

struct TextFieldWidget<Action> {
    state: TextFieldState,
    labelled_by: Option<ElementId>,
    described_by: Option<ElementId>,
    error_message: Option<ElementId>,
    mapper: Rc<dyn Fn(EditIntent) -> Action>,
    on_submit: Option<Box<dyn FnMut() -> Action>>,
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
        if state.content != self.state.content {
            context.invalidate(
                WidgetInvalidation::LAYOUT
                    | WidgetInvalidation::PAINT
                    | WidgetInvalidation::SEMANTICS,
            );
        } else if state != &self.state {
            context.invalidate(WidgetInvalidation::SEMANTICS | WidgetInvalidation::PAINT);
        }
        state.clone_from(&self.state);
    }

    fn activation(&self, _: &Self::State) -> WidgetActivation {
        if self.state.disabled {
            WidgetActivation::disabled()
        } else {
            WidgetActivation::NONE
        }
    }

    fn text_input(&self, _: &Self::State) -> WidgetTextInput {
        let enabled = !self.state.disabled && !self.state.read_only;
        WidgetTextInput::new(enabled, enabled)
    }

    fn event(
        &mut self,
        _: &mut Self::State,
        event: &UiEvent,
        context: &mut EventContext<'_, Action>,
    ) -> crate::WidgetEventOutput {
        if context.phase() == EventPhase::Target
            && self.state.mode == SemanticEditableMode::SingleLine
            && !self.state.disabled
            && let Some(key) = event.as_keyboard()
            && key.phase() == KeyboardPhase::Down
            && key.logical_key() == &LogicalKey::Enter
            && key.composition_state() == KeyboardCompositionState::Inactive
        {
            context.prevent_default();
            if !key.is_repeat()
                && let Some(submit) = self.on_submit.as_mut()
            {
                context.emit(submit());
            }
        }
        crate::WidgetEventOutput::none()
    }

    fn editable(&self, _: &Self::State) -> Option<EditableContribution<Action>> {
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
            TextSensitivity::Public,
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
        WidgetMeasure::Text(
            crate::TextLeafMeasure::new(self.state.content.clone()).with_wrap_mode(wrap),
        )
    }

    fn hit_test(&self, _: &Self::State, context: HitContributionContext) -> HitContribution {
        let size = context.local_size();
        let rect = crate::LogicalRect::try_new(0.0, 0.0, size.width(), size.height())
            .unwrap_or_else(|_| unreachable!("validated local size yields a finite rectangle"));
        HitContribution::single_rect(rect)
    }

    fn semantics(&self, _: &Self::State, _: SemanticContributionContext) -> SemanticContribution {
        let Some(editable) = SemanticEditable::new(
            self.state.snapshot,
            &self.state.content,
            self.state.selection,
            TextSensitivity::Public,
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
        let mut node = SemanticNodeContribution::primary(SemanticRole::EditableText)
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
        for (target, kind) in [
            (&self.labelled_by, SemanticRelationshipKind::LabelledBy),
            (&self.described_by, SemanticRelationshipKind::DescribedBy),
            (&self.error_message, SemanticRelationshipKind::ErrorMessage),
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

impl<Action: 'static> View<Action> for TextField<Action> {
    fn into_element(self) -> Element<Action> {
        let (fields, diagnostics) = self
            .common
            .into_authored_fields(Focusability::Focusable, None);
        Element::from_authored_parts(
            fields,
            Box::new(WidgetAdapter(TextFieldWidget {
                state: TextFieldState {
                    snapshot: self.snapshot,
                    content: self.content,
                    selection: self.selection,
                    mode: self.mode,
                    placeholder: self.placeholder,
                    invalid: self.invalid,
                    required: self.required,
                    read_only: self.read_only,
                    disabled: self.disabled,
                },
                labelled_by: self.labelled_by,
                described_by: self.described_by,
                error_message: self.error_message,
                mapper: self.mapper,
                on_submit: self.on_submit,
            })),
            Vec::new(),
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
