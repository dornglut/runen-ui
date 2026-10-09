//! Application-controlled scalar range input using the ordinary widget and routed-action protocol.
//!
//! Slider retains only the current pointer capture lifetime. Numeric value and
//! accessible value text are rebuilt from application state.

use core::fmt;

use crate::{
    Brush, Color, EventContext, EventPhase, Focusability, HitContribution, HitContributionContext,
    KeyModifiers, KeyboardPhase, LogicalKey, LogicalLength, LogicalPoint, LogicalRect, LogicalSize,
    PaintContribution, PaintContributionContext, PaintContributionItem, PointerButton,
    PointerCaptureKind, PointerDeviceKind, PointerId, PointerPhase, SceneShape, SemanticAction,
    SemanticCommand, SemanticContribution, SemanticContributionContext, SemanticNodeContribution,
    SemanticNumber, SemanticNumberError, SemanticOrientation, SemanticRange, SemanticRangeError,
    SemanticRole, SemanticState, UiEvent, WidgetActivation, WidgetEventOutput, WidgetInvalidation,
    WidgetMeasure, WidgetMeasureInput, WidgetUpdateContext,
    element::{CommonNodeAuthoring, Element, View, common_node_builder_methods},
    widget_erasure::WidgetAdapter,
    widget_protocol::Widget,
};

/// Rejected scalar control input. No invalid `Slider` can be authored.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SliderError {
    NonFinite,
    Range(SemanticRangeError),
    UnrepresentableSpan,
}

impl fmt::Display for SliderError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NonFinite => f.write_str("slider values and steps must be finite"),
            Self::Range(error) => error.fmt(f),
            Self::UnrepresentableSpan => {
                f.write_str("slider span or step count is not finitely representable")
            }
        }
    }
}

impl std::error::Error for SliderError {}

impl From<SemanticNumberError> for SliderError {
    fn from(_: SemanticNumberError) -> Self {
        Self::NonFinite
    }
}

impl From<SemanticRangeError> for SliderError {
    fn from(error: SemanticRangeError) -> Self {
        Self::Range(error)
    }
}

/// Typed scalar range input. Its durable value belongs exclusively to the application.
pub struct Slider<Action> {
    label: String,
    range: SemanticRange,
    orientation: SemanticOrientation,
    enabled: bool,
    common: CommonNodeAuthoring,
    on_change: Option<Box<dyn FnMut(SemanticNumber) -> Action>>,
}

impl<Action> fmt::Debug for Slider<Action> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Slider")
            .field("label", &self.label)
            .field("range", &self.range)
            .field("orientation", &self.orientation)
            .field("enabled", &self.enabled)
            .field("actionable", &self.on_change.is_some())
            .finish_non_exhaustive()
    }
}

impl<Action> Slider<Action> {
    /// Constructs a finite bounded range with a positive small step.
    ///
    /// Endpoints are always reachable even when the span is not an integral
    /// multiple of the small step. Intermediate pointer values snap to that step.
    ///
    /// # Errors
    ///
    /// Rejects nonfinite inputs, reversed/out-of-bounds ranges, nonpositive
    /// steps, and spans whose arithmetic cannot be represented faithfully.
    pub fn new(
        label: impl Into<String>,
        minimum: f64,
        maximum: f64,
        current: f64,
        step: f64,
    ) -> Result<Self, SliderError> {
        let min = SemanticNumber::new(minimum)?;
        let max = SemanticNumber::new(maximum)?;
        let value = SemanticNumber::new(current)?;
        let step = SemanticNumber::new(step)?;
        let range = SemanticRange::new(Some(min), Some(max), Some(value))?.with_small_step(step)?;
        validate_span(&range)?;
        Ok(Self {
            label: label.into(),
            range,
            orientation: SemanticOrientation::Horizontal,
            enabled: true,
            common: CommonNodeAuthoring::default(),
            on_change: None,
        })
    }

    common_node_builder_methods!();

    /// Sets an explicit positive page step; otherwise page input moves ten
    /// small steps, saturating at the authored endpoints.
    ///
    /// # Errors
    ///
    /// Rejects nonfinite or nonpositive steps.
    pub fn page_step(mut self, step: f64) -> Result<Self, SliderError> {
        self.range = self.range.with_large_step(SemanticNumber::new(step)?)?;
        Ok(self)
    }

