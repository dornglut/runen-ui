pub mod composition;
pub mod event;
pub mod facts;
pub mod focus;
pub mod keyboard;
pub mod pointer;
pub mod routing;
pub mod selection;
pub mod semantic;
pub mod shortcut;
pub mod text;

pub use composition::*;
pub use event::*;
pub use facts::*;
pub use focus::*;
pub use keyboard::*;
pub use pointer::*;
pub use routing::*;
pub use selection::*;
pub use semantic::*;
pub use shortcut::*;
pub use text::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn input_response_helpers_preserve_default_routing_contracts() {
        let ignored = InputResponse::ignored();
        assert_eq!(ignored.propagation, EventPropagation::Continue);
        assert_eq!(ignored.capture, PointerCapture::None);
        assert_eq!(ignored.focus_change, FocusChange::None);
        assert!(!ignored.repaint);
        assert!(!ignored.relayout);

        let handled = InputResponse::handled();
        assert_eq!(handled.propagation, EventPropagation::Stop);
        assert_eq!(handled.capture, PointerCapture::None);
        assert_eq!(handled.focus_change, FocusChange::None);
        assert!(!handled.repaint);
        assert!(!handled.relayout);
    }

    #[test]
    fn pointer_events_default_to_mouse_fallback_packets() {
        let event = PointerEvent::new(
            PointerEventKind::Move,
            PointerPosition::new(8.0, 13.0),
            PointerDelta::ZERO,
            None,
            Modifiers::default(),
            0,
        );

        assert_eq!(event.packet.source_kind, PointerSourceKind::Mouse);
        assert_eq!(event.packet.tool_kind, PointerToolKind::Mouse);
        assert!(event.packet.is_pointer_fallback());
        assert!(event.packet.is_valid());
    }

    #[test]
    fn stylus_packets_preserve_pressure_tilt_twist_and_samples() {
        let coalesced = PointerSample::new(
            PointerSampleRole::Coalesced,
            PointerPosition::new(10.0, 12.0),
            PointerDelta::new(1.0, 2.0),
        )
        .with_timestamp_micros(9_900)
        .with_pressure(0.5)
        .with_tilt(PointerTilt::new(-12.0, 18.0));
        let predicted = PointerSample::new(
            PointerSampleRole::Predicted,
            PointerPosition::new(13.0, 16.0),
            PointerDelta::new(3.0, 4.0),
        )
        .with_timestamp_micros(10_200)
        .with_pressure(0.65)
        .with_contact(PointerContactState::Hover);
        let packet = PointerPacket::stylus(PointerDeviceId(42), PointerToolKind::Pen)
            .with_timestamp_micros(10_000)
            .with_pressure(0.625)
            .with_tilt(PointerTilt::new(20.0, -15.0))
            .with_twist_degrees(270.0)
            .with_tangential_pressure(0.25)
            .with_barrel_buttons(PointerBarrelButtons {
                primary: true,
                secondary: false,
            })
            .with_contact(PointerContactState::Hover)
            .with_latency_class(PointerLatencyClass::LowLatencyPreview)
            .with_coalesced_samples([coalesced])
            .with_predicted_samples([predicted]);

        assert_eq!(packet.source_kind, PointerSourceKind::Stylus);
        assert_eq!(packet.device_id, Some(PointerDeviceId(42)));
        assert_eq!(packet.timestamp_micros, Some(10_000));
        assert_eq!(packet.pressure, Some(0.625));
        assert_eq!(packet.tilt, Some(PointerTilt::new(20.0, -15.0)));
        assert_eq!(packet.twist_degrees, Some(270.0));
        assert!(packet.barrel_buttons.primary);
        assert_eq!(packet.contact, PointerContactState::Hover);
        assert_eq!(packet.latency_class, PointerLatencyClass::LowLatencyPreview);
        assert_eq!(packet.coalesced_samples, vec![coalesced]);
        assert_eq!(packet.predicted_samples, vec![predicted]);
        assert!(packet.capabilities.pressure);
        assert!(packet.capabilities.tilt);
        assert!(packet.capabilities.twist);
        assert!(packet.capabilities.hover);
        assert!(packet.capabilities.barrel_buttons);
        assert!(packet.capabilities.coalesced_samples);
        assert!(packet.capabilities.predicted_samples);
        assert!(packet.is_valid());
        assert!(!packet.is_pointer_fallback());
    }

    #[test]
    fn missing_stylus_capabilities_are_explicit_and_valid() {
        let packet = PointerPacket::stylus(PointerDeviceId(7), PointerToolKind::Pen)
            .with_capabilities(PointerDeviceCapabilities {
                pressure: false,
                tilt: false,
                hover: true,
                ..PointerDeviceCapabilities::default()
            });
        assert_eq!(packet.source_kind, PointerSourceKind::Stylus);
        assert_eq!(packet.device_id, Some(PointerDeviceId(7)));
        assert!(!packet.capabilities.pressure);
        assert!(!packet.capabilities.tilt);
        assert!(packet.capabilities.hover);
        assert_eq!(packet.pressure, None);
        assert_eq!(packet.tilt, None);
        assert!(packet.is_valid());
    }

    #[test]
    fn invalid_pointer_packets_are_rejected_by_contract_helpers() {
        let invalid_pressure =
            PointerPacket::stylus(PointerDeviceId(1), PointerToolKind::Pen).with_pressure(1.25);
        let invalid_tilt = PointerPacket::stylus(PointerDeviceId(1), PointerToolKind::Pen)
            .with_tilt(PointerTilt::new(120.0, 0.0));
        let wrong_sample_role = PointerPacket::stylus(PointerDeviceId(1), PointerToolKind::Pen)
            .with_coalesced_samples([PointerSample::new(
                PointerSampleRole::Raw,
                PointerPosition::new(0.0, 0.0),
                PointerDelta::ZERO,
            )]);
        assert!(!invalid_pressure.is_valid());
        assert!(!invalid_tilt.is_valid());
        assert!(!wrong_sample_role.is_valid());
    }

    #[test]
    fn eraser_packets_route_as_stylus_eraser_tool() {
        let packet =
            PointerPacket::stylus(PointerDeviceId(99), PointerToolKind::Pen).with_eraser(true);
        assert!(packet.eraser);
        assert_eq!(packet.tool_kind, PointerToolKind::Eraser);
        assert!(packet.capabilities.eraser);
        assert!(packet.is_valid());
    }
}
