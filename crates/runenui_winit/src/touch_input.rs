//! Native touch-contact normalization into the ordinary pointer protocol.
//!
//! The selected winit host profile owns contact identity and physical-coordinate
//! translation. Runtime receives only unique neutral pointer IDs, the resolved
//! host-session device ID, a touch device kind, and the displayed input context.

use std::collections::BTreeMap;

use runenui_core::{
    InputDeviceId, LogicalDelta, LogicalPoint, PointerButton, PointerButtons, PointerDeviceKind,
    PointerEvent, PointerId, PointerPhase, SurfaceInputContext,
};
use winit::event::TouchPhase;

const FIRST_TOUCH_POINTER_ID: u64 = 2;
const POINTER_ID_STRIDE: u64 = 2;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TouchIngressDiagnostic {
    DuplicateStart,
    UnknownContact,
    PointerIdentityExhausted,
    MovementDeltaOutOfRange,
}

#[derive(Clone, Debug)]
struct Contact {
    pointer_id: PointerId,
    position: LogicalPoint,
    input_context: SurfaceInputContext,
}

/// Tracks active contacts for the selected native host without retaining any
/// platform callback state in the runtime.
#[derive(Debug)]
pub struct TouchInputState {
    next_pointer_id: Option<u64>,
    contacts: BTreeMap<(InputDeviceId, u64), Contact>,
}

impl Default for TouchInputState {
    fn default() -> Self {
        Self {
            next_pointer_id: Some(FIRST_TOUCH_POINTER_ID),
            contacts: BTreeMap::new(),
        }
    }
}

impl TouchInputState {
    /// Normalizes one winit contact transition using exact displayed geometry.
    ///
    /// # Errors
    ///
    /// Rejects duplicate starts, orphan transitions, exhausted pointer identity,
    /// or coordinates whose movement delta is outside the neutral domain.
    pub fn transition(
        &mut self,
        device_id: InputDeviceId,
        contact_id: u64,
        phase: TouchPhase,
        position: LogicalPoint,
        input_context: SurfaceInputContext,
    ) -> Result<PointerEvent, TouchIngressDiagnostic> {
        let key = (device_id, contact_id);
        match phase {
            TouchPhase::Started => {
                if self.contacts.contains_key(&key) {
                    return Err(TouchIngressDiagnostic::DuplicateStart);
                }
                let value = self
                    .next_pointer_id
                    .take()
                    .ok_or(TouchIngressDiagnostic::PointerIdentityExhausted)?;
                let pointer_id = PointerId::new(value)
                    .ok_or(TouchIngressDiagnostic::PointerIdentityExhausted)?;
                self.next_pointer_id = value.checked_add(POINTER_ID_STRIDE);
                self.contacts.insert(
                    key,
                    Contact {
                        pointer_id,
                        position,
                        input_context: input_context.clone(),
                    },
                );
                Ok(pointer_event(
                    pointer_id,
                    device_id,
                    PointerPhase::Down,
                    position,
                    LogicalDelta::ZERO,
                    input_context,
                ))
            }
            TouchPhase::Moved => {
                let contact = self
                    .contacts
                    .get_mut(&key)
                    .ok_or(TouchIngressDiagnostic::UnknownContact)?;
                let movement = LogicalDelta::new(
                    position.x() - contact.position.x(),
                    position.y() - contact.position.y(),
                )
                .map_err(|_| TouchIngressDiagnostic::MovementDeltaOutOfRange)?;
                contact.position = position;
                contact.input_context = input_context.clone();
                Ok(pointer_event(
                    contact.pointer_id,
                    device_id,
                    PointerPhase::Move,
                    position,
                    movement,
                    input_context,
                ))
            }
            TouchPhase::Ended | TouchPhase::Cancelled => {
                let contact = self
                    .contacts
                    .get(&key)
                    .ok_or(TouchIngressDiagnostic::UnknownContact)?;
                let movement = LogicalDelta::new(
                    position.x() - contact.position.x(),
                    position.y() - contact.position.y(),
                )
                .map_err(|_| TouchIngressDiagnostic::MovementDeltaOutOfRange)?;
                let pointer_id = contact.pointer_id;
                self.contacts.remove(&key);
                let neutral_phase = match phase {
                    TouchPhase::Ended => PointerPhase::Up,
                    TouchPhase::Cancelled => PointerPhase::Cancel,
                    _ => unreachable!("the terminal branch contains only terminal phases"),
                };
                Ok(pointer_event(
                    pointer_id,
                    device_id,
                    neutral_phase,
                    position,
                    movement,
                    input_context,
                ))
            }
        }
    }

