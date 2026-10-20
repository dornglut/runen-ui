use std::sync::atomic::{AtomicUsize, Ordering};

use runenui_core::{
    InputDeviceId, KeyLocation, KeyModifiers, KeyboardCompositionState, KeyboardEvent,
    KeyboardPhase, LogicalKey, PhysicalKey, SemanticCommand,
};
use winit::{
    event::{ElementState, KeyEvent},
    keyboard::{
        Key as WinitKey, KeyCode, KeyLocation as WinitKeyLocation, NamedKey,
        PhysicalKey as WinitPhysicalKey,
    },
};

// Process-local adapter-edge identity, not an M13 runtime scope or sequence.
// Rebinding/replacement must not reuse pending acknowledgment identities.
static NEXT_KEYBOARD_ADAPTER_ID: AtomicUsize = AtomicUsize::new(1);

/// One original physical Down translation, not an M13 `WorkSequence` or a
/// runtime input settlement. The translator never mints gameplay permission.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct KeyboardDownAdmission {
    adapter_id: usize,
    down_generation: u64,
}

/// The embedding host must derive this from the actual runtime submission:
/// queued work is not committed processing, and only a definitive pre-queue
/// rejection may be classified as never admitted.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum KeyboardDownDecision {
    Queued,
    RejectedBeforeQueue,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum KeyboardDownAdmissionError {
    StaleOrAlreadyResolved,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum DownAdmission {
    Pending(KeyboardDownAdmission),
    Queued,
    RejectedBeforeQueue,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct PressedKey {
    device_id: InputDeviceId,
    physical_key: PhysicalKey,
    logical_key: LogicalKey,
    location: KeyLocation,
    admission: DownAdmission,
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
    RepeatOfRejectedDown,
    RepeatWhileAdmissionPending,
    ReleaseOfRejectedDown,
}

/// An unrepresentable native keyboard ingress failure.
///
/// Unlike ordinary suppressed OS events, this MUST terminate the embedding
/// host's input authority; it is not a UI nonclaim or gameplay permission.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum KeyboardIngressFailure {
    AdmissionIdentityExhausted,
    /// A terminal key transition arrived before the original Down's runtime
    /// queue-admission outcome was acknowledged. No terminal UI event is safe.
    UnresolvedOriginalDownAdmission,
}

/// Original Down carries a one-use adapter-local acknowledgment identity.
/// Its existence never certifies a runtime receipt, M13 finality or game right.
pub enum KeyboardInputOutcome {
    Submit {
        event: KeyboardEvent,
        down_admission: Option<KeyboardDownAdmission>,
    },
    Suppressed(KeyboardIngressDiagnostic),
    Failed(KeyboardIngressFailure),
}

#[derive(Default)]
pub struct KeyboardInputState {
    pressed: Vec<PressedKey>,
    next_down_admission: u64,
    adapter_id: Option<usize>,
}

impl KeyboardInputState {
    /// Resolve only the original pending translated Down. Hosts MUST mark
    /// `RejectedBeforeQueue` only after proving that this exact original Down
    /// received a definitive pre-sequence rejection (such as `NoFocusedTarget`)
    /// in the same live runtime scope. Capacity, terminal, pending receipts,
    /// missing settlement and a rejected derived Cancel do not qualify.
    ///
    /// A queued Down remains subject to normal M13 finality and exact-scope
    /// cancellation obligations; this adapter is not their authority.
    ///
    /// # Errors
    ///
    /// Returns `KeyboardDownAdmissionError::StaleOrAlreadyResolved` if the
    /// exact original Down is no longer pending or the token was already used.
    pub fn resolve_down_admission(
        &mut self,
        admission: KeyboardDownAdmission,
        decision: KeyboardDownDecision,
    ) -> Result<(), KeyboardDownAdmissionError> {
        let Some(pressed) = self
            .pressed
            .iter_mut()
            .find(|pressed| pressed.admission == DownAdmission::Pending(admission))
        else {
            return Err(KeyboardDownAdmissionError::StaleOrAlreadyResolved);
        };
        pressed.admission = match decision {
            KeyboardDownDecision::Queued => DownAdmission::Queued,
            KeyboardDownDecision::RejectedBeforeQueue => DownAdmission::RejectedBeforeQueue,
        };
        Ok(())
    }

    // A single original transition machine owns Down/repeat/Up ordering.
    // Splitting it for a line-count threshold would scatter the pressed-key
    // invariant and complicate the explicit admission-state change.
    #[allow(clippy::too_many_lines)]
    pub fn key_input(
        &mut self,
        device_id: InputDeviceId,
        transition: &NativeKeyTransition<'_>,
        modifiers: KeyModifiers,
        composition: KeyboardCompositionState,
    ) -> KeyboardInputOutcome {
        let physical_key = translate_physical_key(transition.physical_key);
        let logical_key = translate_logical_key(
            transition.logical_key,
            transition.physical_key,
            modifiers,
            composition,
        );
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
                match self.pressed[index].admission {
                    DownAdmission::RejectedBeforeQueue => {
                        // A later focus transfer cannot bootstrap a new UI
                        // Down from a repeat of this same physical lifetime.
                        return KeyboardInputOutcome::Suppressed(
                            KeyboardIngressDiagnostic::RepeatOfRejectedDown,
                        );
                    }
                    DownAdmission::Pending(_) => {
                        // The caller has not established even queue admission.
                        // No follow-on UI repeat may outrun that decision.
                        return KeyboardInputOutcome::Suppressed(
                            KeyboardIngressDiagnostic::RepeatWhileAdmissionPending,
                        );
                    }
                    DownAdmission::Queued => {}
                }
                self.pressed[index].logical_key = logical_key.clone();
                self.pressed[index].location = location;
                KeyboardInputOutcome::Submit {
                    event: KeyboardEvent::new(
                        KeyboardPhase::Down,
                        physical_key,
                        logical_key,
                        modifiers,
                        true,
                        location,
                        composition,
                        Some(device_id),
                    ),
                    down_admission: None,
                }
            }
            ElementState::Pressed => {
                if existing.is_some() {
                    return KeyboardInputOutcome::Suppressed(
                        KeyboardIngressDiagnostic::DuplicatePress,
                    );
                }
                let Some(next) = self.next_down_admission.checked_add(1) else {
                    // Never alias an old acknowledgment to a new lifetime.
                    return KeyboardInputOutcome::Failed(
                        KeyboardIngressFailure::AdmissionIdentityExhausted,
                    );
                };
                let adapter_id = if let Some(id) = self.adapter_id {
                    id
                } else {
                    let Ok(id) = NEXT_KEYBOARD_ADAPTER_ID.try_update(
                        Ordering::Relaxed,
                        Ordering::Relaxed,
                        |current| current.checked_add(1),
                    ) else {
                        return KeyboardInputOutcome::Failed(
                            KeyboardIngressFailure::AdmissionIdentityExhausted,
                        );
                    };
                    self.adapter_id = Some(id);
                    id
                };
                self.next_down_admission = next;
                let admission = KeyboardDownAdmission {
                    adapter_id,
                    down_generation: next,
                };
                self.pressed.push(PressedKey {
                    device_id,
                    physical_key: physical_key.clone(),
                    logical_key: logical_key.clone(),
                    location,
                    admission: DownAdmission::Pending(admission),
                });
                KeyboardInputOutcome::Submit {
                    event: KeyboardEvent::new(
                        KeyboardPhase::Down,
                        physical_key,
                        logical_key,
                        modifiers,
                        false,
                        location,
                        composition,
                        Some(device_id),
                    ),
                    down_admission: Some(admission),
                }
            }
            ElementState::Released => {
                let Some(index) = existing else {
                    return KeyboardInputOutcome::Suppressed(
                        KeyboardIngressDiagnostic::ReleaseWithoutPress,
                    );
                };
                // An unresolved original Down may never have reached M13. Do not
                // erase its provenance or publish a terminal Up/Cancel that could
                // be routed to an unrelated focused widget. The host must fail
                // closed; an authentic admission decision can still resolve it.
                if matches!(self.pressed[index].admission, DownAdmission::Pending(_)) {
                    return KeyboardInputOutcome::Failed(
                        KeyboardIngressFailure::UnresolvedOriginalDownAdmission,
                    );
                }
                let pressed = self.pressed.remove(index);
                if pressed.admission == DownAdmission::RejectedBeforeQueue {
                    return KeyboardInputOutcome::Suppressed(
                        KeyboardIngressDiagnostic::ReleaseOfRejectedDown,
                    );
                }
                KeyboardInputOutcome::Submit {
                    event: if transition.synthetic {
                        KeyboardEvent::new(
                            KeyboardPhase::Cancel,
                            pressed.physical_key,
                            pressed.logical_key,
                            modifiers,
                            false,
                            pressed.location,
                            composition,
                            Some(device_id),
                        )
                    } else {
                        KeyboardEvent::new(
                            KeyboardPhase::Up,
                            physical_key,
                            logical_key,
                            modifiers,
                            transition.repeat,
                            location,
                            composition,
                            Some(device_id),
                        )
                    },
                    down_admission: None,
                }
            }
        }
    }

    /// Atomically cancel source keys only after their original Down admissions
    /// are resolved. Unknown queue admission must never fabricate a Cancel.
    ///
    /// # Errors
    ///
    /// Returns `UnresolvedOriginalDownAdmission` without removing any key or
    /// emitting a Cancel if this device still has a pending original Down.
    pub fn cancel_device(
        &mut self,
        device_id: InputDeviceId,
        modifiers: KeyModifiers,
        composition: KeyboardCompositionState,
    ) -> Result<Vec<KeyboardEvent>, KeyboardIngressFailure> {
        if self.pressed.iter().any(|pressed| {
            pressed.device_id == device_id && matches!(pressed.admission, DownAdmission::Pending(_))
        }) {
            return Err(KeyboardIngressFailure::UnresolvedOriginalDownAdmission);
        }
        let mut cancelled = Vec::new();
        self.pressed.retain(|pressed| {
            if pressed.device_id != device_id {
                return true;
            }
            if pressed.admission != DownAdmission::RejectedBeforeQueue {
                cancelled.push(cancelled_key(pressed.clone(), modifiers, composition));
            }
            false
        });
        Ok(cancelled)
    }

    /// Atomically cancel all keys after checking every original admission.
    ///
    /// # Errors
    ///
    /// Returns `UnresolvedOriginalDownAdmission` without removing any key or
    /// emitting a Cancel if any original Down is still pending.
    pub fn cancel_all(
        &mut self,
        modifiers: KeyModifiers,
        composition: KeyboardCompositionState,
    ) -> Result<Vec<KeyboardEvent>, KeyboardIngressFailure> {
        if self
            .pressed
            .iter()
            .any(|pressed| matches!(pressed.admission, DownAdmission::Pending(_)))
        {
            return Err(KeyboardIngressFailure::UnresolvedOriginalDownAdmission);
        }
        Ok(self
            .pressed
            .drain(..)
            .filter(|pressed| pressed.admission != DownAdmission::RejectedBeforeQueue)
            .map(|pressed| cancelled_key(pressed, modifiers, composition))
            .collect())
    }

    #[cfg(test)]
    const fn pressed_len(&self) -> usize {
        self.pressed.len()
    }
}