    #[must_use]
    pub fn value_text(mut self, text: impl Into<String>) -> Self {
        self.range = self
            .range
            .with_value_text(text)
            .unwrap_or_else(|_| unreachable!("Slider always has a checked current value"));
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

    /// Emits a value proposal; only application update and rebuild commit it.
    #[must_use]
    pub fn on_change(mut self, action: impl FnMut(SemanticNumber) -> Action + 'static) -> Self {
        self.on_change = Some(Box::new(action));
        self
    }
}

/// Convenience constructor for the checked Slider authoring surface.
///
/// # Errors
///
/// Returns `SliderError` for any invalid bounded scalar input.
pub fn slider<Action>(
    label: impl Into<String>,
    minimum: f64,
    maximum: f64,
    current: f64,
    step: f64,
) -> Result<Slider<Action>, SliderError> {
    Slider::new(label, minimum, maximum, current, step)
}

fn validate_span(range: &SemanticRange) -> Result<(), SliderError> {
    let (Some(min), Some(max), Some(step)) = (range.minimum(), range.maximum(), range.small_step())
    else {
        return Err(SliderError::UnrepresentableSpan);
    };
    let span = max.get() - min.get();
    if !span.is_finite() || !(span / step.get()).is_finite() {
        return Err(SliderError::UnrepresentableSpan);
    }
    Ok(())
}

#[derive(Debug)]
struct SliderState {
    label: String,
    range: SemanticRange,
    orientation: SemanticOrientation,
    enabled: bool,
    actionable: bool,
    drag: Option<PointerId>,
}

struct SliderWidget<Action> {
    label: String,
    range: SemanticRange,
    orientation: SemanticOrientation,
    enabled: bool,
    on_change: Option<Box<dyn FnMut(SemanticNumber) -> Action>>,
}

impl<Action> fmt::Debug for SliderWidget<Action> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("SliderWidget")
            .field("label", &self.label)
            .field("range", &self.range)
            .field("orientation", &self.orientation)
            .field("enabled", &self.enabled)
            .field("actionable", &self.on_change.is_some())
            .finish()
    }
}

/// One geometry law for final local paint and displayed-frame pointer input.
#[derive(Clone, Copy, Debug)]
struct SliderGeometry {
    length: f32,
    center: f32,
    radius: f32,
    travel: f32,
    orientation: SemanticOrientation,
}

impl SliderGeometry {
    fn new(size: LogicalSize, orientation: SemanticOrientation) -> Option<Self> {
        let (length, cross) = match orientation {
            SemanticOrientation::Horizontal => (size.width(), size.height()),
            SemanticOrientation::Vertical => (size.height(), size.width()),
        };
        if length <= 0.0 || cross <= 0.0 {
            return None;
        }
        let radius = (cross * 0.45).min(10.0).min(length / 2.0);
        let travel = 2.0_f32.mul_add(-radius, length);
        if travel <= 0.0 {
            return None;
        }
        Some(Self {
            length,
            center: cross / 2.0,
            radius,
            travel,
            orientation,
        })
    }

    fn fraction_at(self, local: LogicalPoint) -> f64 {
        let coordinate = match self.orientation {
            SemanticOrientation::Horizontal => local.x(),
            SemanticOrientation::Vertical => self.length - local.y(),
        };
        f64::from(((coordinate - self.radius) / self.travel).clamp(0.0, 1.0))
    }

    fn value_at(self, range: &SemanticRange, position: LogicalPoint) -> Option<SemanticNumber> {
        let min = range.minimum()?.get();
        let max = range.maximum()?.get();
        let proposed = (max - min)
            .mul_add(self.fraction_at(position), min)
            .clamp(min, max);
        numeric_value(range, proposed)
    }

    fn rect(self, start: f32, along: f32, cross: f32) -> LogicalRect {
        let (x, y, w, h) = match self.orientation {
            SemanticOrientation::Horizontal => (start, self.center - cross / 2.0, along, cross),
            SemanticOrientation::Vertical => (
                self.center - cross / 2.0,
                self.length - start - along,
                cross,
                along,
            ),
        };
        LogicalRect::try_new(x, y, w, h)
            .unwrap_or_else(|_| unreachable!("bounded slider local geometry is finite"))
    }
}