    /// Cancels all still-live contacts using their last valid displayed context.
    pub fn cancel_all(&mut self) -> Vec<PointerEvent> {
        std::mem::take(&mut self.contacts)
            .into_iter()
            .map(|((device_id, _), contact)| {
                pointer_event(
                    contact.pointer_id,
                    device_id,
                    PointerPhase::Cancel,
                    contact.position,
                    LogicalDelta::ZERO,
                    contact.input_context,
                )
            })
            .collect()
    }

    #[must_use]
    pub fn active_contact_count(&self) -> usize {
        self.contacts.len()
    }
}

fn pointer_event(
    pointer_id: PointerId,
    device_id: InputDeviceId,
    phase: PointerPhase,
    position: LogicalPoint,
    movement: LogicalDelta,
    input_context: SurfaceInputContext,
) -> PointerEvent {
    let event = PointerEvent::new(
        pointer_id,
        PointerDeviceKind::Touch,
        phase,
        position,
        input_context,
    )
    .with_device_id(device_id)
    .with_movement_delta(movement);

    // Native contact phases use the same single-Primary neutral button protocol
    // that runtime pointer admission requires; Cancel closes the existing stream.
    match phase {
        PointerPhase::Down => event
            .with_buttons(PointerButtons::new([PointerButton::Primary]))
            .with_changed_button(PointerButton::Primary),
        PointerPhase::Move => event.with_buttons(PointerButtons::new([PointerButton::Primary])),
        PointerPhase::Up => event.with_changed_button(PointerButton::Primary),
        PointerPhase::Cancel => event,
        _ => unreachable!("native touch normalization produces only contact phases"),
    }
}

#[cfg(test)]
mod tests {
    use runenui_core::{
        InputDeviceId, LogicalPoint, NoHostProtocol, PointerButton, PointerDeviceKind,
        PointerEvent, PointerPhase, StyleEnvironment, SurfaceInputContext, UiApp, text,
    };
    use runenui_runtime::{
        AppRuntime, LayoutConstraints, PumpBudget, RuntimeConfig, SurfaceBuildContext, TraceConfig,
        TracePointerRejection, TraceRecordKind,
    };

    use super::{TouchIngressDiagnostic, TouchInputState};
    use winit::event::TouchPhase;

    struct App;

    impl UiApp for App {
        type State = ();
        type Action = ();
        type HostProtocol = NoHostProtocol;

        fn root((): &Self::State) -> impl runenui_core::View<Self::Action> {
            text("touch input proof")
        }

        fn update(
            (): &mut Self::State,
            (): Self::Action,
        ) -> impl runenui_core::IntoUpdateOutput<Self::Action, Self::HostProtocol> {
        }
    }

    fn point(x: f32, y: f32) -> LogicalPoint {
        LogicalPoint::new(x, y).unwrap_or_else(|_| unreachable!("test point is finite"))
    }

    #[test]
    fn contacts_keep_unique_pointer_identity_and_translate_terminal_cancellation() {
        let mut runtime = AppRuntime::<App>::mount(());
        let styles = StyleEnvironment::default();
        let surface = runtime
            .publish_surface(&SurfaceBuildContext::new(
                &styles,
                LayoutConstraints::unbounded(),
            ))
            .unwrap_or_else(|_| unreachable!("touch test surface publishes"));
        let context = surface.input_context().clone();
        let device = runenui_core::InputDeviceId::new(9)
            .unwrap_or_else(|| unreachable!("neutral input device id is nonzero"));
        let mut touch = TouchInputState::default();

        let down = touch
            .transition(
                device,
                42,
                TouchPhase::Started,
                point(1.0, 2.0),
                context.clone(),
            )
            .unwrap_or_else(|_| unreachable!("first contact starts"));
        let moved = touch
            .transition(
                device,
                42,
                TouchPhase::Moved,
                point(4.0, 8.0),
                context.clone(),
            )
            .unwrap_or_else(|_| unreachable!("active contact moves"));
        let duplicate = touch.transition(
            device,
            42,
            TouchPhase::Started,
            point(4.0, 8.0),
            context.clone(),
        );
        assert_eq!(duplicate, Err(TouchIngressDiagnostic::DuplicateStart));
        let cancel = touch.cancel_all();

        assert_eq!(down.device_kind(), runenui_core::PointerDeviceKind::Touch);
        assert_eq!(down.phase(), runenui_core::PointerPhase::Down);
        assert_eq!(
            down.pointer_id().get() % 2,
            0,
            "touch IDs occupy the even sequence"
        );
        assert_eq!(moved.pointer_id(), down.pointer_id());
        assert_eq!(moved.movement_delta().x().to_bits(), 3.0_f32.to_bits());
        assert_eq!(moved.movement_delta().y().to_bits(), 6.0_f32.to_bits());
        assert_eq!(touch.active_contact_count(), 0);
        assert_eq!(cancel.len(), 1);
        assert_eq!(cancel[0].phase(), runenui_core::PointerPhase::Cancel);
        assert_eq!(cancel[0].pointer_id(), down.pointer_id());
        assert_eq!(cancel[0].surface_context(), &context);
    }

