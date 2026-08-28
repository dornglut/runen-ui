use super::{
    MouseButtonOutcome, MouseIngressDiagnostic, MouseInputState, TranslatedPointerPoint,
    logical_movement_delta,
};
use crate::DemoApp;
use runenui_core::{
    InputDeviceId, KeyModifiers, LogicalDelta, LogicalPoint, PointerButton, PointerPhase,
    StyleTokens,
};
use runenui_runtime::{AppRuntime, LogicalSize, PumpBudget, SurfaceBuildContext};
use winit::event::{ElementState, MouseButton};

fn point(x: f32, y: f32) -> LogicalPoint {
    LogicalPoint::new(x, y)
        .unwrap_or_else(|_| unreachable!("fixture logical point coordinates are finite"))
}

fn device(value: u64) -> InputDeviceId {
    InputDeviceId::new(value)
        .unwrap_or_else(|| unreachable!("fixture input device identity is non-zero"))
}

fn translated_point(position: LogicalPoint, modifiers: KeyModifiers) -> TranslatedPointerPoint {
    let mut runtime = AppRuntime::<DemoApp>::mount(());
    runtime.pump(PumpBudget::new(
        usize::MAX,
        usize::MAX,
        usize::MAX,
        usize::MAX,
    ));
    let style_tokens = StyleTokens::new();
    let logical_size = LogicalSize::try_new(200.0, 120.0)
        .unwrap_or_else(|_| unreachable!("fixture logical size is valid"));
    let context = SurfaceBuildContext::tight(&style_tokens, logical_size);
    let publication = runtime
        .publish_surface(&context)
        .unwrap_or_else(|error| unreachable!("fixture publication is valid: {error:?}"));
    TranslatedPointerPoint {
        position,
        input_context: publication.input_context().clone(),
        modifiers,
    }
}

fn submitted(outcome: MouseButtonOutcome) -> runenui_core::PointerEvent {
    match outcome {
        MouseButtonOutcome::Submit(event) => event,
        MouseButtonOutcome::Suppressed(diagnostic) => {
            unreachable!("fixture transition must submit: {diagnostic:?}")
        }
    }
}

#[test]
fn movement_delta_is_zero_without_a_previous_admitted_point() {
    assert_eq!(
        logical_movement_delta(None, point(12.0, 8.0)),
        Ok(LogicalDelta::ZERO)
    );
}

#[test]
fn movement_delta_uses_previous_admitted_logical_point() {
    let delta = logical_movement_delta(Some(point(10.0, 20.0)), point(14.0, 15.0))
        .unwrap_or_else(|_| unreachable!("fixture movement delta is representable"));
    assert!((delta.x() - 4.0).abs() < f32::EPSILON);
    assert!((delta.y() + 5.0).abs() < f32::EPSILON);
}

#[test]
fn movement_delta_rejects_non_representable_difference() {
    assert_eq!(
        logical_movement_delta(Some(point(f32::MAX, 0.0)), point(f32::MIN, 0.0)),
        Err(MouseIngressDiagnostic::MovementDeltaOutOfRange)
    );
}

#[test]
fn wheel_allocates_and_reuses_mouse_stream_with_exact_neutral_state() {
    let mut mouse = MouseInputState::default();
    let device_id = device(7);
    let modifiers = KeyModifiers::SHIFT.with_alt();
    let initial = translated_point(point(20.0, 30.0), modifiers);
    let moved = mouse
        .cursor_moved(device_id, initial.clone())
        .unwrap_or_else(|_| unreachable!("fixture mouse move is admitted"));
    let pointer_id = moved.pointer_id();
    let _down = submitted(
        mouse
            .button_input(
                device_id,
                ElementState::Pressed,
                MouseButton::Left,
                Some(initial.clone()),
            )
            .unwrap_or_else(|_| unreachable!("fixture button press is admitted")),
    );
    let scroll_delta = LogicalDelta::new(-2.5, 6.0)
        .unwrap_or_else(|_| unreachable!("fixture scroll delta is finite"));

    let wheel = mouse
        .wheel(device_id, initial.clone(), scroll_delta)
        .unwrap_or_else(|_| unreachable!("fixture wheel input is admitted"));

    assert_eq!(wheel.pointer_id(), pointer_id);
    assert_eq!(wheel.device_id(), Some(device_id));
    assert_eq!(wheel.phase(), PointerPhase::Wheel);
    assert_eq!(wheel.position(), initial.position);
    assert_eq!(wheel.surface_context(), &initial.input_context);
    assert_eq!(wheel.movement_delta(), LogicalDelta::ZERO);
    assert_eq!(wheel.scroll_delta(), scroll_delta);
    assert!(wheel.buttons().contains(PointerButton::Primary));
    assert_eq!(wheel.modifiers(), modifiers);
}

