//! Application-controlled divider resizing over ordinary pointer capture and semantic routing.
//!
//! The widget never owns pane dimensions or maps its thin hit target to a pane-size
//! range. It emits relative logical movement or typed bounded-value intents; the
//! application maps those intents to adjacent-pane layout and reauthors the value.

use core::fmt;

use crate::{
    Brush, Color, EventContext, EventPhase, Focusability, HitContribution, HitContributionContext,
    KeyModifiers, KeyboardPhase, LogicalKey, LogicalLength, LogicalRect, PaintContribution,
    PaintContributionContext, PaintContributionItem, PointerButton, PointerCaptureKind,
    PointerDeviceKind, PointerId, PointerPhase, SceneShape, SemanticAction, SemanticCommand,
    SemanticContribution, SemanticContributionContext, SemanticNodeContribution, SemanticNumber,
    SemanticNumberError, SemanticOrientation, SemanticRange, SemanticRangeError, SemanticRole,
    SemanticState, UiEvent, WidgetActivation, WidgetEventOutput, WidgetInvalidation, WidgetMeasure,
    WidgetMeasureInput, WidgetUpdateContext,
    element::{CommonNodeAuthoring, Element, View, common_node_builder_methods},
    widget_erasure::WidgetAdapter,
    widget_protocol::Widget,
};

/// One application-owned pane-size request; no request commits a new size on its own.
///
/// `MoveBy` is signed surface-logical pixels along the divider's movement axis:
/// positive moves right for a vertical divider or down for a horizontal one.
/// The application owns the pixels-to-value mapping, constraints and rounding.
/// AdjustBy carries the authored signed small/page step rather than a value based
/// on a possibly stale render. Consecutive queued actions apply the step to
/// current application state, not the last published `SemanticRange::current`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SplitterRequest {
    MoveBy(f32),
    AdjustBy(SemanticNumber),
    SetValue(SemanticNumber),
}

/// Invalid checked Splitter construction.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SplitterError {
    NonFinite,
    Range(SemanticRangeError),
    UnrepresentableSpan,
}

impl fmt::Display for SplitterError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFinite => f.write_str("splitter values and steps must be finite"),
            Self::Range(error) => error.fmt(f),
            Self::UnrepresentableSpan => {
                f.write_str("splitter span or step count cannot be represented")
            }
        }
    }
}

impl std::error::Error for SplitterError {}

impl From<SemanticNumberError> for SplitterError {
    fn from(_: SemanticNumberError) -> Self {
        Self::NonFinite
    }
}

impl From<SemanticRangeError> for SplitterError {
    fn from(error: SemanticRangeError) -> Self {
        Self::Range(error)
    }
}

/// Resizable divider between two application-owned panes.
///
/// `orientation` describes the divider line, not the direction of movement:
/// vertical dividers move left/right; horizontal dividers move up/down.
/// The common node's authored hit extent is independent of the painted grip.
pub struct Splitter<Action> {
    label: String,
    range: SemanticRange,
    orientation: SemanticOrientation,
    enabled: bool,
    common: CommonNodeAuthoring,
    on_resize: Option<Box<dyn FnMut(SplitterRequest) -> Action>>,
}

impl<Action> fmt::Debug for Splitter<Action> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Splitter")
            .field("label", &self.label)
            .field("range", &self.range)
            .field("orientation", &self.orientation)
            .field("enabled", &self.enabled)
            .field("actionable", &self.on_resize.is_some())
            .finish_non_exhaustive()
    }
}

impl<Action> Splitter<Action> {
    /// Creates a checked bounded splitter; application state owns `current`.
    ///
    /// # Errors
    ///
    /// Rejects nonfinite numbers, reversed or out-of-bounds ranges,
    /// nonpositive steps, and unrepresentable span/step arithmetic.
    pub fn new(
        label: impl Into<String>,
        minimum: f64,
        maximum: f64,
        current: f64,
        step: f64,
    ) -> Result<Self, SplitterError> {
        let min = SemanticNumber::new(minimum)?;
        let max = SemanticNumber::new(maximum)?;
        let value = SemanticNumber::new(current)?;
        let step = SemanticNumber::new(step)?;
        let range = SemanticRange::new(Some(min), Some(max), Some(value))?.with_small_step(step)?;
        let span = max.get() - min.get();
        if !span.is_finite() || !(span / step.get()).is_finite() {
            return Err(SplitterError::UnrepresentableSpan);
        }
        Ok(Self {
            label: label.into(),
            range,
            orientation: SemanticOrientation::Vertical,
            enabled: true,
            common: CommonNodeAuthoring::default(),
            on_resize: None,
        })
    }

