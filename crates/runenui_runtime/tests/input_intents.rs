use runenui_runtime::{
    InputEvent, InputIntent, Key, KeyModifiers, KeyPhase, KeyboardEvent, LogicalPoint,
    PointerButton, PointerEvent, PointerPhase, RuntimeNodeId,
};

#[test]
fn pointer_event_carries_phase_position_button_modifiers_and_target() {
    let target = RuntimeNodeId::from_index(7);
    let modifiers = KeyModifiers::NONE.with_shift().with_alt();
    let event = PointerEvent::new(
        PointerPhase::Pressed,
        LogicalPoint::new(12.5, 24.0),
        Some(PointerButton::Primary),
        modifiers,
        Some(target),
    );

    assert_eq!(event.phase(), PointerPhase::Pressed);
    assert_eq!(event.position(), LogicalPoint::new(12.5, 24.0));
    assert_eq!(event.button(), Some(PointerButton::Primary));
    assert_eq!(event.modifiers(), modifiers);
    assert_eq!(event.target(), Some(target));
    assert!(event.modifiers().shift());
    assert!(!event.modifiers().control());
    assert!(event.modifiers().alt());
    assert!(!event.modifiers().meta());
}

#[test]
fn pointer_event_can_represent_unresolved_movement() {
    let event = PointerEvent::new(
        PointerPhase::Moved,
        LogicalPoint::new(3.0, 4.0),
        None,
        KeyModifiers::NONE,
        None,
    );

    assert_eq!(event.phase(), PointerPhase::Moved);
    assert_eq!(event.button(), None);
    assert_eq!(event.modifiers(), KeyModifiers::NONE);
    assert_eq!(event.target(), None);
}

#[test]
fn keyboard_event_carries_phase_key_modifiers_and_target() {
    let target = RuntimeNodeId::from_index(2);
    let modifiers = KeyModifiers::NONE.with_control().with_meta();
    let event = KeyboardEvent::new(KeyPhase::Pressed, Key::Enter, modifiers, Some(target));

    assert_eq!(event.phase(), KeyPhase::Pressed);
    assert_eq!(event.key(), &Key::Enter);
    assert_eq!(event.modifiers(), modifiers);
    assert_eq!(event.target(), Some(target));
    assert!(!event.modifiers().shift());
    assert!(event.modifiers().control());
    assert!(!event.modifiers().alt());
    assert!(event.modifiers().meta());
}

#[test]
fn key_modifiers_mask_unknown_bits_and_preserve_known_flags() {
    let modifiers = KeyModifiers::from_bits(u8::MAX);

    assert_eq!(modifiers.bits(), 0b1111);
    assert!(modifiers.shift());
    assert!(modifiers.control());
    assert!(modifiers.alt());
    assert!(modifiers.meta());
}

#[test]
fn keyboard_event_can_carry_text_or_host_specific_keys() {
    let character = KeyboardEvent::new(
        KeyPhase::Released,
        Key::Character('x'),
        KeyModifiers::NONE,
        None,
    );
    let named = KeyboardEvent::new(
        KeyPhase::Pressed,
        Key::Named("F13".to_owned()),
        KeyModifiers::NONE,
        None,
    );

    assert_eq!(character.key(), &Key::Character('x'));
    assert_eq!(named.key(), &Key::Named("F13".to_owned()));
}

#[test]
fn input_event_wraps_pointer_and_keyboard_events() {
    let pointer = PointerEvent::new(
        PointerPhase::Released,
        LogicalPoint::new(1.0, 2.0),
        Some(PointerButton::Secondary),
        KeyModifiers::NONE,
        Some(RuntimeNodeId::ROOT),
    );
    let keyboard = KeyboardEvent::new(
        KeyPhase::Pressed,
        Key::Space,
        KeyModifiers::NONE,
        Some(RuntimeNodeId::ROOT),
    );

    assert_eq!(InputEvent::Pointer(pointer), InputEvent::Pointer(pointer));
    assert_eq!(
        InputEvent::Keyboard(keyboard.clone()),
        InputEvent::Keyboard(keyboard)
    );
}

#[test]
fn input_intent_names_runtime_node_activation_without_dispatching() {
    let target = RuntimeNodeId::from_index(4);

    assert_eq!(
        InputIntent::activate_node(target),
        InputIntent::ActivateNode(target)
    );
}
