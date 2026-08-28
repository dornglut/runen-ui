use runenui_core::{
    InputDeviceId, KeyLocation, KeyModifiers, KeyboardCompositionState, KeyboardEvent,
    KeyboardPhase, LogicalKey, PhysicalKey,
};
use winit::{
    event::{ElementState, KeyEvent},
    keyboard::{
        Key as WinitKey, KeyCode, KeyLocation as WinitKeyLocation, NamedKey,
        PhysicalKey as WinitPhysicalKey,
    },
};

#[derive(Clone, Debug, Eq, PartialEq)]
struct PressedKey {
    device_id: InputDeviceId,
    physical_key: PhysicalKey,
    logical_key: LogicalKey,
    location: KeyLocation,
}

pub struct NativeKeyTransition<'a> {
    state: ElementState,
    physical_key: WinitPhysicalKey,
    logical_key: &'a WinitKey,
    repeat: bool,
    location: WinitKeyLocation,
    synthetic: bool,
}

impl<'a> NativeKeyTransition<'a> {
    #[must_use]
    pub const fn from_event(event: &'a KeyEvent, synthetic: bool) -> Self {
        Self {
            state: event.state,
            physical_key: event.physical_key,
            logical_key: &event.logical_key,
            repeat: event.repeat,
            location: event.location,
            synthetic,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum KeyboardIngressDiagnostic {
    SyntheticPress,
    RepeatWithoutPress,
    DuplicatePress,
    ReleaseWithoutPress,
    NoFocusedRuntimeTarget,
}

pub enum KeyboardInputOutcome {
    Submit(KeyboardEvent),
    Suppressed(KeyboardIngressDiagnostic),
}

#[derive(Default)]
pub struct KeyboardInputState {
    pressed: Vec<PressedKey>,
}

impl KeyboardInputState {
    pub fn key_input(
        &mut self,
        device_id: InputDeviceId,
        transition: &NativeKeyTransition<'_>,
        modifiers: KeyModifiers,
        composition: KeyboardCompositionState,
    ) -> KeyboardInputOutcome {
        let physical_key = translate_physical_key(transition.physical_key);
        let logical_key = translate_logical_key(transition.logical_key);
        let location = translate_key_location(transition.location);
        let existing = self.pressed.iter().position(|pressed| {
            pressed.device_id == device_id && pressed.physical_key == physical_key
        });

        match transition.state {
            ElementState::Pressed if transition.synthetic => {
                KeyboardInputOutcome::Suppressed(KeyboardIngressDiagnostic::SyntheticPress)
            }
            ElementState::Pressed if transition.repeat => {
                let Some(index) = existing else {
                    return KeyboardInputOutcome::Suppressed(
                        KeyboardIngressDiagnostic::RepeatWithoutPress,
                    );
                };
                self.pressed[index].logical_key = logical_key.clone();
                self.pressed[index].location = location;
                KeyboardInputOutcome::Submit(KeyboardEvent::new(
                    KeyboardPhase::Down,
                    physical_key,
                    logical_key,
                    modifiers,
                    true,
                    location,
                    composition,
                    Some(device_id),
                ))
            }
            ElementState::Pressed => {
                if existing.is_some() {
                    return KeyboardInputOutcome::Suppressed(
                        KeyboardIngressDiagnostic::DuplicatePress,
                    );
                }
                self.pressed.push(PressedKey {
                    device_id,
                    physical_key: physical_key.clone(),
                    logical_key: logical_key.clone(),
                    location,
                });
                KeyboardInputOutcome::Submit(KeyboardEvent::new(
                    KeyboardPhase::Down,
                    physical_key,
                    logical_key,
                    modifiers,
                    false,
                    location,
                    composition,
                    Some(device_id),
                ))
            }
            ElementState::Released => {
                let Some(index) = existing else {
                    return KeyboardInputOutcome::Suppressed(
                        KeyboardIngressDiagnostic::ReleaseWithoutPress,
                    );
                };
                let pressed = self.pressed.remove(index);
                if transition.synthetic {
                    KeyboardInputOutcome::Submit(KeyboardEvent::new(
                        KeyboardPhase::Cancel,
                        pressed.physical_key,
                        pressed.logical_key,
                        modifiers,
                        false,
                        pressed.location,
                        composition,
                        Some(device_id),
                    ))
                } else {
                    KeyboardInputOutcome::Submit(KeyboardEvent::new(
                        KeyboardPhase::Up,
                        physical_key,
                        logical_key,
                        modifiers,
                        transition.repeat,
                        location,
                        composition,
                        Some(device_id),
                    ))
                }
            }
        }
    }

    pub fn cancel_all(
        &mut self,
        modifiers: KeyModifiers,
        composition: KeyboardCompositionState,
    ) -> Vec<KeyboardEvent> {
        self.pressed
            .drain(..)
            .map(|pressed| {
                KeyboardEvent::new(
                    KeyboardPhase::Cancel,
                    pressed.physical_key,
                    pressed.logical_key,
                    modifiers,
                    false,
                    pressed.location,
                    composition,
                    Some(pressed.device_id),
                )
            })
            .collect()
    }

    #[cfg(test)]
    const fn pressed_len(&self) -> usize {
        self.pressed.len()
    }
}

fn translate_physical_key(key: WinitPhysicalKey) -> PhysicalKey {
    match key {
        WinitPhysicalKey::Code(KeyCode::Enter) => PhysicalKey::Enter,
        WinitPhysicalKey::Code(KeyCode::Space) => PhysicalKey::Space,
        WinitPhysicalKey::Code(KeyCode::Tab) => PhysicalKey::Tab,
        WinitPhysicalKey::Code(KeyCode::Escape) => PhysicalKey::Escape,
        WinitPhysicalKey::Code(KeyCode::ArrowLeft) => PhysicalKey::ArrowLeft,
        WinitPhysicalKey::Code(KeyCode::ArrowRight) => PhysicalKey::ArrowRight,
        WinitPhysicalKey::Code(KeyCode::ArrowUp) => PhysicalKey::ArrowUp,
        WinitPhysicalKey::Code(KeyCode::ArrowDown) => PhysicalKey::ArrowDown,
        WinitPhysicalKey::Code(code) => PhysicalKey::Code(format!("{code:?}")),
        WinitPhysicalKey::Unidentified(native) => {
            PhysicalKey::Code(format!("Unidentified:{native:?}"))
        }
    }
}

fn translate_logical_key(key: &WinitKey) -> LogicalKey {
    match key {
        WinitKey::Named(NamedKey::Enter) => LogicalKey::Enter,
        WinitKey::Named(NamedKey::Space) => LogicalKey::Space,
        WinitKey::Named(NamedKey::Tab) => LogicalKey::Tab,
        WinitKey::Named(NamedKey::Escape) => LogicalKey::Escape,
        WinitKey::Named(NamedKey::ArrowLeft) => LogicalKey::ArrowLeft,
        WinitKey::Named(NamedKey::ArrowRight) => LogicalKey::ArrowRight,
        WinitKey::Named(NamedKey::ArrowUp) => LogicalKey::ArrowUp,
        WinitKey::Named(NamedKey::ArrowDown) => LogicalKey::ArrowDown,
        WinitKey::Character(text) => LogicalKey::Character(text.to_string()),
        WinitKey::Named(named) => LogicalKey::Named(format!("{named:?}")),
        WinitKey::Unidentified(native) => LogicalKey::Named(format!("Unidentified:{native:?}")),
        WinitKey::Dead(Some(character)) => LogicalKey::Named(format!("Dead({character:?})")),
        WinitKey::Dead(None) => LogicalKey::Named(String::from("Dead")),
    }
}

const fn translate_key_location(location: WinitKeyLocation) -> KeyLocation {
    match location {
        WinitKeyLocation::Standard => KeyLocation::Standard,
        WinitKeyLocation::Left => KeyLocation::Left,
        WinitKeyLocation::Right => KeyLocation::Right,
        WinitKeyLocation::Numpad => KeyLocation::Numpad,
    }
}

#[cfg(test)]
mod tests {
    use super::{
        KeyboardIngressDiagnostic, KeyboardInputOutcome, KeyboardInputState, NativeKeyTransition,
        translate_key_location, translate_logical_key, translate_physical_key,
    };
    use runenui_core::{
        InputDeviceId, KeyModifiers, KeyboardCompositionState, KeyboardPhase,
        LogicalKey as NeutralLogicalKey, PhysicalKey as NeutralPhysicalKey,
    };
    use winit::{
        event::ElementState,
        keyboard::{
            Key as WinitKey, KeyCode, KeyLocation, NamedKey, PhysicalKey as WinitPhysicalKey,
        },
    };

    fn device(value: u64) -> InputDeviceId {
        InputDeviceId::new(value)
            .unwrap_or_else(|| unreachable!("fixture device identity is non-zero"))
    }

    fn transition(
        state: ElementState,
        physical_key: WinitPhysicalKey,
        logical_key: &WinitKey,
        repeat: bool,
        synthetic: bool,
    ) -> NativeKeyTransition<'_> {
        NativeKeyTransition {
            state,
            physical_key,
            logical_key,
            repeat,
            location: KeyLocation::Standard,
            synthetic,
        }
    }

    fn submitted(outcome: KeyboardInputOutcome) -> runenui_core::KeyboardEvent {
        match outcome {
            KeyboardInputOutcome::Submit(event) => event,
            KeyboardInputOutcome::Suppressed(diagnostic) => {
                unreachable!("fixture transition must submit: {diagnostic:?}")
            }
        }
    }

    #[test]
    fn native_key_identity_maps_specials_and_preserves_named_fallbacks() {
        assert_eq!(
            translate_physical_key(WinitPhysicalKey::Code(KeyCode::Enter)),
            NeutralPhysicalKey::Enter
        );
        assert_eq!(
            translate_physical_key(WinitPhysicalKey::Code(KeyCode::KeyQ)),
            NeutralPhysicalKey::Code(String::from("KeyQ"))
        );
        assert_eq!(
            translate_logical_key(&WinitKey::Character("ß".into())),
            NeutralLogicalKey::Character(String::from("ß"))
        );
        assert_eq!(
            translate_logical_key(&WinitKey::Named(NamedKey::F5)),
            NeutralLogicalKey::Named(String::from("F5"))
        );
    }

    #[test]
    fn native_key_location_preserves_all_neutral_classes() {
        assert_eq!(
            translate_key_location(KeyLocation::Standard),
            runenui_core::KeyLocation::Standard
        );
        assert_eq!(
            translate_key_location(KeyLocation::Left),
            runenui_core::KeyLocation::Left
        );
        assert_eq!(
            translate_key_location(KeyLocation::Right),
            runenui_core::KeyLocation::Right
        );
        assert_eq!(
            translate_key_location(KeyLocation::Numpad),
            runenui_core::KeyLocation::Numpad
        );
    }

    #[test]
    fn real_down_repeat_and_up_share_one_native_key_lifetime() {
        let mut keyboard = KeyboardInputState::default();
        let device = device(1);
        let modifiers = KeyModifiers::SHIFT.with_control();
        let logical = WinitKey::Named(NamedKey::Space);
        let physical = WinitPhysicalKey::Code(KeyCode::Space);

        let down = submitted(keyboard.key_input(
            device,
            &transition(ElementState::Pressed, physical, &logical, false, false),
            modifiers,
            KeyboardCompositionState::Inactive,
        ));
        assert_eq!(down.phase(), KeyboardPhase::Down);
        assert!(!down.is_repeat());
        assert_eq!(down.device_id(), Some(device));
        assert_eq!(down.modifiers(), modifiers);
        assert_eq!(keyboard.pressed_len(), 1);

        let repeat = submitted(keyboard.key_input(
            device,
            &transition(ElementState::Pressed, physical, &logical, true, false),
            modifiers,
            KeyboardCompositionState::Inactive,
        ));
        assert_eq!(repeat.phase(), KeyboardPhase::Down);
        assert!(repeat.is_repeat());
        assert_eq!(keyboard.pressed_len(), 1);

        let up = submitted(keyboard.key_input(
            device,
            &transition(ElementState::Released, physical, &logical, false, false),
            modifiers,
            KeyboardCompositionState::Inactive,
        ));
        assert_eq!(up.phase(), KeyboardPhase::Up);
        assert_eq!(keyboard.pressed_len(), 0);
    }

    #[test]
    fn synthetic_release_cancels_and_late_release_is_suppressed() {
        let mut keyboard = KeyboardInputState::default();
        let device = device(2);
        let logical = WinitKey::Named(NamedKey::Space);
        let physical = WinitPhysicalKey::Code(KeyCode::Space);
        let _down = submitted(keyboard.key_input(
            device,
            &transition(ElementState::Pressed, physical, &logical, false, false),
            KeyModifiers::NONE,
            KeyboardCompositionState::Inactive,
        ));

        let cancel = submitted(keyboard.key_input(
            device,
            &transition(ElementState::Released, physical, &logical, false, true),
            KeyModifiers::ALT,
            KeyboardCompositionState::Inactive,
        ));
        assert_eq!(cancel.phase(), KeyboardPhase::Cancel);
        assert_eq!(cancel.modifiers(), KeyModifiers::ALT);
        assert_eq!(keyboard.pressed_len(), 0);

        assert!(matches!(
            keyboard.key_input(
                device,
                &transition(ElementState::Released, physical, &logical, false, false),
                KeyModifiers::NONE,
                KeyboardCompositionState::Inactive,
            ),
            KeyboardInputOutcome::Suppressed(KeyboardIngressDiagnostic::ReleaseWithoutPress)
        ));
    }

    #[test]
    fn synthetic_press_and_orphan_repeat_never_open_a_key_lifetime() {
        let mut keyboard = KeyboardInputState::default();
        let device = device(3);
        let logical = WinitKey::Character("a".into());
        let physical = WinitPhysicalKey::Code(KeyCode::KeyA);
        assert!(matches!(
            keyboard.key_input(
                device,
                &transition(ElementState::Pressed, physical, &logical, false, true),
                KeyModifiers::NONE,
                KeyboardCompositionState::Inactive,
            ),
            KeyboardInputOutcome::Suppressed(KeyboardIngressDiagnostic::SyntheticPress)
        ));
        assert!(matches!(
            keyboard.key_input(
                device,
                &transition(ElementState::Pressed, physical, &logical, true, false),
                KeyModifiers::NONE,
                KeyboardCompositionState::Inactive,
            ),
            KeyboardInputOutcome::Suppressed(KeyboardIngressDiagnostic::RepeatWithoutPress)
        ));
        assert_eq!(keyboard.pressed_len(), 0);
    }

    #[test]
    fn authority_loss_cancels_pressed_keys_in_admission_order() {
        let mut keyboard = KeyboardInputState::default();
        let device_a = device(4);
        let device_b = device(5);
        let logical_a = WinitKey::Character("a".into());
        let logical_enter = WinitKey::Named(NamedKey::Enter);
        for (device, physical, logical) in [
            (device_a, WinitPhysicalKey::Code(KeyCode::KeyA), &logical_a),
            (
                device_b,
                WinitPhysicalKey::Code(KeyCode::Enter),
                &logical_enter,
            ),
        ] {
            let _event = submitted(keyboard.key_input(
                device,
                &transition(ElementState::Pressed, physical, logical, false, false),
                KeyModifiers::NONE,
                KeyboardCompositionState::Inactive,
            ));
        }

        let cancelled = keyboard.cancel_all(KeyModifiers::META, KeyboardCompositionState::Inactive);
        assert_eq!(cancelled.len(), 2);
        assert_eq!(cancelled[0].device_id(), Some(device_a));
        assert_eq!(cancelled[1].device_id(), Some(device_b));
        assert!(
            cancelled
                .iter()
                .all(|event| event.phase() == KeyboardPhase::Cancel)
        );
        assert!(
            cancelled
                .iter()
                .all(|event| event.modifiers() == KeyModifiers::META)
        );
        assert_eq!(keyboard.pressed_len(), 0);
    }
}