    common_node_builder_methods!();

    /// Adds an optional larger step for PageUp/PageDown.
    ///
    /// # Errors
    ///
    /// Rejects a nonfinite or nonpositive page step.
    pub fn page_step(mut self, step: f64) -> Result<Self, SplitterError> {
        self.range = self.range.with_large_step(SemanticNumber::new(step)?)?;
        Ok(self)
    }

    #[must_use]
    pub fn value_text(mut self, text: impl Into<String>) -> Self {
        self.range = self
            .range
            .with_value_text(text)
            .unwrap_or_else(|_| unreachable!("checked splitter has current value"));
        self
    }

    #[must_use]
    pub const fn orientation(mut self, orientation: SemanticOrientation) -> Self {
        self.orientation = orientation;
        self
    }

    #[must_use]
    pub const fn enabled(mut self, enabled: bool) -> Self {
        self.enabled = enabled;
        self
    }

    #[must_use]
    pub const fn disabled(self) -> Self {
        self.enabled(false)
    }

    /// Emits intents into the ordinary routed application-action FIFO.
    ///
    /// The callback must not store a pane size in this widget. Apply each
    /// request against the application's current state and rebuild.
    #[must_use]
    pub fn on_resize(mut self, callback: impl FnMut(SplitterRequest) -> Action + 'static) -> Self {
        self.on_resize = Some(Box::new(callback));
        self
    }
}

/// Convenience constructor for checked application-owned splitter authoring.
///
/// # Errors
///
/// Returns `SplitterError` for invalid input.
pub fn splitter<Action>(
    label: impl Into<String>,
    minimum: f64,
    maximum: f64,
    current: f64,
    step: f64,
) -> Result<Splitter<Action>, SplitterError> {
    Splitter::new(label, minimum, maximum, current, step)
}

#[derive(Debug)]
struct SplitterState {
    label: String,
    range: SemanticRange,
    orientation: SemanticOrientation,
    enabled: bool,
    actionable: bool,
    drag: Option<PointerId>,
}

struct SplitterWidget<Action> {
    label: String,
    range: SemanticRange,
    orientation: SemanticOrientation,
    enabled: bool,
    on_resize: Option<Box<dyn FnMut(SplitterRequest) -> Action>>,
}

impl<Action> fmt::Debug for SplitterWidget<Action> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SplitterWidget")
            .field("label", &self.label)
            .field("range", &self.range)
            .field("orientation", &self.orientation)
            .field("enabled", &self.enabled)
            .field("actionable", &self.on_resize.is_some())
            .finish()
    }
}

fn signed_step(step: SemanticNumber, increase: bool) -> SplitterRequest {
    let delta = if increase { step.get() } else { -step.get() };
    SplitterRequest::AdjustBy(
        SemanticNumber::new(delta)
            .unwrap_or_else(|_| unreachable!("negating a finite step stays finite")),
    )
}

fn keyboard_request(
    state: &SplitterState,
    event: &crate::KeyboardEvent,
) -> Option<SplitterRequest> {
    if event.phase() != KeyboardPhase::Down || event.modifiers() != KeyModifiers::NONE {
        return None;
    }
    match (state.orientation, event.logical_key()) {
        (SemanticOrientation::Vertical, LogicalKey::ArrowRight)
        | (SemanticOrientation::Horizontal, LogicalKey::ArrowDown) => {
            state.range.small_step().map(|step| signed_step(step, true))
        }
        (SemanticOrientation::Vertical, LogicalKey::ArrowLeft)
        | (SemanticOrientation::Horizontal, LogicalKey::ArrowUp) => state
            .range
            .small_step()
            .map(|step| signed_step(step, false)),
        (_, LogicalKey::Home) => state.range.minimum().map(SplitterRequest::SetValue),
        (_, LogicalKey::End) => state.range.maximum().map(SplitterRequest::SetValue),
        (_, LogicalKey::PageDown) if state.range.large_step().is_some() => {
            state.range.large_step().map(|step| signed_step(step, true))
        }
        (_, LogicalKey::PageUp) if state.range.large_step().is_some() => state
            .range
            .large_step()
            .map(|step| signed_step(step, false)),
        _ => None,
    }
}