fn cancelled_key(
    pressed: PressedKey,
    modifiers: KeyModifiers,
    composition: KeyboardCompositionState,
) -> KeyboardEvent {
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
        WinitPhysicalKey::Code(KeyCode::Home) => PhysicalKey::Home,
        WinitPhysicalKey::Code(KeyCode::End) => PhysicalKey::End,
        WinitPhysicalKey::Code(KeyCode::PageUp) => PhysicalKey::PageUp,
        WinitPhysicalKey::Code(KeyCode::PageDown) => PhysicalKey::PageDown,
        WinitPhysicalKey::Code(code) => PhysicalKey::Code(format!("{code:?}")),
        WinitPhysicalKey::Unidentified(native) => {
            PhysicalKey::Code(format!("Unidentified:{native:?}"))
        }
    }
}

fn translate_logical_key(
    key: &WinitKey,
    _physical_key: WinitPhysicalKey,
    modifiers: KeyModifiers,
    composition: KeyboardCompositionState,
) -> LogicalKey {
    match key {
        WinitKey::Named(NamedKey::Enter) => LogicalKey::Enter,
        WinitKey::Named(NamedKey::Space) => LogicalKey::Space,
        WinitKey::Named(NamedKey::Tab) => LogicalKey::Tab,
        WinitKey::Named(NamedKey::Escape) => LogicalKey::Escape,
        WinitKey::Named(NamedKey::Backspace) => LogicalKey::Backspace,
        WinitKey::Named(NamedKey::Delete) => LogicalKey::Delete,
        WinitKey::Named(NamedKey::ArrowLeft) => LogicalKey::ArrowLeft,
        WinitKey::Named(NamedKey::ArrowRight) => LogicalKey::ArrowRight,
        WinitKey::Named(NamedKey::ArrowUp) => LogicalKey::ArrowUp,
        WinitKey::Named(NamedKey::ArrowDown) => LogicalKey::ArrowDown,
        WinitKey::Named(NamedKey::Home) => LogicalKey::Home,
        WinitKey::Named(NamedKey::End) => LogicalKey::End,
        WinitKey::Named(NamedKey::PageUp) => LogicalKey::PageUp,
        WinitKey::Named(NamedKey::PageDown) => LogicalKey::PageDown,
        WinitKey::Character(text) => native_shortcut_command(text, modifiers, composition)
            .map_or_else(
                || LogicalKey::Character(text.to_string()),
                LogicalKey::Command,
            ),
        WinitKey::Named(named) => LogicalKey::Named(format!("{named:?}")),
        WinitKey::Unidentified(native) => LogicalKey::Named(format!("Unidentified:{native:?}")),
        WinitKey::Dead(Some(character)) => LogicalKey::Named(format!("Dead({character:?})")),
        WinitKey::Dead(None) => LogicalKey::Named(String::from("Dead")),
    }
}