fn numeric_value(range: &SemanticRange, proposed: f64) -> Option<SemanticNumber> {
    let (min, max, step) = (range.minimum()?, range.maximum()?, range.small_step()?);
    if !proposed.is_finite() || !(min.get()..=max.get()).contains(&proposed) {
        return None;
    }
    if proposed <= min.get() {
        return Some(min);
    }
    if proposed >= max.get() {
        return Some(max);
    }
    let count = ((proposed - min.get()) / step.get()).round();
    let value = count
        .mul_add(step.get(), min.get())
        .clamp(min.get(), max.get());
    SemanticNumber::new(value).ok()
}

fn requested_value(range: &SemanticRange, command: SemanticCommand) -> Option<SemanticNumber> {
    let min = range.minimum()?.get();
    let max = range.maximum()?.get();
    let current = range.current()?.get();
    let small = range.small_step()?.get();
    match command {
        SemanticCommand::Increment => numeric_value(range, (current + small).min(max)),
        SemanticCommand::Decrement => numeric_value(range, (current - small).max(min)),
        SemanticCommand::SetValue(value) => numeric_value(range, value.get()),
        _ => None,
    }
}

fn keyboard_value(
    range: &SemanticRange,
    orientation: SemanticOrientation,
    event: &crate::KeyboardEvent,
) -> Option<SemanticNumber> {
    if event.phase() != KeyboardPhase::Down || event.modifiers() != KeyModifiers::NONE {
        return None;
    }
    let command = match (orientation, event.logical_key()) {
        (SemanticOrientation::Horizontal, LogicalKey::ArrowRight)
        | (SemanticOrientation::Vertical, LogicalKey::ArrowUp) => SemanticCommand::Increment,
        (SemanticOrientation::Horizontal, LogicalKey::ArrowLeft)
        | (SemanticOrientation::Vertical, LogicalKey::ArrowDown) => SemanticCommand::Decrement,
        (_, LogicalKey::Home) => return range.minimum(),
        (_, LogicalKey::End) => return range.maximum(),
        (_, LogicalKey::PageUp | LogicalKey::PageDown) => {
            let min = range.minimum()?.get();
            let max = range.maximum()?.get();
            let current = range.current()?.get();
            let small = range.small_step()?.get();
            let page = range
                .large_step()
                .map_or_else(|| (small * 10.0).min(max - min), SemanticNumber::get);
            let value = if matches!(event.logical_key(), LogicalKey::PageUp) {
                (current + page).min(max)
            } else {
                (current - page).max(min)
            };
            return numeric_value(range, value);
        }
        _ => return None,
    };
    requested_value(range, command)
}

impl<Action> SliderWidget<Action> {
    fn actionable(&self) -> bool {
        self.on_change.is_some() && self.range.minimum() != self.range.maximum()
    }

    fn emit_value(
        &mut self,
        state: &SliderState,
        proposed: Option<SemanticNumber>,
        context: &mut EventContext<'_, Action>,
    ) {
        let Some(value) = proposed.filter(|value| Some(*value) != state.range.current()) else {
            return;
        };
        if let Some(callback) = self.on_change.as_mut() {
            context.emit(callback(value));
        }
    }
}

impl<Action> Widget<Action> for SliderWidget<Action> {
    type State = SliderState;