fn semantic_request(state: &SplitterState, command: SemanticCommand) -> Option<SplitterRequest> {
    match command {
        SemanticCommand::Increment => state.range.small_step().map(|step| signed_step(step, true)),
        SemanticCommand::Decrement => state
            .range
            .small_step()
            .map(|step| signed_step(step, false)),
        SemanticCommand::SetValue(value) => Some(SplitterRequest::SetValue(value)),
        _ => None,
    }
}

fn request_admissible(state: &SplitterState, request: SplitterRequest) -> bool {
    let (Some(min), Some(max), Some(_current)) = (
        state.range.minimum(),
        state.range.maximum(),
        state.range.current(),
    ) else {
        return false;
    };
    match request {
        // A published bound is not a live FIFO cursor: emitting one request can
        // move the application away from that bound before another routed event
        // is processed. Admission may reject zero, but MUST NOT filter direction
        // using the previous mounted value; application update owns clamping.
        SplitterRequest::AdjustBy(delta) => delta.get() != 0.0,
        // An absolute request equal to the last published value still matters:
        // queued earlier relative requests may already have moved app state.
        SplitterRequest::SetValue(value) => {
            (min.get()..=max.get()).contains(&value.get())
        }
        SplitterRequest::MoveBy(delta) => delta.is_finite() && delta != 0.0,
    }
}

fn usable_extent(context: &EventContext<'_, impl Sized>) -> bool {
    context.pointer_local_position().is_some()
        && context
            .pointer_local_size()
            .is_some_and(|size| size.width() > 0.0 && size.height() > 0.0)
}

impl<Action> SplitterWidget<Action> {
    fn actionable(&self) -> bool {
        self.on_resize.is_some() && self.range.minimum() != self.range.maximum()
    }

    fn emit(
        &mut self,
        state: &SplitterState,
        request: SplitterRequest,
        context: &mut EventContext<'_, Action>,
    ) {
        if request_admissible(state, request) {
            if let Some(callback) = self.on_resize.as_mut() {
                context.emit(callback(request));
            }
        }
    }
}

impl<Action> Widget<Action> for SplitterWidget<Action> {
    type State = SplitterState;

    fn create_state(&self) -> Self::State {
        SplitterState {
            label: self.label.clone(),
            range: self.range.clone(),
            orientation: self.orientation,
            enabled: self.enabled,
            actionable: self.actionable(),
            drag: None,
        }
    }

    fn update(&self, state: &mut Self::State, context: &mut WidgetUpdateContext<Action>) {
        if state.label != self.label {
            context.invalidate(WidgetInvalidation::SEMANTICS);
        }
        if state.range != self.range || state.orientation != self.orientation {
            context.invalidate(WidgetInvalidation::SEMANTICS | WidgetInvalidation::PAINT);
        }
        if state.enabled != self.enabled || state.actionable != self.actionable() {
            context.invalidate(
                WidgetInvalidation::INTERACTION
                    | WidgetInvalidation::HIT_TEST
                    | WidgetInvalidation::SEMANTICS
                    | WidgetInvalidation::PAINT,
            );
        }
        state.label.clone_from(&self.label);
        state.range.clone_from(&self.range);
        state.orientation = self.orientation;
        state.enabled = self.enabled;
        state.actionable = self.actionable();
        // Keep a capture lifetime only, never a last size or a pending resize.
    }

    fn activation(&self, _: &Self::State) -> WidgetActivation {
        if self.actionable() {
            WidgetActivation::actionable(self.enabled)
        } else if self.enabled {
            WidgetActivation::NONE
        } else {
            WidgetActivation::disabled()
        }
    }