#[test]
fn primary_first_chord_retains_stream_through_partial_release_move_and_wheel() {
    let mut mouse = MouseInputState::default();
    let device_id = device(10);
    let initial = translated_point(point(20.0, 30.0), KeyModifiers::NONE);

    let primary_down = submitted(
        mouse
            .button_input(
                device_id,
                ElementState::Pressed,
                MouseButton::Left,
                Some(initial.clone()),
            )
            .unwrap_or_else(|_| unreachable!("primary press is admitted")),
    );
    let pointer_id = primary_down.pointer_id();
    assert_eq!(primary_down.changed_button(), Some(PointerButton::Primary));
    assert!(primary_down.buttons().contains(PointerButton::Primary));

    let secondary_down = submitted(
        mouse
            .button_input(
                device_id,
                ElementState::Pressed,
                MouseButton::Right,
                Some(initial.clone()),
            )
            .unwrap_or_else(|_| unreachable!("secondary press is admitted")),
    );
    assert_eq!(secondary_down.pointer_id(), pointer_id);
    assert_eq!(
        secondary_down.changed_button(),
        Some(PointerButton::Secondary)
    );
    assert!(secondary_down.buttons().contains(PointerButton::Primary));
    assert!(secondary_down.buttons().contains(PointerButton::Secondary));

    let primary_up = submitted(
        mouse
            .button_input(
                device_id,
                ElementState::Released,
                MouseButton::Left,
                Some(initial.clone()),
            )
            .unwrap_or_else(|_| unreachable!("partial primary release is admitted")),
    );
    assert_eq!(primary_up.pointer_id(), pointer_id);
    assert_eq!(primary_up.phase(), PointerPhase::Up);
    assert_eq!(primary_up.changed_button(), Some(PointerButton::Primary));
    assert!(!primary_up.buttons().contains(PointerButton::Primary));
    assert!(primary_up.buttons().contains(PointerButton::Secondary));
    assert_eq!(mouse.active_device_id(), Some(device_id));

    let moved_point = TranslatedPointerPoint {
        position: point(24.0, 35.0),
        ..initial.clone()
    };
    let moved = mouse
        .cursor_moved(device_id, moved_point.clone())
        .unwrap_or_else(|_| unreachable!("move after partial release is admitted"));
    assert_eq!(moved.pointer_id(), pointer_id);
    assert_eq!(moved.changed_button(), None);
    assert!(!moved.buttons().contains(PointerButton::Primary));
    assert!(moved.buttons().contains(PointerButton::Secondary));

    let scroll_delta = LogicalDelta::new(0.0, 5.0)
        .unwrap_or_else(|_| unreachable!("fixture scroll delta is finite"));
    let wheel = mouse
        .wheel(device_id, moved_point.clone(), scroll_delta)
        .unwrap_or_else(|_| unreachable!("wheel after partial release is admitted"));
    assert_eq!(wheel.pointer_id(), pointer_id);
    assert_eq!(wheel.changed_button(), None);
    assert!(!wheel.buttons().contains(PointerButton::Primary));
    assert!(wheel.buttons().contains(PointerButton::Secondary));

    let secondary_up = submitted(
        mouse
            .button_input(
                device_id,
                ElementState::Released,
                MouseButton::Right,
                Some(moved_point.clone()),
            )
            .unwrap_or_else(|_| unreachable!("final secondary release is admitted")),
    );
    assert_eq!(secondary_up.pointer_id(), pointer_id);
    assert_eq!(
        secondary_up.changed_button(),
        Some(PointerButton::Secondary)
    );
    assert!(secondary_up.buttons().is_empty());
    assert_eq!(mouse.active_device_id(), None);

    let next = mouse
        .cursor_moved(device_id, moved_point)
        .unwrap_or_else(|_| unreachable!("next stream move is admitted"));
    assert_ne!(next.pointer_id(), pointer_id);
}