    fn create_state(&self) -> Self::State {
        SliderState {
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
            context.invalidate(WidgetInvalidation::PAINT | WidgetInvalidation::SEMANTICS);
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
        // Preserve only the pending capture lifetime across disabled rebuilds.
        // The next routed pointer/capture event releases it; never retain a value.
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
            && state.drag.as_ref() == Some(&capture.pointer_id())
        {
            state.drag = None;
            return WidgetEventOutput::changed();
        }
        if let Some(pointer) = event.as_pointer()
            && state.drag.as_ref() == Some(&pointer.pointer_id())
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
            let value = keyboard_value(&state.range, state.orientation, key);
            if value.is_some() {
                self.emit_value(state, value, context);
                context.prevent_default();
                context.stop_propagation();
            }
            return WidgetEventOutput::none();
        }
        if let Some(command) = event.as_semantic_command() {
            let value = requested_value(&state.range, command.command());
            if value.is_some() {
                self.emit_value(state, value, context);
                context.prevent_default();
                context.stop_propagation();
            }
            return WidgetEventOutput::none();
        }
        let Some(pointer) = event.as_pointer() else {
            return WidgetEventOutput::none();
        };
        let active = state.drag.as_ref() == Some(&pointer.pointer_id());
        match pointer.phase() {
            PointerPhase::Down
                if state.drag.is_none()
                    && (pointer.device_kind() == PointerDeviceKind::Touch
                        || pointer.changed_button() == Some(PointerButton::Primary)) =>
            {
                let (Some(position), Some(size)) = (
                    context.pointer_local_position(),
                    context.pointer_local_size(),
                ) else {
                    return WidgetEventOutput::none();
                };
                let Some(geometry) = SliderGeometry::new(size, state.orientation) else {
                    return WidgetEventOutput::none();
                };
                state.drag = Some(pointer.pointer_id());
                context.capture_pointer();
                self.emit_value(state, geometry.value_at(&state.range, position), context);
                context.prevent_default();
                context.stop_propagation();
                WidgetEventOutput::changed()
            }
            PointerPhase::Move if active => {
                let (Some(position), Some(size)) = (
                    context.pointer_local_position(),
                    context.pointer_local_size(),
                ) else {
                    state.drag = None;
                    context.release_pointer_capture();
                    return WidgetEventOutput::changed();
                };
                let Some(geometry) = SliderGeometry::new(size, state.orientation) else {
                    state.drag = None;
                    context.release_pointer_capture();
                    return WidgetEventOutput::changed();
                };
                self.emit_value(state, geometry.value_at(&state.range, position), context);
                context.prevent_default();
                context.stop_propagation();
                WidgetEventOutput::none()
            }
            _ => WidgetEventOutput::none(),
        }
    }

    fn measure(&self, _: &Self::State, _: WidgetMeasureInput) -> WidgetMeasure {
        let (width, height) = match self.orientation {
            SemanticOrientation::Horizontal => (160.0, 24.0),
            SemanticOrientation::Vertical => (24.0, 160.0),
        };
        WidgetMeasure::measured(
            LogicalLength::new(width)
                .unwrap_or_else(|_| unreachable!("fixed intrinsic width is finite")),
            LogicalLength::new(height)
                .unwrap_or_else(|_| unreachable!("fixed intrinsic height is finite")),
        )
    }

    fn paint(&self, state: &Self::State, context: PaintContributionContext) -> PaintContribution {
        let Some(geometry) = SliderGeometry::new(context.local_size(), state.orientation) else {
            return PaintContribution::empty();
        };
        let Some(min) = state.range.minimum() else {
            return PaintContribution::empty();
        };
        let Some(max) = state.range.maximum() else {
            return PaintContribution::empty();
        };
        let Some(current) = state.range.current() else {
            return PaintContribution::empty();
        };
        let span = max.get() - min.get();
        let fraction = if span > 0.0 {
            ((current.get() - min.get()) / span).clamp(0.0, 1.0)
        } else {
            0.0
        };
        #[allow(clippy::cast_possible_truncation)]
        let filled = geometry.travel * fraction as f32;
        let track = context
            .computed_style()
            .background()
            .cloned()
            .unwrap_or_else(|| Brush::solid(Color::rgba(170, 170, 170, 255)));
        let thumb = context
            .computed_style()
            .foreground()
            .unwrap_or(Color::rgba(70, 115, 205, 255));
        let thickness = (geometry.radius * 0.5).max(1.0);
        let start = geometry.radius;
        let items = vec![
            PaintContributionItem::fill(
                SceneShape::rect(geometry.rect(start, geometry.travel, thickness)),
                track,
            ),
            PaintContributionItem::fill(
                SceneShape::rect(geometry.rect(start, filled, thickness)),
                Brush::solid(thumb),
            ),
            PaintContributionItem::fill(
                SceneShape::ellipse(geometry.rect(
                    start + filled - geometry.radius,
                    2.0 * geometry.radius,
                    2.0 * geometry.radius,
                )),
                Brush::solid(thumb),
            ),
        ];
        PaintContribution::new(items)
    }

