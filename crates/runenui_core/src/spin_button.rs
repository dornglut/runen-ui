//! Application-owned numeric editing through the accepted single M10 `TextField`.
//! This file supplies checked range authoring and step intents, not an editor,
//! parser, formatter, value cache, or a second semantic-command queue.

use core::fmt;

use crate::{
    EditIntent, IntoElementId, KeyboardCompositionState, KeyboardEvent, KeyboardPhase, LogicalKey,
    SemanticCommand, SemanticInvalidState, SemanticNumber, SemanticNumberError, SemanticRange,
    SemanticRangeError, TextDocumentSnapshot, TextField, TextFieldError, TextSelection,
    element::{CommonNodeAuthoring, Element, View, common_node_builder_methods},
};

/// A checked signed relative step or exact absolute numeric request.
///
/// The application applies each request to its latest state, in ordinary FIFO
/// order; the published range is never used as a hidden accumulating cursor.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SpinButtonRequest {
    AdjustBy(SemanticNumber),
    SetValue(SemanticNumber),
}

pub fn semantic_request(
    range: &SemanticRange,
    command: SemanticCommand,
) -> Option<SpinButtonRequest> {
    let step_ready = range.current().is_some() && range.minimum() != range.maximum();
    match command {
        SemanticCommand::Increment if step_ready => {
            range.small_step().map(SpinButtonRequest::AdjustBy)
        }
        SemanticCommand::Decrement if step_ready => range
            .small_step()
            .and_then(|step| SemanticNumber::new(-step.get()).ok())
            .map(SpinButtonRequest::AdjustBy),
        SemanticCommand::SetValue(value)
            if range
                .minimum()
                .is_none_or(|minimum| value.get() >= minimum.get())
                && range
                    .maximum()
                    .is_none_or(|maximum| value.get() <= maximum.get()) =>
        {
            Some(SpinButtonRequest::SetValue(value))
        }
        _ => None,
    }
}

pub fn keyboard_request(
    range: &SemanticRange,
    key: &KeyboardEvent,
) -> Option<SpinButtonRequest> {
    if key.phase() != KeyboardPhase::Down
        || key.modifiers() != crate::KeyModifiers::NONE
        || key.composition_state() != KeyboardCompositionState::Inactive
    {
        return None;
    }
    match key.logical_key() {
        LogicalKey::ArrowUp => semantic_request(range, SemanticCommand::Increment),
        LogicalKey::ArrowDown => semantic_request(range, SemanticCommand::Decrement),
        _ => None,
    }
}

/// Checked authoring rejection, with no source text in diagnostics.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SpinButtonError {
    Text(TextFieldError),
    NonFinite,
    Range(SemanticRangeError),
}

impl fmt::Display for SpinButtonError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Text(error) => error.fmt(f),
            Self::NonFinite => f.write_str("numeric input bounds, value and step must be finite"),
            Self::Range(error) => error.fmt(f),
        }
    }
}
impl std::error::Error for SpinButtonError {}
impl From<TextFieldError> for SpinButtonError {
    fn from(error: TextFieldError) -> Self {
        Self::Text(error)
    }
}
impl From<SemanticNumberError> for SpinButtonError {
    fn from(_: SemanticNumberError) -> Self {
        Self::NonFinite
    }
}
impl From<SemanticRangeError> for SpinButtonError {
    fn from(error: SemanticRangeError) -> Self {
        Self::Range(error)
    }
}

/// Convenience checked constructor for the existing #310 semantic range.
/// It never parses the source text or stores any durable numeric state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SpinButtonRange(SemanticRange);

impl SpinButtonRange {
    /// A temporarily invalid or incomplete text source uses `current=None`.
    /// Optional bounds permit open-ended `SpinButton` inputs, unlike bounded `Slider` controls.
    /// Both ordinary f64 bounds and explicit `Option<f64>` bounds are accepted.
    /// Step intents remain relative; application policy owns finite arithmetic
    /// and clamping against its latest state.
    ///
    /// # Errors
    /// Rejects non-finite numbers, reversed bounds, out-of-range current
    /// or non-positive steps before mounting.
    pub fn new(
        minimum: impl Into<Option<f64>>,
        maximum: impl Into<Option<f64>>,
        current: Option<f64>,
        step: f64,
    ) -> Result<Self, SpinButtonError> {
        let minimum = minimum.into().map(SemanticNumber::new).transpose()?;
        let maximum = maximum.into().map(SemanticNumber::new).transpose()?;
        let current = current.map(SemanticNumber::new).transpose()?;
        let step = SemanticNumber::new(step)?;
        let range = SemanticRange::new(minimum, maximum, current)?.with_small_step(step)?;
        Ok(Self(range))
    }