    fn event(
        &mut self,
        state: &mut Self::State,
        event: &UiEvent,
        context: &mut EventContext<'_, Action>,
    ) -> WidgetEventOutput {
        if context.phase() != EventPhase::Target {
            return WidgetEventOutput::none();
        }
        if let Some(capture) = event.as_pointer_capture()
            && capture.kind() == PointerCaptureKind::Lost
            && state.drag == Some(capture.pointer_id())
        {
            state.drag = None;
            return WidgetEventOutput::changed();
        }
        if let Some(pointer) = event.as_pointer()
            && state.drag == Some(pointer.pointer_id())
            && (matches!(pointer.phase(), PointerPhase::Up | PointerPhase::Cancel)
                || !state.enabled
                || !state.actionable)
        {
            state.drag = None;
            context.release_pointer_capture();
            context.prevent_default();
            context.stop_propagation();
            return WidgetEventOutput::changed();
        }
        if !state.enabled || !state.actionable || context.default_is_prevented() {
            return WidgetEventOutput::none();
        }
        if let Some(key) = event.as_keyboard() {
            if let Some(request) = keyboard_request(state, key) {
                self.emit(state, request, context);
                context.prevent_default();
                context.stop_propagation();
            }
            return WidgetEventOutput::none();
        }
        if let Some(command) = event.as_semantic_command() {
            if let Some(request) = semantic_request(state, command.command()) {
                self.emit(state, request, context);
                context.prevent_default();
                context.stop_propagation();
            }
            return WidgetEventOutput::none();
        }
        let Some(pointer) = event.as_pointer() else {
            return WidgetEventOutput::none();
        };
        let active = state.drag == Some(pointer.pointer_id());
        match pointer.phase() {
            PointerPhase::Down
                if state.drag.is_none()
                    && (pointer.device_kind() == PointerDeviceKind::Touch
                        || pointer.changed_button() == Some(PointerButton::Primary)) =>
            {
                if !usable_extent(context) {
                    return WidgetEventOutput::none();
                }
                state.drag = Some(pointer.pointer_id());
                context.capture_pointer();
                context.prevent_default();
                context.stop_propagation();
                WidgetEventOutput::changed()
            }
            PointerPhase::Move if active => {
                if !usable_extent(context) {
                    state.drag = None;
                    context.release_pointer_capture();
                    return WidgetEventOutput::changed();
                }
                let delta = pointer.movement_delta();
                let axis = match state.orientation {
                    SemanticOrientation::Vertical => delta.x(),
                    SemanticOrientation::Horizontal => delta.y(),
                };
                self.emit(state, SplitterRequest::MoveBy(axis), context);
                context.prevent_default();
                context.stop_propagation();
                WidgetEventOutput::none()
            }
            _ => WidgetEventOutput::none(),
        }
    }

    fn measure(&self, _: &Self::State, _: WidgetMeasureInput) -> WidgetMeasure {
        let (width, height) = match self.orientation {
            SemanticOrientation::Vertical => (12.0, 160.0),
            SemanticOrientation::Horizontal => (160.0, 12.0),
        };
        WidgetMeasure::measured(
            LogicalLength::new(width).unwrap_or_else(|_| unreachable!("finite width")),
            LogicalLength::new(height).unwrap_or_else(|_| unreachable!("finite height")),
        )
    }

    fn paint(&self, state: &Self::State, context: PaintContributionContext) -> PaintContribution {
        let size = context.local_size();
        if size.width() <= 0.0 || size.height() <= 0.0 {
            return PaintContribution::empty();
        }
        let rect = match state.orientation {
            SemanticOrientation::Vertical => LogicalRect::try_new(
                (size.width() - size.width().min(2.0)) / 2.0,
                0.0,
                size.width().min(2.0),
                size.height(),
            ),
            SemanticOrientation::Horizontal => LogicalRect::try_new(
                0.0,
                (size.height() - size.height().min(2.0)) / 2.0,
                size.width(),
                size.height().min(2.0),
            ),
        }
        .unwrap_or_else(|_| unreachable!("checked local geometry"));
        let brush = Brush::solid(
            context
                .computed_style()
                .foreground()
                .unwrap_or(Color::rgba(130, 140, 155, 255)),
        );
        PaintContribution::new(vec![PaintContributionItem::fill(
            SceneShape::rect(rect),
            brush,
        )])
    }

    fn hit_test(&self, state: &Self::State, context: HitContributionContext) -> HitContribution {
        if !state.enabled || !state.actionable {
            return HitContribution::empty();
        }
        let size = context.local_size();
        HitContribution::single_rect(
            LogicalRect::try_new(0.0, 0.0, size.width(), size.height())
                .unwrap_or_else(|_| unreachable!("valid final local size")),
        )
    }

    fn semantics(
        &self,
        state: &Self::State,
        _: SemanticContributionContext,
    ) -> SemanticContribution {
        let mut node = SemanticNodeContribution::primary(SemanticRole::Splitter)
            .with_name(state.label.clone())
            .with_orientation(state.orientation)
            .with_range(state.range.clone())
            .with_state(SemanticState::ENABLED.with_disabled(!state.enabled));
        if state.enabled && state.actionable {
            node = node
                .with_action(SemanticAction::Increment)
                .with_action(SemanticAction::Decrement)
                .with_action(SemanticAction::SetValue);
        }
        SemanticContribution::single(node)
    }
}

