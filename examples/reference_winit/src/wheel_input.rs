use runenui_core::LogicalDelta;
use winit::{
    event::{DeviceId, MouseScrollDelta},
    event_loop::ActiveEventLoop,
};

use crate::{PointIngressDiagnostic, ReferenceHost};

/// Standalone reference-host policy for one native line/row scroll unit.
///
/// This is edge UX policy, not `RunenUI` protocol. The neutral runtime receives only
/// logical-coordinate deltas and another host may choose a different line metric.
const REFERENCE_LINE_SCROLL_LOGICAL_UNITS: f64 = 60.0;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum WheelIngressDiagnostic {
    InvalidDisplayedScale,
    ScrollDeltaOutOfRange,
}

#[must_use]
fn native_wheel_delta_is_zero(delta: &MouseScrollDelta) -> bool {
    match delta {
        MouseScrollDelta::LineDelta(x, y) => *x == 0.0 && *y == 0.0,
        MouseScrollDelta::PixelDelta(delta) => delta.x == 0.0 && delta.y == 0.0,
    }
}

#[allow(
    clippy::cast_possible_truncation,
    reason = "native f64 wheel deltas are finite and f32-range-checked before conversion into RunenUI logical coordinates"
)]
fn normalize_wheel_delta(
    delta: MouseScrollDelta,
    displayed_scale_factor: f64,
) -> Result<LogicalDelta, WheelIngressDiagnostic> {
    let (logical_x, logical_y) = match delta {
        MouseScrollDelta::LineDelta(x, y) => (
            f64::from(x) * REFERENCE_LINE_SCROLL_LOGICAL_UNITS,
            f64::from(y) * REFERENCE_LINE_SCROLL_LOGICAL_UNITS,
        ),
        MouseScrollDelta::PixelDelta(delta) => {
            if !displayed_scale_factor.is_finite() || displayed_scale_factor <= 0.0 {
                return Err(WheelIngressDiagnostic::InvalidDisplayedScale);
            }
            (
                delta.x / displayed_scale_factor,
                delta.y / displayed_scale_factor,
            )
        }
    };

    if !logical_x.is_finite()
        || !logical_y.is_finite()
        || logical_x < f64::from(f32::MIN)
        || logical_x > f64::from(f32::MAX)
        || logical_y < f64::from(f32::MIN)
        || logical_y > f64::from(f32::MAX)
    {
        return Err(WheelIngressDiagnostic::ScrollDeltaOutOfRange);
    }

    LogicalDelta::new(logical_x as f32, logical_y as f32)
        .map_err(|_| WheelIngressDiagnostic::ScrollDeltaOutOfRange)
}

pub fn handle_mouse_wheel(
    host: &mut ReferenceHost,
    event_loop: &ActiveEventLoop,
    native_device_id: DeviceId,
    delta: MouseScrollDelta,
) {
    // Winit reports gesture phase separately from displacement. RunenUI's accepted
    // wheel protocol carries logical displacement rather than native gesture phase,
    // so phase-only zero-delta notifications must be inert before consulting any
    // device, cursor, or displayed-frame authority.
    if native_wheel_delta_is_zero(&delta) {
        return;
    }

    let Some(device_id) = host.resolve_native_device_id(event_loop, native_device_id) else {
        return;
    };

    let translated = match host.translate_latest_cursor() {
        Ok(translated) => translated,
        Err(diagnostic) => {
            host.note_point_ingress_diagnostic(diagnostic);
            host.handle_native_point_authority_loss(
                event_loop,
                "native wheel arrived without matching point authority",
            );
            return;
        }
    };
    let Some(displayed_mapping) = host.displayed_frame.as_ref().map(|frame| frame.mapping) else {
        host.note_point_ingress_diagnostic(PointIngressDiagnostic::NoDisplayedFrame);
        return;
    };
    let scroll_delta = match normalize_wheel_delta(delta, displayed_mapping.native_scale_factor) {
        Ok(delta) => delta,
        Err(diagnostic) => {
            eprintln!("reference_winit wheel ingress withheld: {diagnostic:?}");
            return;
        }
    };

    if host
        .mouse
        .active_device_id()
        .is_some_and(|active| active != device_id)
        && !host.cancel_mouse_for_device_change(event_loop, "native mouse wheel device changed")
    {
        return;
    }

    let event = match host.mouse.wheel(device_id, translated, scroll_delta) {
        Ok(event) => event,
        Err(diagnostic) => {
            host.fail(
                event_loop,
                &format!("native mouse wheel could not be represented: {diagnostic:?}"),
            );
            return;
        }
    };
    host.last_point_ingress_diagnostic = None;
    host.last_mouse_ingress_diagnostic = None;
    if !host.submit_pointer_event(event_loop, event, "native mouse wheel translation") {
        return;
    }
    host.drive_runtime(event_loop);
    host.request_pending_redraw();
}