    fn hit_test(&self, state: &Self::State, context: HitContributionContext) -> HitContribution {
        if !state.enabled || !state.actionable {
            return HitContribution::empty();
        }
        let size = context.local_size();
        HitContribution::single_rect(
            LogicalRect::try_new(0.0, 0.0, size.width(), size.height())
                .unwrap_or_else(|_| unreachable!("final local size is valid")),
        )
    }

    fn semantics(
        &self,
        state: &Self::State,
        _: SemanticContributionContext,
    ) -> SemanticContribution {
        let mut node = SemanticNodeContribution::primary(SemanticRole::Slider)
            .with_name(state.label.clone())
            .with_range(state.range.clone())
            .with_orientation(state.orientation)
            .with_state(SemanticState::ENABLED.with_disabled(!state.enabled));
        if state.actionable && state.enabled {
            node = node
                .with_action(SemanticAction::Increment)
                .with_action(SemanticAction::Decrement)
                .with_action(SemanticAction::SetValue);
        }
        SemanticContribution::single(node)
    }
}

impl<Action: 'static> View<Action> for Slider<Action> {
    fn into_element(self) -> Element<Action> {
        let (fields, diagnostics) = self
            .common
            .into_authored_fields(Focusability::Automatic, None);
        Element::from_authored_parts(
            fields,
            Box::new(WidgetAdapter(SliderWidget {
                label: self.label,
                range: self.range,
                orientation: self.orientation,
                enabled: self.enabled,
                on_change: self.on_change,
            })),
            Vec::new(),
            diagnostics,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::{Slider, SliderError, SliderGeometry, keyboard_value, numeric_value};
    use crate::{
        KeyLocation, KeyModifiers, KeyboardCompositionState, KeyboardEvent, KeyboardPhase,
        LogicalKey, LogicalSize, PhysicalKey, SemanticNumber, SemanticOrientation,
        SemanticRangeError,
    };

    #[test]
    fn no_callback_and_collapsed_range_are_semantically_readable_not_actionable() {
        use super::SliderWidget;
        use crate::Widget;

        let range = Slider::<()>::new("Volume", 0.0, 100.0, 50.0, 5.0)
            .unwrap_or_else(|_| unreachable!("valid numeric range"));
        let passive = SliderWidget::<()> {
            label: range.label,
            range: range.range,
            orientation: range.orientation,
            enabled: true,
            on_change: None,
        };
        let passive_state = passive.create_state();
        assert!(!passive.activation(&passive_state).is_actionable());

        let collapsed = Slider::<()>::new("Locked", 4.0, 4.0, 4.0, 1.0)
            .unwrap_or_else(|_| unreachable!("valid degenerate bounded value"));
        let noninteractive = SliderWidget {
            label: collapsed.label,
            range: collapsed.range,
            orientation: collapsed.orientation,
            enabled: true,
            on_change: Some(Box::new(|_| ())),
        };
        let state = noninteractive.create_state();
        assert!(!state.actionable);
        assert!(!noninteractive.activation(&state).is_actionable());
    }

    #[test]
    fn invalid_numeric_authoring_fails_before_mount() {
        assert!(matches!(
            Slider::<()>::new("Volume", 0.0, 1.0, f64::NAN, 0.1),
            Err(SliderError::NonFinite)
        ));
        assert!(matches!(
            Slider::<()>::new("Volume", 3.0, 1.0, 2.0, 0.1),
            Err(SliderError::Range(SemanticRangeError::ReversedBounds))
        ));
        assert!(matches!(
            Slider::<()>::new("Volume", 0.0, 1.0, 2.0, 0.1),
            Err(SliderError::Range(SemanticRangeError::CurrentAboveMaximum))
        ));
        assert!(matches!(
            Slider::<()>::new("Volume", 0.0, 1.0, 0.5, 0.0),
            Err(SliderError::Range(SemanticRangeError::NonPositiveSmallStep))
        ));
        assert!(matches!(
            Slider::<()>::new("Volume", -1e308, 1e308, 0.0, 1.0),
            Err(SliderError::UnrepresentableSpan)
        ));
        assert!(matches!(
            Slider::<()>::new("Volume", 0.0, 1.0, 0.5, 1e-320),
            Err(SliderError::UnrepresentableSpan)
        ));
        assert!(Slider::<()>::new("Volume", 0.0, 1.0, 0.5, 0.1).is_ok());
    }

    #[test]
    fn geometry_is_oriented_and_degenerate_layout_is_inert() {
        assert!(SliderGeometry::new(LogicalSize::ZERO, SemanticOrientation::Horizontal).is_none());
        let size = LogicalSize::new(
            crate::LogicalLength::new(160.0).unwrap_or_else(|_| unreachable!()),
            crate::LogicalLength::new(24.0).unwrap_or_else(|_| unreachable!()),
        );
        let horizontal = SliderGeometry::new(size, SemanticOrientation::Horizontal)
            .unwrap_or_else(|| unreachable!());
        assert!(horizontal.travel > 0.0);
        assert!(SliderGeometry::new(size, SemanticOrientation::Vertical).is_some());
    }

    #[test]
    fn snapped_values_preserve_both_endpoints() {
        let slider =
            Slider::<()>::new("Scale", 0.0, 1.0, 0.5, 0.3).unwrap_or_else(|_| unreachable!());
        assert_eq!(
            numeric_value(&slider.range, 1.0),
            SemanticNumber::new(1.0).ok()
        );
        assert_eq!(
            numeric_value(&slider.range, 0.0),
            SemanticNumber::new(0.0).ok()
        );
        assert_eq!(
            numeric_value(&slider.range, 0.4),
            SemanticNumber::new(0.3).ok()
        );
        assert!(numeric_value(&slider.range, 2.0).is_none());
    }

    #[test]
    fn keyboard_orientation_is_value_semantic_not_screen_scroll() {
        let slider =
            Slider::<()>::new("Scale", 0.0, 100.0, 50.0, 5.0).unwrap_or_else(|_| unreachable!());
        let event = KeyboardEvent::new(
            KeyboardPhase::Down,
            PhysicalKey::ArrowUp,
            LogicalKey::ArrowUp,
            KeyModifiers::NONE,
            false,
            KeyLocation::Standard,
            KeyboardCompositionState::Inactive,
            None,
        );
        assert_eq!(
            keyboard_value(&slider.range, SemanticOrientation::Vertical, &event),
            SemanticNumber::new(55.0).ok()
        );
        assert_eq!(
            keyboard_value(&slider.range, SemanticOrientation::Horizontal, &event),
            None
        );
    }

    #[test]
    fn vertical_pointer_and_page_policy_follow_checked_orientation_and_step() {
        let vertical = SliderGeometry::new(
            LogicalSize::try_new(24.0, 160.0).unwrap_or_else(|_| unreachable!("finite extent")),
            SemanticOrientation::Vertical,
        )
        .unwrap_or_else(|| unreachable!("positive vertical track"));
        let bottom =
            crate::LogicalPoint::new(12.0, 160.0).unwrap_or_else(|_| unreachable!("finite point"));
        let top =
            crate::LogicalPoint::new(12.0, 0.0).unwrap_or_else(|_| unreachable!("finite point"));
        assert_eq!(vertical.fraction_at(bottom), 0.0);
        assert_eq!(vertical.fraction_at(top), 1.0);

        let slider = Slider::<()>::new("Scale", 0.0, 100.0, 50.0, 5.0)
            .unwrap_or_else(|_| unreachable!("finite range"))
            .page_step(20.0)
            .unwrap_or_else(|_| unreachable!("positive page step"));
        let page = KeyboardEvent::new(
            KeyboardPhase::Down,
            PhysicalKey::PageUp,
            LogicalKey::PageUp,
            KeyModifiers::NONE,
            false,
            KeyLocation::Standard,
            KeyboardCompositionState::Inactive,
            None,
        );
        assert_eq!(
            keyboard_value(&slider.range, SemanticOrientation::Vertical, &page),
            SemanticNumber::new(70.0).ok()
        );
    }
}