    /// Application-authored spoken value text is legal only with a current
    /// numeric value. No fake value-text for incomplete input.
    /// # Errors
    /// Rejects value text when the current numeric value is absent.
    pub fn value_text(mut self, value: impl Into<String>) -> Result<Self, SpinButtonError> {
        self.0 = self.0.with_value_text(value)?;
        Ok(self)
    }

    #[must_use]
    pub const fn semantic_range(&self) -> &SemanticRange {
        &self.0
    }
}

/// One standard single-line `TextField` whose primary role is `SpinButton`.
///
/// All document, selection, caret, composition and edit transactions use the
/// accepted `TextField`/M10 widget unchanged. Only range semantics and ordinary
/// application step-intent emission are added.
pub struct SpinButton<Action> {
    field: TextField<Action>,
    label: String,
    range: SemanticRange,
    on_adjust: Option<Box<dyn FnMut(SpinButtonRequest) -> Action>>,
    common: CommonNodeAuthoring,
}

impl<Action> fmt::Debug for SpinButton<Action> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SpinButton")
            .field("field", &self.field)
            .field("label", &self.label)
            .field("range", &self.range)
            .finish_non_exhaustive()
    }
}

impl<Action> SpinButton<Action> {
    /// Constructs one checked numeric editor. The application independently
    /// supplies the exact text and the optional parsed/validated numeric value.
    ///
    /// # Errors
    /// Rejects a foreign/invalid M10 selection or a pre-existing hard newline.
    pub fn new(
        label: impl Into<String>,
        snapshot: TextDocumentSnapshot,
        content: impl Into<String>,
        selection: TextSelection,
        range: SpinButtonRange,
        on_edit: impl Fn(EditIntent) -> Action + 'static,
    ) -> Result<Self, SpinButtonError> {
        let field = TextField::new(
            snapshot,
            content,
            selection,
            crate::SemanticEditableMode::SingleLine,
            on_edit,
        )?;
        Ok(Self {
            field,
            label: label.into(),
            range: range.0,
            on_adjust: None,
            common: CommonNodeAuthoring::default(),
        })
    }

    common_node_builder_methods!();

    /// Emits relative/absolute intents; each app update commits or clamps
    /// against its latest numeric state and independently reauthors text.
    #[must_use]
    pub fn on_adjust(
        mut self,
        callback: impl FnMut(SpinButtonRequest) -> Action + 'static,
    ) -> Self {
        self.on_adjust = Some(Box::new(callback));
        self
    }

    #[must_use]
    pub fn disabled(mut self, disabled: bool) -> Self {
        self.field = self.field.disabled(disabled);
        self
    }
    #[must_use]
    pub fn read_only(mut self, read_only: bool) -> Self {
        self.field = self.field.read_only(read_only);
        self
    }
    #[must_use]
    pub fn required(mut self, required: bool) -> Self {
        self.field = self.field.required(required);
        self
    }
    #[must_use]
    pub fn invalid(mut self, invalid: SemanticInvalidState) -> Self {
        self.field = self.field.invalid(invalid);
        self
    }
    #[must_use]
    pub fn placeholder(mut self, placeholder: impl Into<String>) -> Self {
        self.field = self.field.placeholder(placeholder);
        self
    }
    #[must_use]
    pub fn labelled_by(mut self, id: impl IntoElementId) -> Self {
        self.field = self.field.labelled_by(id);
        self
    }
    #[must_use]
    pub fn described_by(mut self, id: impl IntoElementId) -> Self {
        self.field = self.field.described_by(id);
        self
    }
    #[must_use]
    pub fn error_message(mut self, id: impl IntoElementId) -> Self {
        self.field = self.field.error_message(id);
        self
    }
    #[must_use]
    pub fn on_submit(mut self, callback: impl FnMut() -> Action + 'static) -> Self {
        self.field = self.field.on_submit(callback);
        self
    }
}

impl<Action: 'static> View<Action> for SpinButton<Action> {
    fn into_element(self) -> Element<Action> {
        self.field
            .with_spin_button(self.label, self.range, self.on_adjust)
            .with_common_node(self.common)
            .into_element()
    }
}