    #[test]
    fn touch_pointer_ids_use_the_even_namespace_disjoint_from_mouse_adapter_ids() {
        let mut runtime = AppRuntime::<App>::mount(());
        let styles = StyleEnvironment::default();
        let surface = runtime
            .publish_surface(&SurfaceBuildContext::new(
                &styles,
                LayoutConstraints::unbounded(),
            ))
            .unwrap_or_else(|_| unreachable!("touch test surface publishes"));
        let device = runenui_core::InputDeviceId::new(10)
            .unwrap_or_else(|| unreachable!("neutral input device id is nonzero"));
        let event = TouchInputState::default()
            .transition(
                device,
                0,
                TouchPhase::Started,
                point(0.0, 0.0),
                surface.input_context().clone(),
            )
            .unwrap_or_else(|_| unreachable!("native contact ID zero is still valid"));
        assert_eq!(event.pointer_id().get() % 2, 0);
    }

    #[test]
    fn unrepresentable_terminal_delta_retains_contact_for_explicit_cancellation() {
        let mut runtime = AppRuntime::<App>::mount(());
        let styles = StyleEnvironment::default();
        let surface = runtime
            .publish_surface(&SurfaceBuildContext::new(
                &styles,
                LayoutConstraints::unbounded(),
            ))
            .unwrap_or_else(|_| unreachable!("touch test surface publishes"));
        let device = runenui_core::InputDeviceId::new(11)
            .unwrap_or_else(|| unreachable!("neutral input device ID is nonzero"));
        let context = surface.input_context().clone();
        let mut touch = TouchInputState::default();
        touch
            .transition(
                device,
                4,
                TouchPhase::Started,
                point(f32::MAX, 0.0),
                context,
            )
            .unwrap_or_else(|_| unreachable!("finite extreme contact coordinate is admitted"));

        assert_eq!(
            touch.transition(
                device,
                4,
                TouchPhase::Ended,
                point(-f32::MAX, 0.0),
                surface.input_context().clone(),
            ),
            Err(TouchIngressDiagnostic::MovementDeltaOutOfRange)
        );
        assert_eq!(touch.active_contact_count(), 1);
        assert_eq!(touch.cancel_all().len(), 1);
    }

    fn traced_runtime_and_context() -> (AppRuntime<App>, SurfaceInputContext) {
        let config = RuntimeConfig::default().with_trace_config(TraceConfig::new(1024));
        let mut runtime = AppRuntime::<App>::mount_with_config((), config);
        let styles = StyleEnvironment::default();
        let publication = runtime
            .publish_surface(&SurfaceBuildContext::new(
                &styles,
                LayoutConstraints::unbounded(),
            ))
            .unwrap_or_else(|_| unreachable!("touch test surface publishes"));
        (runtime, publication.input_context().clone())
    }

    fn submit_and_pump(runtime: &mut AppRuntime<App>, event: PointerEvent) {
        runtime
            .submit_pointer(event)
            .unwrap_or_else(|_| unreachable!("the neutral pointer queue admits touch work"));
        let _ = runtime.pump(PumpBudget::new(
            usize::MAX,
            usize::MAX,
            usize::MAX,
            usize::MAX,
        )).expect("pump observation").report().to_owned();
    }

    #[test]
    fn native_touch_phases_have_exact_primary_contact_button_facts() {
        let (_, context) = traced_runtime_and_context();
        let device =
            InputDeviceId::new(21).unwrap_or_else(|| unreachable!("fixture device is nonzero"));
        let mut touch = TouchInputState::default();

        let down = touch
            .transition(
                device,
                7,
                TouchPhase::Started,
                point(1.0, 2.0),
                context.clone(),
            )
            .unwrap_or_else(|_| unreachable!("contact starts"));
        assert_eq!(down.phase(), PointerPhase::Down);
        assert_eq!(down.changed_button(), Some(PointerButton::Primary));
        assert_eq!(
            down.buttons().iter().collect::<Vec<_>>(),
            [PointerButton::Primary]
        );
        assert_eq!(down.device_kind(), PointerDeviceKind::Touch);
        assert_eq!(down.device_id(), Some(device));

        let moved = touch
            .transition(
                device,
                7,
                TouchPhase::Moved,
                point(3.0, 4.0),
                context.clone(),
            )
            .unwrap_or_else(|_| unreachable!("contact moves"));
        assert_eq!(moved.phase(), PointerPhase::Move);
        assert_eq!(moved.changed_button(), None);
        assert_eq!(
            moved.buttons().iter().collect::<Vec<_>>(),
            [PointerButton::Primary]
        );

        let up = touch
            .transition(
                device,
                7,
                TouchPhase::Ended,
                point(3.0, 4.0),
                context.clone(),
            )
            .unwrap_or_else(|_| unreachable!("contact ends"));
        assert_eq!(up.phase(), PointerPhase::Up);
        assert_eq!(up.changed_button(), Some(PointerButton::Primary));
        assert!(up.buttons().is_empty());

        let _ = touch
            .transition(
                device,
                8,
                TouchPhase::Started,
                point(5.0, 6.0),
                context.clone(),
            )
            .unwrap_or_else(|_| unreachable!("second contact starts"));
        let cancel = touch
            .transition(device, 8, TouchPhase::Cancelled, point(5.0, 6.0), context)
            .unwrap_or_else(|_| unreachable!("second contact cancels"));
        assert_eq!(cancel.phase(), PointerPhase::Cancel);
        assert_eq!(cancel.changed_button(), None);
        assert!(cancel.buttons().is_empty());
        assert_eq!(touch.active_contact_count(), 0);
    }