fn native_shortcut_command(
    logical_text: &str,
    modifiers: KeyModifiers,
    composition: KeyboardCompositionState,
) -> Option<SemanticCommand> {
    if composition != KeyboardCompositionState::Inactive || modifiers.alt() {
        return None;
    }
    let primary = if cfg!(target_os = "macos") {
        modifiers.meta() && !modifiers.control()
    } else {
        modifiers.control() && !modifiers.meta()
    };
    if !primary {
        return None;
    }

    let mut characters = logical_text.chars();
    let key = characters.next()?;
    if characters.next().is_some() || !key.is_ascii() {
        return None;
    }
    let key = key.to_ascii_lowercase();
    match (key, modifiers.shift()) {
        ('a', false) => Some(SemanticCommand::SelectAll),
        ('c', false) => Some(SemanticCommand::Copy),
        ('x', false) => Some(SemanticCommand::Cut),
        ('v', false) => Some(SemanticCommand::Paste),
        ('z', false) => Some(SemanticCommand::Undo),
        ('z', true) => Some(SemanticCommand::Redo),
        ('y', false) if !cfg!(target_os = "macos") => Some(SemanticCommand::Redo),
        _ => None,
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
        KeyboardDownAdmission, KeyboardDownAdmissionError, KeyboardDownDecision,
        KeyboardIngressDiagnostic, KeyboardIngressFailure, KeyboardInputOutcome,
        KeyboardInputState, NativeKeyTransition, native_shortcut_command, translate_key_location,
        translate_logical_key, translate_physical_key,
    };
    use runenui_core::{
        InputDeviceId, KeyModifiers, KeyboardCompositionState, KeyboardPhase,
        LogicalKey as NeutralLogicalKey, PhysicalKey as NeutralPhysicalKey, SemanticCommand,
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
            KeyboardInputOutcome::Submit { event, .. } => event,
            KeyboardInputOutcome::Suppressed(diagnostic) => {
                unreachable!("fixture transition must submit: {diagnostic:?}")
            }
            KeyboardInputOutcome::Failed(failure) => {
                unreachable!("fixture transition must not fail: {failure:?}")
            }
        }
    }

    fn pending_down(
        keyboard: &mut KeyboardInputState,
        device: InputDeviceId,
        physical: WinitPhysicalKey,
        logical: &WinitKey,
    ) -> KeyboardDownAdmission {
        let outcome = keyboard.key_input(
            device,
            &transition(ElementState::Pressed, physical, logical, false, false),
            KeyModifiers::NONE,
            KeyboardCompositionState::Inactive,
        );
        let KeyboardInputOutcome::Submit {
            event,
            down_admission: Some(admission),
        } = outcome
        else {
            unreachable!("original Down must require one explicit admission decision");
        };
        assert_eq!(event.phase(), KeyboardPhase::Down);
        assert!(!event.is_repeat());
        admission
    }

    #[test]
    fn unresolved_original_down_cannot_publish_a_terminal_up_or_synthetic_cancel() {
        let mut keyboard = KeyboardInputState::default();
        let a = device(104);
        let b = device(105);
        let logical = WinitKey::Character("w".into());
        let physical = WinitPhysicalKey::Code(KeyCode::KeyW);
        let pending = pending_down(&mut keyboard, a, physical, &logical);
        let other = pending_down(&mut keyboard, b, physical, &logical);
        assert_eq!(
            keyboard.resolve_down_admission(other, KeyboardDownDecision::Queued),
            Ok(())
        );

        for synthetic in [false, true] {
            assert!(matches!(
                keyboard.key_input(
                    a,
                    &transition(ElementState::Released, physical, &logical, false, synthetic),
                    KeyModifiers::NONE,
                    KeyboardCompositionState::Inactive,
                ),
                KeyboardInputOutcome::Failed(
                    KeyboardIngressFailure::UnresolvedOriginalDownAdmission
                )
            ));
            assert_eq!(keyboard.pressed_len(), 2);
        }

        // The terminal failure did not silently resolve, delete or alias the
        // original token. A real pre-queue rejection can still be acknowledged.
        assert_eq!(
            keyboard.resolve_down_admission(pending, KeyboardDownDecision::RejectedBeforeQueue),
            Ok(())
        );
        assert!(matches!(
            keyboard.key_input(
                a,
                &transition(ElementState::Released, physical, &logical, false, false),
                KeyModifiers::NONE,
                KeyboardCompositionState::Inactive,
            ),
            KeyboardInputOutcome::Suppressed(KeyboardIngressDiagnostic::ReleaseOfRejectedDown)
        ));
        let b_up = submitted(keyboard.key_input(
            b,
            &transition(ElementState::Released, physical, &logical, false, false),
            KeyModifiers::NONE,
            KeyboardCompositionState::Inactive,
        ));
        assert_eq!(b_up.phase(), KeyboardPhase::Up);
        assert_eq!(keyboard.pressed_len(), 0);

        let next = pending_down(&mut keyboard, a, physical, &logical);
        assert_ne!(next, pending);
        assert!(matches!(
            keyboard.key_input(
                a,
                &transition(ElementState::Released, physical, &logical, false, false),
                KeyModifiers::NONE,
                KeyboardCompositionState::Inactive,
            ),
            KeyboardInputOutcome::Failed(KeyboardIngressFailure::UnresolvedOriginalDownAdmission)
        ));
        assert_eq!(
            keyboard.resolve_down_admission(next, KeyboardDownDecision::Queued),
            Ok(())
        );
        assert_eq!(
            submitted(keyboard.key_input(
                a,
                &transition(ElementState::Released, physical, &logical, false, false),
                KeyModifiers::NONE,
                KeyboardCompositionState::Inactive,
            ))
            .phase(),
            KeyboardPhase::Up
        );
    }

    #[test]
    fn pending_source_cancellation_is_atomic_and_does_not_affect_another_keyboard() {
        let mut keyboard = KeyboardInputState::default();
        let a = device(106);
        let b = device(107);
        let logical = WinitKey::Character("w".into());
        let physical = WinitPhysicalKey::Code(KeyCode::KeyW);
        let pending_a = pending_down(&mut keyboard, a, physical, &logical);
        let queued_b = pending_down(&mut keyboard, b, physical, &logical);
        keyboard
            .resolve_down_admission(queued_b, KeyboardDownDecision::Queued)
            .unwrap_or_else(|error| unreachable!("B was queued: {error:?}"));
        let queued_a = pending_down(
            &mut keyboard,
            a,
            WinitPhysicalKey::Code(KeyCode::KeyX),
            &WinitKey::Character("x".into()),
        );
        keyboard
            .resolve_down_admission(queued_a, KeyboardDownDecision::Queued)
            .unwrap_or_else(|error| unreachable!("A's independent X was queued: {error:?}"));

        assert!(matches!(
            keyboard.cancel_device(a, KeyModifiers::NONE, KeyboardCompositionState::Inactive),
            Err(KeyboardIngressFailure::UnresolvedOriginalDownAdmission)
        ));
        assert!(matches!(
            keyboard.cancel_all(KeyModifiers::NONE, KeyboardCompositionState::Inactive),
            Err(KeyboardIngressFailure::UnresolvedOriginalDownAdmission)
        ));
        assert_eq!(keyboard.pressed_len(), 3, "neither failure removed a key");
        let b_cancels = keyboard
            .cancel_device(b, KeyModifiers::NONE, KeyboardCompositionState::Inactive)
            .unwrap_or_else(|error| unreachable!("B is independently admitted: {error:?}"));
        assert_eq!(b_cancels.len(), 1);
        assert_eq!(b_cancels[0].device_id(), Some(b));
        assert_eq!(keyboard.pressed_len(), 2);
        assert_eq!(
            keyboard.resolve_down_admission(pending_a, KeyboardDownDecision::RejectedBeforeQueue),
            Ok(())
        );
        let a_cancels = keyboard
            .cancel_device(a, KeyModifiers::NONE, KeyboardCompositionState::Inactive)
            .unwrap_or_else(|error| {
                unreachable!("A's original Down was rejected before queue: {error:?}")
            });
        assert_eq!(
            a_cancels.len(),
            1,
            "A's already queued X survives the failed Cancel"
        );
        assert_eq!(
            a_cancels[0].physical_key(),
            &NeutralPhysicalKey::Code(String::from("KeyX"))
        );
        assert_eq!(keyboard.pressed_len(), 0);
    }

    #[test]
    fn rejected_original_down_retains_only_physical_tombstone_until_authentic_up() {
        let mut keyboard = KeyboardInputState::default();
        let a = device(81);
        let logical = WinitKey::Character("w".into());
        let physical = WinitPhysicalKey::Code(KeyCode::KeyW);
        let initial = pending_down(&mut keyboard, a, physical, &logical);
        assert_eq!(
            keyboard.resolve_down_admission(initial, KeyboardDownDecision::RejectedBeforeQueue),
            Ok(())
        );
        assert_eq!(
            keyboard.resolve_down_admission(initial, KeyboardDownDecision::Queued),
            Err(KeyboardDownAdmissionError::StaleOrAlreadyResolved)
        );
        assert!(matches!(
            keyboard.key_input(
                a,
                &transition(ElementState::Pressed, physical, &logical, true, false),
                KeyModifiers::NONE,
                KeyboardCompositionState::Inactive,
            ),
            KeyboardInputOutcome::Suppressed(KeyboardIngressDiagnostic::RepeatOfRejectedDown)
        ));
        assert_eq!(keyboard.pressed_len(), 1);
        assert!(matches!(
            keyboard.key_input(
                a,
                &transition(ElementState::Pressed, physical, &logical, false, false),
                KeyModifiers::NONE,
                KeyboardCompositionState::Inactive,
            ),
            KeyboardInputOutcome::Suppressed(KeyboardIngressDiagnostic::DuplicatePress)
        ));
        assert!(matches!(
            keyboard.key_input(
                a,
                &transition(ElementState::Released, physical, &logical, false, false),
                KeyModifiers::NONE,
                KeyboardCompositionState::Inactive,
            ),
            KeyboardInputOutcome::Suppressed(KeyboardIngressDiagnostic::ReleaseOfRejectedDown)
        ));
        assert_eq!(keyboard.pressed_len(), 0);
        let new_down = pending_down(&mut keyboard, a, physical, &logical);
        assert_ne!(initial, new_down);
        assert_eq!(
            keyboard.resolve_down_admission(initial, KeyboardDownDecision::RejectedBeforeQueue),
            Err(KeyboardDownAdmissionError::StaleOrAlreadyResolved)
        );
        assert_eq!(
            keyboard.resolve_down_admission(new_down, KeyboardDownDecision::Queued),
            Ok(())
        );
        assert_eq!(
            keyboard
                .cancel_device(a, KeyModifiers::NONE, KeyboardCompositionState::Inactive)
                .unwrap_or_else(|error| unreachable!(
                    "all affected original Downs are resolved: {error:?}"
                ))
                .len(),
            1,
        );
    }

    #[test]
    fn rejected_a_and_queued_b_same_key_cancel_only_b_without_cross_device_state() {
        let mut keyboard = KeyboardInputState::default();
        let a = device(91);
        let b = device(92);
        let logical = WinitKey::Character("w".into());
        let physical = WinitPhysicalKey::Code(KeyCode::KeyW);
        let reject_a = pending_down(&mut keyboard, a, physical, &logical);
        let queued_b = pending_down(&mut keyboard, b, physical, &logical);
        keyboard
            .resolve_down_admission(reject_a, KeyboardDownDecision::RejectedBeforeQueue)
            .unwrap_or_else(|error| unreachable!("expected live original admission: {error:?}"));
        keyboard
            .resolve_down_admission(queued_b, KeyboardDownDecision::Queued)
            .unwrap_or_else(|error| unreachable!("expected live original admission: {error:?}"));
        assert_eq!(
            keyboard
                .cancel_device(a, KeyModifiers::NONE, KeyboardCompositionState::Inactive)
                .unwrap_or_else(|error| unreachable!(
                    "all affected original Downs are resolved: {error:?}"
                ))
                .len(),
            0
        );
        let b_repeat = submitted(keyboard.key_input(
            b,
            &transition(ElementState::Pressed, physical, &logical, true, false),
            KeyModifiers::NONE,
            KeyboardCompositionState::Inactive,
        ));
        assert!(b_repeat.is_repeat());
        let b_up = submitted(keyboard.key_input(
            b,
            &transition(ElementState::Released, physical, &logical, false, false),
            KeyModifiers::NONE,
            KeyboardCompositionState::Inactive,
        ));
        assert_eq!(b_up.phase(), KeyboardPhase::Up);
        assert_eq!(keyboard.pressed_len(), 0);
    }

    #[test]
    fn same_device_rejected_w_keeps_queued_x_cancel_and_pending_fails_closed() {
        let mut keyboard = KeyboardInputState::default();
        let a = device(93);
        let w = WinitKey::Character("w".into());
        let x = WinitKey::Character("x".into());
        let rejected = pending_down(&mut keyboard, a, WinitPhysicalKey::Code(KeyCode::KeyW), &w);
        let queued = pending_down(&mut keyboard, a, WinitPhysicalKey::Code(KeyCode::KeyX), &x);
        keyboard
            .resolve_down_admission(rejected, KeyboardDownDecision::RejectedBeforeQueue)
            .unwrap_or_else(|error| unreachable!("expected live original admission: {error:?}"));
        keyboard
            .resolve_down_admission(queued, KeyboardDownDecision::Queued)
            .unwrap_or_else(|error| unreachable!("expected live original admission: {error:?}"));
        let pending = pending_down(&mut keyboard, a, WinitPhysicalKey::Code(KeyCode::KeyZ), &x);
        assert!(matches!(
            keyboard.cancel_all(KeyModifiers::NONE, KeyboardCompositionState::Inactive),
            Err(KeyboardIngressFailure::UnresolvedOriginalDownAdmission)
        ));
        assert_eq!(keyboard.pressed_len(), 3, "failure must be atomic");
        assert_eq!(
            keyboard.resolve_down_admission(pending, KeyboardDownDecision::RejectedBeforeQueue),
            Ok(())
        );
        let cancels = keyboard
            .cancel_all(KeyModifiers::NONE, KeyboardCompositionState::Inactive)
            .unwrap_or_else(|error| {
                unreachable!("all affected original Downs are now resolved: {error:?}")
            });
        assert_eq!(cancels.len(), 1);
        assert_eq!(
            cancels[0].physical_key(),
            &NeutralPhysicalKey::Code(String::from("KeyX"))
        );
        assert_eq!(
            keyboard.resolve_down_admission(pending, KeyboardDownDecision::RejectedBeforeQueue),
            Err(KeyboardDownAdmissionError::StaleOrAlreadyResolved)
        );
    }

    #[test]
    fn original_down_token_cannot_alias_another_adapter_or_replacement() {
        let a = device(101);
        let key = WinitKey::Character("w".into());
        let physical = WinitPhysicalKey::Code(KeyCode::KeyW);
        let mut first = KeyboardInputState::default();
        let old = pending_down(&mut first, a, physical, &key);
        let mut second = KeyboardInputState::default();
        let unrelated = pending_down(&mut second, a, physical, &key);
        assert_ne!(
            old, unrelated,
            "adapter lifetimes never reuse pending token identity"
        );
        assert_eq!(
            second.resolve_down_admission(old, KeyboardDownDecision::RejectedBeforeQueue),
            Err(KeyboardDownAdmissionError::StaleOrAlreadyResolved)
        );
        assert_eq!(
            first.resolve_down_admission(unrelated, KeyboardDownDecision::Queued),
            Err(KeyboardDownAdmissionError::StaleOrAlreadyResolved)
        );
        drop(first);
        let mut replacement = KeyboardInputState::default();
        let fresh = pending_down(&mut replacement, a, physical, &key);
        assert_ne!(
            old, fresh,
            "replacement must not alias the old adapter's first Down"
        );
        assert_eq!(
            replacement.resolve_down_admission(old, KeyboardDownDecision::RejectedBeforeQueue),
            Err(KeyboardDownAdmissionError::StaleOrAlreadyResolved)
        );
        assert_eq!(
            replacement.resolve_down_admission(fresh, KeyboardDownDecision::Queued),
            Ok(())
        );
        assert_eq!(
            second.resolve_down_admission(unrelated, KeyboardDownDecision::Queued),
            Ok(())
        );
    }

    #[test]
    fn window_local_key_lifetimes_never_cross_acknowledge_or_cancel() {
        let device = device(201);
        let logical = WinitKey::Character("w".into());
        let physical = WinitPhysicalKey::Code(KeyCode::KeyW);
        let mut left_window = KeyboardInputState::default();
        let mut right_window = KeyboardInputState::default();
        let left = pending_down(&mut left_window, device, physical, &logical);
        let right = pending_down(&mut right_window, device, physical, &logical);

        assert_ne!(
            left, right,
            "identical native IDs do not alias window-local adapter tokens"
        );
        assert_eq!(
            right_window.resolve_down_admission(left, KeyboardDownDecision::Queued),
            Err(KeyboardDownAdmissionError::StaleOrAlreadyResolved)
        );
        assert_eq!(
            left_window.resolve_down_admission(left, KeyboardDownDecision::RejectedBeforeQueue),
            Ok(())
        );
        assert_eq!(
            right_window.resolve_down_admission(right, KeyboardDownDecision::Queued),
            Ok(())
        );
        let left_cancels = left_window
            .cancel_device(device, KeyModifiers::NONE, KeyboardCompositionState::Inactive)
            .unwrap_or_else(|error| unreachable!("left original is rejected: {error:?}"));
        assert_eq!(left_cancels.len(), 0);
        assert_eq!(left_window.pressed_len(), 0);
        let physical_up = submitted(right_window.key_input(
            device,
            &transition(ElementState::Released, physical, &logical, false, false),
            KeyModifiers::NONE,
            KeyboardCompositionState::Inactive,
        ));
        assert_eq!(physical_up.phase(), KeyboardPhase::Up);
        assert_eq!(physical_up.device_id(), Some(device));
        assert_eq!(right_window.pressed_len(), 0);
    }

    #[test]
    fn retired_device_generation_does_not_alias_a_fresh_neutral_source() {
        let mut keyboard = KeyboardInputState::default();
        let old_device = device(220);
        let reincarnated = device(221);
        let physical = WinitPhysicalKey::Code(KeyCode::KeyW);
        let logical = WinitKey::Character("w".into());
        let old = pending_down(&mut keyboard, old_device, physical, &logical);
        keyboard
            .resolve_down_admission(old, KeyboardDownDecision::RejectedBeforeQueue)
            .unwrap_or_else(|error| unreachable!("old original was rejected: {error:?}"));
        let old_cancels = keyboard
            .cancel_device(
                old_device,
                KeyModifiers::NONE,
                KeyboardCompositionState::Inactive,
            )
            .unwrap_or_else(|error| {
                unreachable!("old rejected tombstone can retire: {error:?}")
            });
        assert_eq!(old_cancels.len(), 0);

        let fresh = pending_down(&mut keyboard, reincarnated, physical, &logical);
        assert_ne!(old, fresh);
        assert_eq!(
            keyboard.resolve_down_admission(old, KeyboardDownDecision::Queued),
            Err(KeyboardDownAdmissionError::StaleOrAlreadyResolved)
        );
        assert_eq!(
            keyboard.resolve_down_admission(fresh, KeyboardDownDecision::Queued),
            Ok(())
        );
        let repeated_old_cancels = keyboard
            .cancel_device(
                old_device,
                KeyModifiers::NONE,
                KeyboardCompositionState::Inactive,
            )
            .unwrap_or_else(|error| unreachable!("old device is absent: {error:?}"));
        assert_eq!(repeated_old_cancels.len(), 0);
        let cancelled = keyboard
            .cancel_device(
                reincarnated,
                KeyModifiers::NONE,
                KeyboardCompositionState::Inactive,
            )
            .unwrap_or_else(|error| unreachable!("fresh original is queued: {error:?}"));
        assert_eq!(cancelled.len(), 1);
        assert_eq!(cancelled[0].phase(), KeyboardPhase::Cancel);
        assert_eq!(cancelled[0].device_id(), Some(reincarnated));
        assert_eq!(keyboard.pressed_len(), 0);
    }

    #[test]
    fn admission_identity_exhaustion_never_reuses_an_old_token() {
        let mut keyboard = KeyboardInputState {
            next_down_admission: u64::MAX,
            ..KeyboardInputState::default()
        };
        let key = WinitKey::Character("a".into());
        assert!(matches!(
            keyboard.key_input(
                device(100),
                &transition(
                    ElementState::Pressed,
                    WinitPhysicalKey::Code(KeyCode::KeyA),
                    &key,
                    false,
                    false,
                ),
                KeyModifiers::NONE,
                KeyboardCompositionState::Inactive,
            ),
            KeyboardInputOutcome::Failed(KeyboardIngressFailure::AdmissionIdentityExhausted)
        ));
        assert_eq!(keyboard.pressed_len(), 0);
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
            translate_logical_key(
                &WinitKey::Character("ß".into()),
                WinitPhysicalKey::Code(KeyCode::KeyQ),
                KeyModifiers::NONE,
                KeyboardCompositionState::Inactive,
            ),
            NeutralLogicalKey::Character(String::from("ß"))
        );
        assert_eq!(
            translate_logical_key(
                &WinitKey::Named(NamedKey::F5),
                WinitPhysicalKey::Code(KeyCode::F5),
                KeyModifiers::NONE,
                KeyboardCompositionState::Inactive,
            ),
            NeutralLogicalKey::Named(String::from("F5"))
        );
    }

    #[test]
    fn native_navigation_keys_map_to_structured_physical_and_logical_identities() {
        for (physical, logical, expected_physical, expected_logical) in [
            (
                KeyCode::Home,
                NamedKey::Home,
                NeutralPhysicalKey::Home,
                NeutralLogicalKey::Home,
            ),
            (
                KeyCode::End,
                NamedKey::End,
                NeutralPhysicalKey::End,
                NeutralLogicalKey::End,
            ),
            (
                KeyCode::PageUp,
                NamedKey::PageUp,
                NeutralPhysicalKey::PageUp,
                NeutralLogicalKey::PageUp,
            ),
            (
                KeyCode::PageDown,
                NamedKey::PageDown,
                NeutralPhysicalKey::PageDown,
                NeutralLogicalKey::PageDown,
            ),
        ] {
            assert_eq!(
                translate_physical_key(WinitPhysicalKey::Code(physical)),
                expected_physical
            );
            assert_eq!(
                translate_logical_key(
                    &WinitKey::Named(logical),
                    WinitPhysicalKey::Code(physical),
                    KeyModifiers::NONE,
                    KeyboardCompositionState::Inactive,
                ),
                expected_logical
            );
        }

        assert_eq!(
            translate_physical_key(WinitPhysicalKey::Code(KeyCode::Numpad7)),
            NeutralPhysicalKey::Code(String::from("Numpad7"))
        );
        assert_eq!(
            translate_logical_key(
                &WinitKey::Named(NamedKey::Home),
                WinitPhysicalKey::Code(KeyCode::Numpad7),
                KeyModifiers::NONE,
                KeyboardCompositionState::Inactive,
            ),
            NeutralLogicalKey::Home,
            "logical Home meaning must not rewrite the independent numpad physical identity"
        );
    }

    #[test]
    fn native_editing_keys_and_platform_shortcuts_normalize_without_text_commits() {
        assert_eq!(
            translate_logical_key(
                &WinitKey::Named(NamedKey::Backspace),
                WinitPhysicalKey::Code(KeyCode::Backspace),
                KeyModifiers::NONE,
                KeyboardCompositionState::Inactive,
            ),
            NeutralLogicalKey::Backspace
        );
        assert_eq!(
            translate_logical_key(
                &WinitKey::Named(NamedKey::Delete),
                WinitPhysicalKey::Code(KeyCode::Delete),
                KeyModifiers::NONE,
                KeyboardCompositionState::Inactive,
            ),
            NeutralLogicalKey::Delete
        );
        let primary = if cfg!(target_os = "macos") {
            KeyModifiers::META
        } else {
            KeyModifiers::CONTROL
        };
        for (key, expected) in [
            (KeyCode::KeyA, SemanticCommand::SelectAll),
            (KeyCode::KeyC, SemanticCommand::Copy),
            (KeyCode::KeyX, SemanticCommand::Cut),
            (KeyCode::KeyV, SemanticCommand::Paste),
            (KeyCode::KeyZ, SemanticCommand::Undo),
        ] {
            assert_eq!(
                native_shortcut_command(
                    match key {
                        KeyCode::KeyA => "a",
                        KeyCode::KeyC => "c",
                        KeyCode::KeyX => "x",
                        KeyCode::KeyV => "v",
                        KeyCode::KeyZ => "z",
                        KeyCode::KeyY => "y",
                        _ => unreachable!("shortcut fixture key has a logical character"),
                    },
                    primary,
                    KeyboardCompositionState::Inactive,
                ),
                Some(expected)
            );
        }
        assert_eq!(
            native_shortcut_command(
                "Z",
                primary.with_shift(),
                KeyboardCompositionState::Inactive,
            ),
            Some(SemanticCommand::Redo)
        );
        assert_eq!(
            native_shortcut_command("c", primary.with_alt(), KeyboardCompositionState::Inactive,),
            None,
            "AltGr/Option combinations must remain text input"
        );
        assert_eq!(
            native_shortcut_command("c", primary, KeyboardCompositionState::Active,),
            None,
            "composition-owned keys must not become editing shortcuts"
        );

        let mut keyboard = KeyboardInputState::default();
        let native_copy = WinitKey::Character("C".into());
        let event = submitted(keyboard.key_input(
            device(7),
            &transition(
                ElementState::Pressed,
                WinitPhysicalKey::Code(KeyCode::KeyC),
                &native_copy,
                false,
                false,
            ),
            primary,
            KeyboardCompositionState::Inactive,
        ));
        assert_eq!(
            event.logical_key(),
            &NeutralLogicalKey::Command(SemanticCommand::Copy),
            "the adapter emits normalized commands in the normal keyboard event"
        );
    }

    #[test]
    fn native_shortcuts_follow_logical_characters_on_qwertz_layouts() {
        let primary = if cfg!(target_os = "macos") {
            KeyModifiers::META
        } else {
            KeyModifiers::CONTROL
        };
        let mut keyboard = KeyboardInputState::default();
        let german_undo = WinitKey::Character("z".into());
        let event = submitted(keyboard.key_input(
            device(7),
            &transition(
                ElementState::Pressed,
                WinitPhysicalKey::Code(KeyCode::KeyY),
                &german_undo,
                false,
                false,
            ),
            primary,
            KeyboardCompositionState::Inactive,
        ));
        assert_eq!(
            event.logical_key(),
            &NeutralLogicalKey::Command(SemanticCommand::Undo),
            "Cmd/Ctrl+Z uses the logical Z even though its physical key code is KeyY"
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

        let KeyboardInputOutcome::Submit {
            event: down,
            down_admission: Some(admission),
        } = keyboard.key_input(
            device,
            &transition(ElementState::Pressed, physical, &logical, false, false),
            modifiers,
            KeyboardCompositionState::Inactive,
        )
        else {
            unreachable!("original Down must require an explicit acknowledgment");
        };
        assert!(matches!(
            keyboard.key_input(
                device,
                &transition(ElementState::Pressed, physical, &logical, true, false),
                modifiers,
                KeyboardCompositionState::Inactive,
            ),
            KeyboardInputOutcome::Suppressed(
                KeyboardIngressDiagnostic::RepeatWhileAdmissionPending
            )
        ));
        keyboard
            .resolve_down_admission(admission, KeyboardDownDecision::Queued)
            .unwrap_or_else(|error| unreachable!("expected live original admission: {error:?}"));
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
        let admission = pending_down(&mut keyboard, device, physical, &logical);
        assert_eq!(
            keyboard.resolve_down_admission(admission, KeyboardDownDecision::Queued),
            Ok(())
        );

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
    fn selective_device_cancellation_preserves_same_key_from_another_keyboard() {
        let mut keyboard = KeyboardInputState::default();
        let a = device(61);
        let b = device(62);
        let logical = WinitKey::Character("a".into());
        let physical = WinitPhysicalKey::Code(KeyCode::KeyA);
        let down = transition(ElementState::Pressed, physical, &logical, false, false);
        let KeyboardInputOutcome::Submit {
            event: down_a,
            down_admission: Some(admission_a),
        } = keyboard.key_input(
            a,
            &down,
            KeyModifiers::NONE,
            KeyboardCompositionState::Inactive,
        )
        else {
            unreachable!("original A Down has one admission token");
        };
        keyboard
            .resolve_down_admission(admission_a, KeyboardDownDecision::Queued)
            .unwrap_or_else(|error| unreachable!("A original Down is queued: {error:?}"));
        let KeyboardInputOutcome::Submit {
            event: down_b,
            down_admission: Some(admission_b),
        } = keyboard.key_input(
            b,
            &down,
            KeyModifiers::NONE,
            KeyboardCompositionState::Inactive,
        )
        else {
            unreachable!("B original Down must require admission");
        };
        keyboard
            .resolve_down_admission(admission_b, KeyboardDownDecision::Queued)
            .unwrap_or_else(|error| unreachable!("expected live original admission: {error:?}"));
        assert_eq!(down_a.device_id(), Some(a));
        assert_eq!(down_b.device_id(), Some(b));
        assert_eq!(keyboard.pressed_len(), 2);

        let cancelled = keyboard
            .cancel_device(a, KeyModifiers::SHIFT, KeyboardCompositionState::Active)
            .unwrap_or_else(|error| unreachable!("A original Down is queued: {error:?}"));
        assert_eq!(cancelled.len(), 1);
        assert_eq!(cancelled[0].phase(), KeyboardPhase::Cancel);
        assert_eq!(cancelled[0].device_id(), Some(a));
        assert_eq!(cancelled[0].physical_key(), down_a.physical_key());
        assert_eq!(cancelled[0].logical_key(), down_a.logical_key());
        assert_eq!(cancelled[0].modifiers(), KeyModifiers::SHIFT);
        assert_eq!(
            cancelled[0].composition_state(),
            KeyboardCompositionState::Active
        );
        assert_eq!(keyboard.pressed_len(), 1);
        assert_eq!(
            keyboard
                .cancel_device(a, KeyModifiers::NONE, KeyboardCompositionState::Inactive)
                .unwrap_or_else(|error| unreachable!(
                    "all affected original Downs are resolved: {error:?}"
                ))
                .len(),
            0
        );
        assert!(matches!(
            keyboard.key_input(
                a,
                &transition(ElementState::Released, physical, &logical, false, false),
                KeyModifiers::NONE,
                KeyboardCompositionState::Inactive,
            ),
            KeyboardInputOutcome::Suppressed(KeyboardIngressDiagnostic::ReleaseWithoutPress)
        ));
        let repeated_b = submitted(keyboard.key_input(
            b,
            &transition(ElementState::Pressed, physical, &logical, true, false),
            KeyModifiers::NONE,
            KeyboardCompositionState::Inactive,
        ));
        assert_eq!(repeated_b.phase(), KeyboardPhase::Down);
        assert!(repeated_b.is_repeat());
        let up_b = submitted(keyboard.key_input(
            b,
            &transition(ElementState::Released, physical, &logical, false, false),
            KeyModifiers::NONE,
            KeyboardCompositionState::Inactive,
        ));
        assert_eq!(up_b.phase(), KeyboardPhase::Up);
        assert_eq!(up_b.device_id(), Some(b));
        assert_eq!(keyboard.pressed_len(), 0);

        let new_device = device(63);
        let successor = submitted(keyboard.key_input(
            new_device,
            &down,
            KeyModifiers::NONE,
            KeyboardCompositionState::Inactive,
        ));
        assert_eq!(successor.device_id(), Some(new_device));
        assert_eq!(successor.phase(), KeyboardPhase::Down);
    }

    #[test]
    fn selective_cancel_preserves_interleaved_admission_order_and_other_devices() {
        let mut keyboard = KeyboardInputState::default();
        let a = device(64);
        let b = device(65);
        let logical_a = WinitKey::Character("a".into());
        let logical_c = WinitKey::Character("c".into());
        for (device, physical, logical) in [
            (a, WinitPhysicalKey::Code(KeyCode::KeyA), &logical_a),
            (b, WinitPhysicalKey::Code(KeyCode::KeyA), &logical_a),
            (a, WinitPhysicalKey::Code(KeyCode::KeyC), &logical_c),
            (b, WinitPhysicalKey::Code(KeyCode::KeyC), &logical_c),
        ] {
            let admission = pending_down(&mut keyboard, device, physical, logical);
            keyboard
                .resolve_down_admission(admission, KeyboardDownDecision::Queued)
                .unwrap_or_else(|error| unreachable!("original Down is queued: {error:?}"));
        }
        let cancelled = keyboard
            .cancel_device(a, KeyModifiers::NONE, KeyboardCompositionState::Inactive)
            .unwrap_or_else(|error| unreachable!("A originals are queued: {error:?}"));
        assert_eq!(cancelled.len(), 2);
        assert_eq!(
            cancelled[0].physical_key(),
            &NeutralPhysicalKey::Code(String::from("KeyA"))
        );
        assert_eq!(
            cancelled[1].physical_key(),
            &NeutralPhysicalKey::Code(String::from("KeyC"))
        );
        assert!(cancelled.iter().all(|key| key.device_id() == Some(a)));
        assert_eq!(keyboard.pressed_len(), 2);
        let b_cancelled = keyboard
            .cancel_all(KeyModifiers::ALT, KeyboardCompositionState::Inactive)
            .unwrap_or_else(|error| unreachable!("B originals are queued: {error:?}"));
        assert_eq!(b_cancelled.len(), 2);
        assert!(b_cancelled.iter().all(|key| key.device_id() == Some(b)));
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
            let admission = pending_down(&mut keyboard, device, physical, logical);
            keyboard
                .resolve_down_admission(admission, KeyboardDownDecision::Queued)
                .unwrap_or_else(|error| unreachable!("original Down is queued: {error:?}"));
        }

        let cancelled = keyboard
            .cancel_all(KeyModifiers::META, KeyboardCompositionState::Inactive)
            .unwrap_or_else(|error| unreachable!("all original Downs are queued: {error:?}"));
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
