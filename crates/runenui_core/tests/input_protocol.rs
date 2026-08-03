use runenui_core::{
    CommittedTextEvent, CompositionRange, CompositionRangeError, InputDeviceId, KeyLocation,
    KeyModifiers, KeyboardCompositionState, KeyboardEvent, KeyboardPhase, LogicalKey, PhysicalKey,
    UiEvent,
};

#[test]
fn keyboard_event_preserves_host_neutral_physical_logical_and_device_facts() {
    let device = InputDeviceId::new(7).unwrap_or_else(|| unreachable!("nonzero device"));
    let event = KeyboardEvent::new(
        KeyboardPhase::Down,
        PhysicalKey::Code(String::from("KeyQ")),
        LogicalKey::Character(String::from("@")),
        KeyModifiers::SHIFT,
        true,
        KeyLocation::Left,
        KeyboardCompositionState::Active,
        Some(device),
    );

    assert_eq!(event.phase(), KeyboardPhase::Down);
    assert_eq!(
        event.physical_key(),
        &PhysicalKey::Code(String::from("KeyQ"))
    );
    assert_eq!(
        event.logical_key(),
        &LogicalKey::Character(String::from("@"))
    );
    assert_eq!(event.modifiers(), KeyModifiers::SHIFT);
    assert!(event.is_repeat());
    assert_eq!(event.location(), KeyLocation::Left);
    assert_eq!(event.composition_state(), KeyboardCompositionState::Active);
    assert_eq!(event.device_id(), Some(device));
    assert!(UiEvent::Keyboard(event).as_keyboard().is_some());
}

#[test]
fn committed_text_requires_nonempty_unicode_and_stays_distinct_from_keys() {
    assert!(CommittedTextEvent::new("", None).is_err());
    let event = CommittedTextEvent::new("🙂é", None)
        .unwrap_or_else(|_| unreachable!("nonempty committed text is valid"));
    assert_eq!(event.text(), "🙂é");
    assert_eq!(event.text().chars().count(), 2);
    assert!(UiEvent::CommittedText(event).as_committed_text().is_some());
}

#[test]
fn composition_ranges_remain_checked_utf8_boundaries() {
    let preedit = "aé🙂";
    assert_eq!(
        CompositionRange::new(preedit, 1, 3).map(|range| (range.start(), range.end())),
        Ok((1, 3))
    );
    assert_eq!(
        CompositionRange::new(preedit, 3, 1),
        Err(CompositionRangeError::Reversed)
    );
    assert_eq!(
        CompositionRange::new(preedit, 0, preedit.len() + 1),
        Err(CompositionRangeError::OutOfBounds)
    );
    assert_eq!(
        CompositionRange::new(preedit, 2, 3),
        Err(CompositionRangeError::NotScalarBoundary)
    );
}