    #[test]
    fn translated_concurrent_native_contacts_reach_runtime_without_profile_rejection() {
        let (mut runtime, context) = traced_runtime_and_context();
        let device =
            InputDeviceId::new(22).unwrap_or_else(|| unreachable!("fixture device is nonzero"));
        let mut touch = TouchInputState::default();
        let mut started_ids = Vec::new();

        for (contact, phase, position) in [
            (10, TouchPhase::Started, point(1.0, 2.0)),
            (11, TouchPhase::Started, point(4.0, 5.0)),
            (10, TouchPhase::Moved, point(2.0, 3.0)),
            (11, TouchPhase::Moved, point(5.0, 6.0)),
            (10, TouchPhase::Ended, point(2.0, 3.0)),
            (11, TouchPhase::Cancelled, point(5.0, 6.0)),
            (12, TouchPhase::Started, point(7.0, 8.0)),
        ] {
            let event = touch
                .transition(device, contact, phase, position, context.clone())
                .unwrap_or_else(|_| unreachable!("native contact transition is valid"));
            if matches!(phase, TouchPhase::Started) {
                started_ids.push(event.pointer_id().get());
            }
            submit_and_pump(&mut runtime, event);
        }

        assert_eq!(touch.active_contact_count(), 1);
        for event in touch.cancel_all() {
            assert_eq!(event.phase(), PointerPhase::Cancel);
            submit_and_pump(&mut runtime, event);
        }
        assert_eq!(touch.active_contact_count(), 0);

        let mut registered = Vec::new();
        let mut closed = Vec::new();
        for record in runtime.trace().records() {
            match record.kind() {
                TraceRecordKind::PointerIngressRejected { .. } => {
                    unreachable!("all native touch transitions must satisfy runtime admission");
                }
                TraceRecordKind::PointerStreamRegistered { pointer_id, .. } => {
                    registered.push(pointer_id.get());
                }
                TraceRecordKind::PointerStreamClosed { pointer_id } => {
                    closed.push(pointer_id.get());
                }
                _ => {}
            }
        }
        started_ids.sort_unstable();
        registered.sort_unstable();
        closed.sort_unstable();
        assert_eq!(registered, started_ids);
        assert_eq!(closed, started_ids);
    }

    #[test]
    fn buttonless_touch_dialect_rejects_without_registering_a_pointer_stream() {
        let (mut runtime, context) = traced_runtime_and_context();
        let device =
            InputDeviceId::new(23).unwrap_or_else(|| unreachable!("fixture device is nonzero"));
        let mut touch = TouchInputState::default();
        let valid = touch
            .transition(device, 16, TouchPhase::Started, point(1.0, 2.0), context)
            .unwrap_or_else(|_| unreachable!("native contact starts"));
        let invalid = PointerEvent::new(
            valid.pointer_id(),
            PointerDeviceKind::Touch,
            PointerPhase::Down,
            valid.position(),
            valid.surface_context().clone(),
        )
        .with_device_id(device);

        submit_and_pump(&mut runtime, invalid);
        assert!(runtime.trace().records().any(|record| matches!(
            record.kind(),
            TraceRecordKind::PointerIngressRejected {
                pointer_id,
                phase: PointerPhase::Down,
                outcome: TracePointerRejection::TouchProfileUnsupported,
            } if *pointer_id == valid.pointer_id()
        )));
        assert!(!runtime.trace().records().any(|record| matches!(
            record.kind(),
            TraceRecordKind::PointerStreamRegistered { pointer_id, .. }
                if *pointer_id == valid.pointer_id()
        )));

        submit_and_pump(&mut runtime, valid);
        assert!(runtime.trace().records().any(|record| matches!(
            record.kind(),
            TraceRecordKind::PointerStreamRegistered { .. }
        )));
    }
}
