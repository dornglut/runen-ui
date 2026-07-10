use runenui_runtime::prelude::{
    InputEvent, InputIntent, InputIntentResolver, Key, KeyModifiers, KeyPhase, KeyboardEvent,
    LogicalPoint, PointerButton, PointerEvent, PointerPhase, RuntimeNodeId,
};

#[test]
fn primary_pointer_press_with_target_resolves_activation_intent() {
    let target = RuntimeNodeId::from_index(7);
    let event = InputEvent::Pointer(PointerEvent::new(
        PointerPhase::Pressed,
        LogicalPoint::new(12.0, 24.0),
        Some(PointerButton::Primary),
        KeyModifiers::NONE,
        Some(target),
    ));

    assert_eq!(
        event.resolve_intent(),
        Some(InputIntent::activate_node(target))
    );
}

#[test]
fn primary_pointer_press_without_target_is_ignored() {
    let event = InputEvent::Pointer(PointerEvent::new(
        PointerPhase::Pressed,
        LogicalPoint::new(12.0, 24.0),
        Some(PointerButton::Primary),
        KeyModifiers::NONE,
        None,
    ));

    assert_eq!(event.resolve_intent(), None);
}

#[test]
fn non_primary_pointer_input_is_ignored() {
    let target = RuntimeNodeId::from_index(3);
    let secondary = InputEvent::Pointer(PointerEvent::new(
        PointerPhase::Pressed,
        LogicalPoint::new(1.0, 2.0),
        Some(PointerButton::Secondary),
        KeyModifiers::NONE,
        Some(target),
    ));
    let movement = InputEvent::Pointer(PointerEvent::new(
        PointerPhase::Moved,
        LogicalPoint::new(1.0, 2.0),
        None,
        KeyModifiers::NONE,
        Some(target),
    ));
    let release = InputEvent::Pointer(PointerEvent::new(
        PointerPhase::Released,
        LogicalPoint::new(1.0, 2.0),
        Some(PointerButton::Primary),
        KeyModifiers::NONE,
        Some(target),
    ));

    assert_eq!(secondary.resolve_intent(), None);
    assert_eq!(movement.resolve_intent(), None);
    assert_eq!(release.resolve_intent(), None);
}

#[test]
fn enter_and_space_key_press_with_target_resolve_activation_intents() {
    let enter_target = RuntimeNodeId::from_index(4);
    let space_target = RuntimeNodeId::from_index(5);
    let enter = InputEvent::Keyboard(KeyboardEvent::new(
        KeyPhase::Pressed,
        Key::Enter,
        KeyModifiers::NONE,
        Some(enter_target),
    ));
    let space = InputEvent::Keyboard(KeyboardEvent::new(
        KeyPhase::Pressed,
        Key::Space,
        KeyModifiers::NONE,
        Some(space_target),
    ));

    assert_eq!(
        enter.resolve_intent(),
        Some(InputIntent::activate_node(enter_target))
    );
    assert_eq!(
        space.resolve_intent(),
        Some(InputIntent::activate_node(space_target))
    );
}

#[test]
fn keyboard_input_without_activation_key_or_target_is_ignored() {
    let target = RuntimeNodeId::from_index(8);
    let released_enter = InputEvent::Keyboard(KeyboardEvent::new(
        KeyPhase::Released,
        Key::Enter,
        KeyModifiers::NONE,
        Some(target),
    ));
    let tab = InputEvent::Keyboard(KeyboardEvent::new(
        KeyPhase::Pressed,
        Key::Tab,
        KeyModifiers::NONE,
        Some(target),
    ));
    let no_target = InputEvent::Keyboard(KeyboardEvent::new(
        KeyPhase::Pressed,
        Key::Enter,
        KeyModifiers::NONE,
        None,
    ));

    assert_eq!(released_enter.resolve_intent(), None);
    assert_eq!(tab.resolve_intent(), None);
    assert_eq!(no_target.resolve_intent(), None);
}