impl<Action: 'static> View<Action> for Splitter<Action> {
    fn into_element(self) -> Element<Action> {
        let (fields, diagnostics) = self
            .common
            .into_authored_fields(Focusability::Automatic, None);
        Element::from_authored_parts(
            fields,
            Box::new(WidgetAdapter(SplitterWidget {
                label: self.label,
                range: self.range,
                orientation: self.orientation,
                enabled: self.enabled,
                on_resize: self.on_resize,
            })),
            Vec::new(),
            diagnostics,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::{
        Splitter, SplitterError, SplitterRequest, SplitterState, keyboard_request,
        request_admissible,
    };
    use crate::{
        KeyLocation, KeyModifiers, KeyboardCompositionState, KeyboardEvent, KeyboardPhase,
        LogicalKey, PhysicalKey, SemanticNumber, SemanticOrientation, SemanticRangeError,
    };

    #[test]
    fn rejects_invalid_ranges_and_does_not_invent_pane_size() {
        assert!(matches!(
            Splitter::<()>::new("Pane", 0.0, 1.0, f64::NAN, 1.0),
            Err(SplitterError::NonFinite)
        ));
        assert!(matches!(
            Splitter::<()>::new("Pane", 2.0, 1.0, 1.0, 1.0),
            Err(SplitterError::Range(SemanticRangeError::ReversedBounds))
        ));
        assert!(matches!(
            Splitter::<()>::new("Pane", 0.0, 1.0, 2.0, 1.0),
            Err(SplitterError::Range(
                SemanticRangeError::CurrentAboveMaximum
            ))
        ));
        assert!(matches!(
            Splitter::<()>::new("Pane", 0.0, 1.0, 0.5, 0.0),
            Err(SplitterError::Range(
                SemanticRangeError::NonPositiveSmallStep
            ))
        ));
        assert!(matches!(
            Splitter::<()>::new("Pane", -1e308, 1e308, 0.0, 1.0),
            Err(SplitterError::UnrepresentableSpan)
        ));
        assert!(Splitter::<()>::new("Pane", 0.0, 100.0, 50.0, 5.0).is_ok());
    }

    #[test]
    fn divider_orientation_is_not_slider_orientation() {
        let splitter =
            Splitter::<()>::new("Pane", 0.0, 100.0, 50.0, 5.0).unwrap_or_else(|_| unreachable!());
        let state = SplitterState {
            label: splitter.label,
            range: splitter.range,
            orientation: splitter.orientation,
            enabled: true,
            actionable: true,
            drag: None,
        };
        let key = |logical| {
            KeyboardEvent::new(
                KeyboardPhase::Down,
                PhysicalKey::ArrowRight,
                logical,
                KeyModifiers::NONE,
                false,
                KeyLocation::Standard,
                KeyboardCompositionState::Inactive,
                None,
            )
        };
        assert_eq!(
            keyboard_request(&state, &key(LogicalKey::ArrowRight)),
            Some(SplitterRequest::AdjustBy(
                SemanticNumber::new(5.0).unwrap_or_else(|_| unreachable!())
            ))
        );
        assert_eq!(keyboard_request(&state, &key(LogicalKey::ArrowUp)), None);
        let mut across = state;
        across.orientation = SemanticOrientation::Horizontal;
        assert_eq!(
            keyboard_request(&across, &key(LogicalKey::ArrowDown)),
            Some(SplitterRequest::AdjustBy(
                SemanticNumber::new(5.0).unwrap_or_else(|_| unreachable!())
            ))
        );
        assert_eq!(
            keyboard_request(&across, &key(LogicalKey::ArrowUp)),
            Some(SplitterRequest::AdjustBy(
                SemanticNumber::new(-5.0).unwrap_or_else(|_| unreachable!())
            ))
        );
        assert_eq!(
            keyboard_request(&across, &key(LogicalKey::ArrowRight)),
            None
        );
        assert!(!request_admissible(&across, SplitterRequest::MoveBy(0.0)));
        assert!(!request_admissible(
            &across,
            SplitterRequest::SetValue(
                SemanticNumber::new(150.0).unwrap_or_else(|_| unreachable!())
            )
        ));
    }
}