#[cfg(test)]
mod tests {
    use super::{WheelIngressDiagnostic, native_wheel_delta_is_zero, normalize_wheel_delta};
    use runenui_core::LogicalDelta;
    use winit::{dpi::PhysicalPosition, event::MouseScrollDelta};

    #[test]
    fn line_delta_uses_explicit_reference_host_logical_step() {
        let delta = normalize_wheel_delta(MouseScrollDelta::LineDelta(1.5, -2.0), 3.0)
            .unwrap_or_else(|_| unreachable!("fixture line delta is representable"));
        assert!((delta.x() - 90.0).abs() < f32::EPSILON);
        assert!((delta.y() + 120.0).abs() < f32::EPSILON);
    }

    #[test]
    fn pixel_delta_uses_exact_displayed_native_scale() {
        let delta = normalize_wheel_delta(
            MouseScrollDelta::PixelDelta(PhysicalPosition::new(120.0, -40.0)),
            2.0,
        )
        .unwrap_or_else(|_| unreachable!("fixture pixel delta is representable"));
        assert!((delta.x() - 60.0).abs() < f32::EPSILON);
        assert!((delta.y() + 20.0).abs() < f32::EPSILON);
    }

    #[test]
    fn phase_only_zero_delta_is_detected_before_authority_checks() {
        assert!(native_wheel_delta_is_zero(&MouseScrollDelta::LineDelta(
            0.0, -0.0,
        )));
        assert!(native_wheel_delta_is_zero(&MouseScrollDelta::PixelDelta(
            PhysicalPosition::new(-0.0, 0.0),
        )));
        assert!(!native_wheel_delta_is_zero(&MouseScrollDelta::LineDelta(
            0.0, 1.0,
        )));
        assert!(!native_wheel_delta_is_zero(&MouseScrollDelta::PixelDelta(
            PhysicalPosition::new(1.0, 0.0),
        )));
    }

    #[test]
    fn zero_native_delta_remains_zero_logical_delta() {
        assert_eq!(
            normalize_wheel_delta(MouseScrollDelta::LineDelta(0.0, 0.0), 1.0),
            Ok(LogicalDelta::ZERO)
        );
    }

    #[test]
    fn wheel_normalization_rejects_invalid_or_unrepresentable_native_values() {
        assert_eq!(
            normalize_wheel_delta(
                MouseScrollDelta::PixelDelta(PhysicalPosition::new(1.0, 1.0)),
                0.0,
            ),
            Err(WheelIngressDiagnostic::InvalidDisplayedScale)
        );
        assert_eq!(
            normalize_wheel_delta(
                MouseScrollDelta::PixelDelta(PhysicalPosition::new(f64::NAN, 0.0)),
                1.0,
            ),
            Err(WheelIngressDiagnostic::ScrollDeltaOutOfRange)
        );
        assert_eq!(
            normalize_wheel_delta(MouseScrollDelta::LineDelta(f32::MAX, 0.0), 1.0),
            Err(WheelIngressDiagnostic::ScrollDeltaOutOfRange)
        );
    }
}
