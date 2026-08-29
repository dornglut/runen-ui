use std::collections::BTreeSet;

use runenui_core::{
    InputDeviceId, KeyModifiers, LogicalDelta, LogicalPoint, PointerButton, PointerButtons,
    PointerDeviceKind, PointerEvent, PointerId, PointerPhase, SurfaceInputContext,
};
use winit::{
    dpi::PhysicalPosition,
    event::{ElementState, MouseButton},
};

const FIRST_POINTER_ID: u64 = 1;
const BACK_BUTTON_ID: u16 = 4;
const FORWARD_BUTTON_ID: u16 = 5;

type DeviceButton = (InputDeviceId, MouseButton);

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TranslatedPointerPoint {
    pub position: LogicalPoint,
    pub input_context: SurfaceInputContext,
    pub modifiers: KeyModifiers,
}

impl TranslatedPointerPoint {
    #[must_use]
    pub const fn with_modifiers(mut self, modifiers: KeyModifiers) -> Self {
        self.modifiers = modifiers;
        self
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MouseIngressDiagnostic {
    DuplicatePress(MouseButton),
    UnmatchedRelease(MouseButton),
    PointUnavailableAtPress(MouseButton),
    SuppressedRelease(MouseButton),
    MissingActiveStream(MouseButton),
    DeviceMismatch {
        active: InputDeviceId,
        incoming: InputDeviceId,
    },
    MovementDeltaOutOfRange,
    PointerIdentityExhausted,
}

#[derive(Debug)]
pub enum MouseButtonOutcome {
    Submit(PointerEvent),
    Suppressed(MouseIngressDiagnostic),
}

#[derive(Clone, Debug)]
struct MousePointerStream {
    pointer_id: PointerId,
    device_id: InputDeviceId,
    position: LogicalPoint,
    input_context: SurfaceInputContext,
}

#[derive(Debug)]
pub struct MouseInputState {
    next_pointer_id: Option<u64>,
    active_stream: Option<MousePointerStream>,
    last_native_position: Option<PhysicalPosition<f64>>,
    pressed_buttons: BTreeSet<DeviceButton>,
    suppressed_buttons: BTreeSet<DeviceButton>,
}

impl Default for MouseInputState {
    fn default() -> Self {
        Self {
            next_pointer_id: Some(FIRST_POINTER_ID),
            active_stream: None,
            last_native_position: None,
            pressed_buttons: BTreeSet::new(),
            suppressed_buttons: BTreeSet::new(),
        }
    }
}

impl MouseInputState {
    pub const fn note_cursor_position(&mut self, position: PhysicalPosition<f64>) {
        self.last_native_position = Some(position);
    }

    #[must_use]
    pub const fn last_native_position(&self) -> Option<PhysicalPosition<f64>> {
        self.last_native_position
    }

    #[must_use]
    pub const fn active_device_id(&self) -> Option<InputDeviceId> {
        match &self.active_stream {
            Some(stream) => Some(stream.device_id),
            None => None,
        }
    }

    /// Advances the neutral pointer stream from one translated native cursor move.
    ///
    /// # Errors
    ///
    /// Returns [`MouseIngressDiagnostic`] when the incoming device conflicts with the active
    /// stream, the movement delta cannot be represented, or pointer identity is exhausted.
    pub fn cursor_moved(
        &mut self,
        device_id: InputDeviceId,
        point: TranslatedPointerPoint,
    ) -> Result<PointerEvent, MouseIngressDiagnostic> {
        self.validate_active_device(device_id)?;
        let movement_delta = logical_movement_delta(
            self.active_stream.as_ref().map(|stream| stream.position),
            point.position,
        )?;
        let pointer_id = self.ensure_active_stream(device_id, &point)?;
        let buttons = self.effective_buttons(device_id);
        self.update_active_stream(&point);
        Ok(PointerEvent::new(
            pointer_id,
            PointerDeviceKind::Mouse,
            PointerPhase::Move,
            point.position,
            point.input_context,
        )
        .with_device_id(device_id)
        .with_movement_delta(movement_delta)
        .with_buttons(buttons)
        .with_modifiers(point.modifiers))
    }

    /// Translates one native wheel transition within the current neutral pointer stream.
    ///
    /// # Errors
    ///
    /// Returns [`MouseIngressDiagnostic`] when the incoming device conflicts with the active
    /// stream or pointer identity is exhausted.
    pub fn wheel(
        &mut self,
        device_id: InputDeviceId,
        point: TranslatedPointerPoint,
        scroll_delta: LogicalDelta,
    ) -> Result<PointerEvent, MouseIngressDiagnostic> {
        self.validate_active_device(device_id)?;
        let pointer_id = self.ensure_active_stream(device_id, &point)?;
        let buttons = self.effective_buttons(device_id);
        self.update_active_stream(&point);
        Ok(PointerEvent::new(
            pointer_id,
            PointerDeviceKind::Mouse,
            PointerPhase::Wheel,
            point.position,
            point.input_context,
        )
        .with_device_id(device_id)
        .with_scroll_delta(scroll_delta)
        .with_buttons(buttons)
        .with_modifiers(point.modifiers))
    }

    /// Translates one native mouse-button transition while preserving stream lifetime semantics.
    ///
    /// # Errors
    ///
    /// Returns [`MouseIngressDiagnostic`] when an otherwise representable transition conflicts
    /// with active-device or stream authority, or when pointer identity is exhausted. Benign
    /// duplicate, unmatched, or deliberately suppressed transitions are returned as
    /// [`MouseButtonOutcome::Suppressed`] instead.
    pub fn button_input(
        &mut self,
        device_id: InputDeviceId,
        state: ElementState,
        button: MouseButton,
        point: Option<TranslatedPointerPoint>,
    ) -> Result<MouseButtonOutcome, MouseIngressDiagnostic> {
        match state {
            ElementState::Pressed => self.button_pressed(device_id, button, point),
            ElementState::Released => self.button_released(device_id, button, point),
        }
    }

    pub fn invalidate_point_authority(&mut self, modifiers: KeyModifiers) -> Option<PointerEvent> {
        self.last_native_position = None;
        let buttons = self
            .active_stream
            .as_ref()
            .map_or_else(PointerButtons::default, |stream| {
                self.effective_buttons(stream.device_id)
            });
        self.suppressed_buttons
            .extend(self.pressed_buttons.iter().copied());
        self.take_active_cancel(buttons, modifiers)
    }

    pub fn cancel_for_device_change(&mut self, modifiers: KeyModifiers) -> Option<PointerEvent> {
        let device_id = self.active_device_id()?;
        let buttons = self.effective_buttons(device_id);
        self.suppress_pressed_buttons_for_device(device_id);
        self.take_active_cancel(buttons, modifiers)
    }

    fn button_pressed(
        &mut self,
        device_id: InputDeviceId,
        button: MouseButton,
        point: Option<TranslatedPointerPoint>,
    ) -> Result<MouseButtonOutcome, MouseIngressDiagnostic> {
        self.validate_active_device(device_id)?;
        let device_button = (device_id, button);
        if !self.pressed_buttons.insert(device_button) {
            return Ok(MouseButtonOutcome::Suppressed(
                MouseIngressDiagnostic::DuplicatePress(button),
            ));
        }

        let Some(point) = point else {
            self.suppressed_buttons.insert(device_button);
            return Ok(MouseButtonOutcome::Suppressed(
                MouseIngressDiagnostic::PointUnavailableAtPress(button),
            ));
        };

        let pointer_id = self.ensure_active_stream(device_id, &point)?;
        let buttons = self.effective_buttons(device_id);
        self.update_active_stream(&point);
        let event = PointerEvent::new(
            pointer_id,
            PointerDeviceKind::Mouse,
            PointerPhase::Down,
            point.position,
            point.input_context,
        )
        .with_device_id(device_id)
        .with_buttons(buttons)
        .with_changed_button(translate_mouse_button(button))
        .with_modifiers(point.modifiers);
        Ok(MouseButtonOutcome::Submit(event))
    }

    fn button_released(
        &mut self,
        device_id: InputDeviceId,
        button: MouseButton,
        point: Option<TranslatedPointerPoint>,
    ) -> Result<MouseButtonOutcome, MouseIngressDiagnostic> {
        let device_button = (device_id, button);
        if !self.pressed_buttons.contains(&device_button) {
            return Ok(MouseButtonOutcome::Suppressed(
                MouseIngressDiagnostic::UnmatchedRelease(button),
            ));
        }
        if self.suppressed_buttons.remove(&device_button) {
            self.pressed_buttons.remove(&device_button);
            return Ok(MouseButtonOutcome::Suppressed(
                MouseIngressDiagnostic::SuppressedRelease(button),
            ));
        }
        self.validate_active_device(device_id)?;

        let Some(point) = point else {
            self.pressed_buttons.remove(&device_button);
            self.suppress_pressed_buttons_for_device(device_id);
            return Ok(MouseButtonOutcome::Suppressed(
                MouseIngressDiagnostic::MissingActiveStream(button),
            ));
        };
        let Some(stream) = self.active_stream.as_ref() else {
            self.pressed_buttons.remove(&device_button);
            self.suppress_pressed_buttons_for_device(device_id);
            return Ok(MouseButtonOutcome::Suppressed(
                MouseIngressDiagnostic::MissingActiveStream(button),
            ));
        };
        let pointer_id = stream.pointer_id;
        let stream_device_id = stream.device_id;

        self.pressed_buttons.remove(&device_button);
        let buttons = self.effective_buttons(device_id);
        if buttons.is_empty() {
            self.active_stream = None;
        } else {
            self.update_active_stream(&point);
        }

        let event = PointerEvent::new(
            pointer_id,
            PointerDeviceKind::Mouse,
            PointerPhase::Up,
            point.position,
            point.input_context,
        )
        .with_device_id(stream_device_id)
        .with_buttons(buttons)
        .with_changed_button(translate_mouse_button(button))
        .with_modifiers(point.modifiers);
        Ok(MouseButtonOutcome::Submit(event))
    }

    fn validate_active_device(
        &self,
        incoming: InputDeviceId,
    ) -> Result<(), MouseIngressDiagnostic> {
        let Some(active) = self.active_device_id() else {
            return Ok(());
        };
        if active == incoming {
            Ok(())
        } else {
            Err(MouseIngressDiagnostic::DeviceMismatch { active, incoming })
        }
    }

    fn ensure_active_stream(
        &mut self,
        device_id: InputDeviceId,
        point: &TranslatedPointerPoint,
    ) -> Result<PointerId, MouseIngressDiagnostic> {
        if let Some(stream) = self.active_stream.as_ref() {
            if stream.device_id != device_id {
                return Err(MouseIngressDiagnostic::DeviceMismatch {
                    active: stream.device_id,
                    incoming: device_id,
                });
            }
            return Ok(stream.pointer_id);
        }

        let value = self
            .next_pointer_id
            .take()
            .ok_or(MouseIngressDiagnostic::PointerIdentityExhausted)?;
        let pointer_id =
            PointerId::new(value).ok_or(MouseIngressDiagnostic::PointerIdentityExhausted)?;
        self.next_pointer_id = value.checked_add(1);
        self.active_stream = Some(MousePointerStream {
            pointer_id,
            device_id,
            position: point.position,
            input_context: point.input_context.clone(),
        });
        Ok(pointer_id)
    }

    fn update_active_stream(&mut self, point: &TranslatedPointerPoint) {
        let Some(stream) = self.active_stream.as_mut() else {
            return;
        };
        stream.position = point.position;
        stream.input_context = point.input_context.clone();
    }

    fn suppress_pressed_buttons_for_device(&mut self, device_id: InputDeviceId) {
        self.suppressed_buttons.extend(
            self.pressed_buttons
                .iter()
                .copied()
                .filter(|(pressed_device, _)| *pressed_device == device_id),
        );
    }

    fn effective_buttons(&self, device_id: InputDeviceId) -> PointerButtons {
        PointerButtons::new(
            self.pressed_buttons
                .iter()
                .filter(|device_button| {
                    device_button.0 == device_id && !self.suppressed_buttons.contains(device_button)
                })
                .map(|(_, button)| translate_mouse_button(*button)),
        )
    }

    fn take_active_cancel(
        &mut self,
        buttons: PointerButtons,
        modifiers: KeyModifiers,
    ) -> Option<PointerEvent> {
        let stream = self.active_stream.take()?;
        Some(
            PointerEvent::new(
                stream.pointer_id,
                PointerDeviceKind::Mouse,
                PointerPhase::Cancel,
                stream.position,
                stream.input_context,
            )
            .with_device_id(stream.device_id)
            .with_buttons(buttons)
            .with_modifiers(modifiers),
        )
    }
}

fn logical_movement_delta(
    previous: Option<LogicalPoint>,
    current: LogicalPoint,
) -> Result<LogicalDelta, MouseIngressDiagnostic> {
    let Some(previous) = previous else {
        return Ok(LogicalDelta::ZERO);
    };
    LogicalDelta::new(current.x() - previous.x(), current.y() - previous.y())
        .map_err(|_| MouseIngressDiagnostic::MovementDeltaOutOfRange)
}

#[must_use]
pub const fn translate_mouse_button(button: MouseButton) -> PointerButton {
    match button {
        MouseButton::Left => PointerButton::Primary,
        MouseButton::Right => PointerButton::Secondary,
        MouseButton::Middle => PointerButton::Middle,
        MouseButton::Back => PointerButton::Other(BACK_BUTTON_ID),
        MouseButton::Forward => PointerButton::Other(FORWARD_BUTTON_ID),
        MouseButton::Other(button) => PointerButton::Other(button),
    }
}

#[cfg(test)]
mod tests;