/// Creates a `SpinButton` as a single `TextField` editing owner.
///
/// # Errors
/// Rejects invalid source document, selection or newline policy.
pub fn spin_button<Action>(
    label: impl Into<String>,
    snapshot: TextDocumentSnapshot,
    content: impl Into<String>,
    selection: TextSelection,
    range: SpinButtonRange,
    on_edit: impl Fn(EditIntent) -> Action + 'static,
) -> Result<SpinButton<Action>, SpinButtonError> {
    SpinButton::new(label, snapshot, content, selection, range, on_edit)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checked_range_preserves_absent_value_and_rejects_bad_arithmetic() {
        let absent = SpinButtonRange::new(-100.0, 100.0, None, 0.5)
            .unwrap_or_else(|_| unreachable!("valid checked range"));
        assert_eq!(absent.semantic_range().current(), None);
        assert!(matches!(
            absent.value_text("unknown"),
            Err(SpinButtonError::Range(
                SemanticRangeError::ValueTextWithoutCurrent
            ))
        ));
        assert!(SpinButtonRange::new(0.0, 10.0, Some(12.0), 1.0).is_err());
        assert!(SpinButtonRange::new(10.0, 0.0, None, 1.0).is_err());
        assert!(SpinButtonRange::new(0.0, 10.0, None, 0.0).is_err());
        assert!(SpinButtonRange::new(0.0, 10.0, None, -1.0).is_err());
        assert!(SpinButtonRange::new(f64::NEG_INFINITY, 10.0, None, 1.0).is_err());
        assert!(SpinButtonRange::new(-f64::MAX, f64::MAX, None, f64::MIN_POSITIVE).is_ok());
        assert!(SpinButtonRange::new(0.0, 10.0, Some(f64::NAN), 1.0).is_err());
    }

    #[test]
    fn optional_bounds_preserve_exact_set_value_and_relative_step_intents() {
        let unbounded = SpinButtonRange::new(None, None, None, 0.25)
            .unwrap_or_else(|_| unreachable!("checked unbounded range"));
        assert_eq!(unbounded.semantic_range().minimum(), None);
        assert_eq!(unbounded.semantic_range().maximum(), None);
        assert_eq!(unbounded.semantic_range().current(), None);
        assert_eq!(
            semantic_request(
                unbounded.semantic_range(),
                SemanticCommand::SetValue(
                    SemanticNumber::new(-1000.0).unwrap_or_else(|_| unreachable!("finite value"))
                )
            ),
            Some(SpinButtonRequest::SetValue(
                SemanticNumber::new(-1000.0).unwrap_or_else(|_| unreachable!("finite value"))
            ))
        );
        assert_eq!(
            semantic_request(unbounded.semantic_range(), SemanticCommand::Increment),
            None,
            "an absent parsed value never becomes a fabricated stepping baseline"
        );

        let upper_only = SpinButtonRange::new(None, Some(5.0), Some(-2.0), 0.5)
            .unwrap_or_else(|_| unreachable!("valid upper bound"));
        assert_eq!(
            semantic_request(
                upper_only.semantic_range(),
                SemanticCommand::SetValue(
                    SemanticNumber::new(-9.0).unwrap_or_else(|_| unreachable!("finite value"))
                )
            ),
            Some(SpinButtonRequest::SetValue(
                SemanticNumber::new(-9.0).unwrap_or_else(|_| unreachable!("finite value"))
            ))
        );
        assert_eq!(
            semantic_request(
                upper_only.semantic_range(),
                SemanticCommand::SetValue(
                    SemanticNumber::new(6.0).unwrap_or_else(|_| unreachable!("finite value"))
                )
            ),
            None
        );
        let lower_only = SpinButtonRange::new(Some(-5.0), None, Some(0.0), 0.5)
            .unwrap_or_else(|_| unreachable!("valid lower bound"));
        assert_eq!(
            semantic_request(
                lower_only.semantic_range(),
                SemanticCommand::SetValue(
                    SemanticNumber::new(-6.0).unwrap_or_else(|_| unreachable!("finite value"))
                )
            ),
            None
        );
        assert_eq!(
            semantic_request(lower_only.semantic_range(), SemanticCommand::Increment),
            Some(SpinButtonRequest::AdjustBy(
                SemanticNumber::new(0.5).unwrap_or_else(|_| unreachable!("positive step"))
            ))
        );
    }
}