#[test]
fn secondary_first_chord_retains_stream_until_final_primary_release() {
    let mut mouse = MouseInputState::default();
    let device_id = device(11);
    let initial = translated_point(point(12.0, 18.0), KeyModifiers::CONTROL);

    let secondary_down = submitted(
        mouse
            .button_input(
                device_id,
                ElementState::Pressed,
                MouseButton::Right,
                Some(initial.clone()),
            )
            .unwrap_or_else(|_| unreachable!("secondary press is admitted")),
    );
    let pointer_id = secondary_down.pointer_id();

    let primary_down = submitted(
        mouse
            .button_input(
                device_id,
                ElementState::Pressed,
                MouseButton::Left,
                Some(initial.clone()),
            )
            .unwrap_or_else(|_| unreachable!("primary press is admitted")),
    );
    assert_eq!(primary_down.pointer_id(), pointer_id);
    assert!(primary_down.buttons().contains(PointerButton::Primary));
    assert!(primary_down.buttons().contains(PointerButton::Secondary));

    let secondary_up = submitted(
        mouse
            .button_input(
                device_id,
                ElementState::Released,
                MouseButton::Right,
                Some(initial.clone()),
            )
            .unwrap_or_else(|_| unreachable!("partial secondary release is admitted")),
    );
    assert_eq!(secondary_up.pointer_id(), pointer_id);
    assert_eq!(
        secondary_up.changed_button(),
        Some(PointerButton::Secondary)
    );
    assert!(secondary_up.buttons().contains(PointerButton::Primary));
    assert!(!secondary_up.buttons().contains(PointerButton::Secondary));
    assert_eq!(mouse.active_device_id(), Some(device_id));

    let moved_point = TranslatedPointerPoint {
        position: point(15.0, 22.0),
        ..initial
    };
    let moved = mouse
        .cursor_moved(device_id, moved_point.clone())
        .unwrap_or_else(|_| unreachable!("move after partial release is admitted"));
    assert_eq!(moved.pointer_id(), pointer_id);
    assert!(moved.buttons().contains(PointerButton::Primary));
    assert!(!moved.buttons().contains(PointerButton::Secondary));

    let primary_up = submitted(
        mouse
            .button_input(
                device_id,
                ElementState::Released,
                MouseButton::Left,
                Some(moved_point),
            )
            .unwrap_or_else(|_| unreachable!("final primary release is admitted")),
    );
    assert_eq!(primary_up.pointer_id(), pointer_id);
    assert_eq!(primary_up.changed_button(), Some(PointerButton::Primary));
    assert!(primary_up.buttons().is_empty());
    assert_eq!(mouse.active_device_id(), None);
}

#[test]
fn wheel_rejects_device_change_without_mutating_active_stream() {
    let mut mouse = MouseInputState::default();
    let first_device = device(8);
    let second_device = device(9);
    let point = translated_point(point(5.0, 7.0), KeyModifiers::NONE);
    let moved = mouse
        .cursor_moved(first_device, point.clone())
        .unwrap_or_else(|_| unreachable!("fixture first device opens a stream"));
    let pointer_id = moved.pointer_id();
    let delta = LogicalDelta::new(0.0, 4.0)
        .unwrap_or_else(|_| unreachable!("fixture scroll delta is finite"));

    assert!(matches!(
        mouse.wheel(second_device, point, delta),
        Err(MouseIngressDiagnostic::DeviceMismatch { active, incoming })
            if active == first_device && incoming == second_device
    ));
    assert_eq!(mouse.active_device_id(), Some(first_device));

    let cancel = mouse
        .cancel_for_device_change(KeyModifiers::NONE)
        .unwrap_or_else(|| unreachable!("active stream can be cancelled"));
    assert_eq!(cancel.pointer_id(), pointer_id);
    assert_eq!(cancel.device_id(), Some(first_device));
}
