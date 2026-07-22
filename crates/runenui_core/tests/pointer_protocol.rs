use runenui_core::{
    __runtime::RuntimeNamespace, InputDeviceId, KeyModifiers, LogicalDelta, LogicalPoint,
    PointerButton, PointerButtons, PointerDeviceKind, PointerEvent, PointerId, PointerPhase,
};

#[test]
fn complete_pointer_payload_round_trips_through_host_neutral_accessors() {
    let namespace = RuntimeNamespace::__runtime_new();
    let surface = namespace.__runtime_surface_id(3, 7);
    let context = namespace
        .__runtime_surface_context(surface, 11, 13)
        .unwrap_or_else(|| unreachable!("the surface belongs to this namespace"));
    let pointer_id =
        PointerId::new(17).unwrap_or_else(|| unreachable!("the pointer identity is non-zero"));
    let device_id =
        InputDeviceId::new(19).unwrap_or_else(|| unreachable!("the device identity is non-zero"));
    let position = LogicalPoint::new(1.25, -2.5)
        .unwrap_or_else(|_| unreachable!("the logical position is finite"));
    let movement = LogicalDelta::new(3.0, -4.0)
        .unwrap_or_else(|_| unreachable!("the movement delta is finite"));
    let scroll =
        LogicalDelta::new(-5.0, 6.0).unwrap_or_else(|_| unreachable!("the scroll delta is finite"));
    let buttons = PointerButtons::new([
        PointerButton::Other(9),
        PointerButton::Primary,
        PointerButton::Primary,
    ]);
    let modifiers = KeyModifiers::NONE.with_shift().with_meta();

    let event = PointerEvent::new(
        pointer_id,
        PointerDeviceKind::Pen,
        PointerPhase::Wheel,
        position,
        context.clone(),
    )
    .with_device_id(device_id)
    .with_movement_delta(movement)
    .with_scroll_delta(scroll)
    .with_buttons(buttons.clone())
    .with_changed_button(PointerButton::Other(9))
    .with_modifiers(modifiers);

    assert_eq!(event.pointer_id(), pointer_id);
    assert_eq!(event.device_id(), Some(device_id));
    assert_eq!(event.device_kind(), PointerDeviceKind::Pen);
    assert_eq!(event.phase(), PointerPhase::Wheel);
    assert_eq!(event.position(), position);
    assert_eq!(event.movement_delta(), movement);
    assert_eq!(event.scroll_delta(), scroll);
    assert_eq!(event.buttons(), &buttons);
    assert_eq!(event.changed_button(), Some(PointerButton::Other(9)));
    assert_eq!(event.modifiers(), modifiers);
    assert_eq!(event.surface_context(), &context);
    assert_eq!(
        event.buttons().iter().collect::<Vec<_>>(),
        [PointerButton::Primary, PointerButton::Other(9),]
    );
}
