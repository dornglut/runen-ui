//! Host-neutral pointer, device, and logical-scroll protocol values.

use core::{
    error::Error,
    fmt,
    hash::{Hash, Hasher},
    num::NonZeroU64,
};

use crate::{MountedNodeId, SurfaceInputContext};

#[derive(Clone, Copy, Debug, PartialEq)]
struct FiniteLogical(f32);

impl FiniteLogical {
    const fn new(value: f32) -> Option<Self> {
        if value.is_nan() || value == f32::INFINITY || value == f32::NEG_INFINITY {
            None
        } else if value == 0.0 {
            Some(Self(0.0))
        } else {
            Some(Self(value))
        }
    }

    const fn get(self) -> f32 {
        self.0
    }
}

impl Eq for FiniteLogical {}

impl Hash for FiniteLogical {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.0.to_bits().hash(state);
    }
}

/// Error returned for a non-finite logical point coordinate.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LogicalPointError;

impl fmt::Display for LogicalPointError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("logical point coordinates must be finite")
    }
}

impl Error for LogicalPointError {}

/// Error returned for a non-finite logical delta component.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct LogicalDeltaError;

impl fmt::Display for LogicalDeltaError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("logical delta components must be finite")
    }
}

impl Error for LogicalDeltaError {}

/// Finite position in `RunenUI` logical coordinate space.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct LogicalPoint {
    x: FiniteLogical,
    y: FiniteLogical,
}

impl LogicalPoint {
    /// Creates a finite logical point.
    ///
    /// # Errors
    ///
    /// Returns [`LogicalPointError`] when either coordinate is non-finite.
    pub const fn new(x: f32, y: f32) -> Result<Self, LogicalPointError> {
        let Some(x) = FiniteLogical::new(x) else {
            return Err(LogicalPointError);
        };
        let Some(y) = FiniteLogical::new(y) else {
            return Err(LogicalPointError);
        };
        Ok(Self { x, y })
    }

    /// Returns the horizontal coordinate.
    #[must_use]
    pub const fn x(self) -> f32 {
        self.x.get()
    }

    /// Returns the vertical coordinate.
    #[must_use]
    pub const fn y(self) -> f32 {
        self.y.get()
    }
}

/// Finite movement or scrolling delta in logical coordinates.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct LogicalDelta {
    x: FiniteLogical,
    y: FiniteLogical,
}

impl LogicalDelta {
    /// Zero logical movement.
    pub const ZERO: Self = Self {
        x: FiniteLogical(0.0),
        y: FiniteLogical(0.0),
    };

    /// Creates a finite logical delta.
    ///
    /// # Errors
    ///
    /// Returns [`LogicalDeltaError`] when either component is non-finite.
    pub const fn new(x: f32, y: f32) -> Result<Self, LogicalDeltaError> {
        let Some(x) = FiniteLogical::new(x) else {
            return Err(LogicalDeltaError);
        };
        let Some(y) = FiniteLogical::new(y) else {
            return Err(LogicalDeltaError);
        };
        Ok(Self { x, y })
    }

    /// Returns the horizontal component.
    #[must_use]
    pub const fn x(self) -> f32 {
        self.x.get()
    }

    /// Returns the vertical component.
    #[must_use]
    pub const fn y(self) -> f32 {
        self.y.get()
    }
}

impl Default for FiniteLogical {
    fn default() -> Self {
        Self(0.0)
    }
}

/// Keyboard modifier state shared by neutral input families.
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq)]
pub struct KeyModifiers {
    bits: u8,
}

impl KeyModifiers {
    const SHIFT_BIT: u8 = 0b0001;
    const CONTROL_BIT: u8 = 0b0010;
    const ALT_BIT: u8 = 0b0100;
    const META_BIT: u8 = 0b1000;
    const ALL_BITS: u8 = Self::SHIFT_BIT | Self::CONTROL_BIT | Self::ALT_BIT | Self::META_BIT;

    /// Empty modifier state.
    pub const NONE: Self = Self { bits: 0 };
    /// Shift modifier state.
    pub const SHIFT: Self = Self {
        bits: Self::SHIFT_BIT,
    };
    /// Control modifier state.
    pub const CONTROL: Self = Self {
        bits: Self::CONTROL_BIT,
    };
    /// Alt modifier state.
    pub const ALT: Self = Self {
        bits: Self::ALT_BIT,
    };
    /// Meta, Command, or Windows modifier state.
    pub const META: Self = Self {
        bits: Self::META_BIT,
    };

    /// Creates a modifier state while discarding unknown bits.
    #[must_use]
    pub const fn from_bits(bits: u8) -> Self {
        Self {
            bits: bits & Self::ALL_BITS,
        }
    }

    /// Returns the normalized modifier bits.
    #[must_use]
    pub const fn bits(self) -> u8 {
        self.bits
    }

    /// Returns a state with Shift added.
    #[must_use]
    pub const fn with_shift(self) -> Self {
        Self::from_bits(self.bits | Self::SHIFT_BIT)
    }

    /// Returns a state with Control added.
    #[must_use]
    pub const fn with_control(self) -> Self {
        Self::from_bits(self.bits | Self::CONTROL_BIT)
    }

    /// Returns a state with Alt added.
    #[must_use]
    pub const fn with_alt(self) -> Self {
        Self::from_bits(self.bits | Self::ALT_BIT)
    }

    /// Returns a state with Meta added.
    #[must_use]
    pub const fn with_meta(self) -> Self {
        Self::from_bits(self.bits | Self::META_BIT)
    }

    /// Returns whether Shift is active.
    #[must_use]
    pub const fn shift(self) -> bool {
        self.bits & Self::SHIFT_BIT != 0
    }

    /// Returns whether Control is active.
    #[must_use]
    pub const fn control(self) -> bool {
        self.bits & Self::CONTROL_BIT != 0
    }

    /// Returns whether Alt is active.
    #[must_use]
    pub const fn alt(self) -> bool {
        self.bits & Self::ALT_BIT != 0
    }

    /// Returns whether Meta is active.
    #[must_use]
    pub const fn meta(self) -> bool {
        self.bits & Self::META_BIT != 0
    }
}

macro_rules! session_identity {
    ($name:ident, $description:literal) => {
        #[doc = $description]
        #[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
        pub struct $name(NonZeroU64);

        impl $name {
            /// Creates a non-zero host-session identity.
            #[must_use]
            pub const fn new(value: u64) -> Option<Self> {
                match NonZeroU64::new(value) {
                    Some(value) => Some(Self(value)),
                    None => None,
                }
            }

            /// Returns the host-session numeric identity.
            #[must_use]
            pub const fn get(self) -> u64 {
                self.0.get()
            }
        }
    };
}

session_identity!(
    PointerId,
    "Opaque host-session identity for one pointer stream from entry through final button release or cancellation."
);
session_identity!(
    InputDeviceId,
    "Optional opaque host-session identity for one neutral input device."
);

/// Neutral pointer device category.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum PointerDeviceKind {
    Mouse,
    Touch,
    Pen,
    Other,
}

/// Pointer button identity.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum PointerButton {
    Primary,
    Secondary,
    Middle,
    Other(u16),
}

/// Deterministic normalized set of currently active pointer buttons.
#[derive(Clone, Debug, Default, Eq, Hash, PartialEq)]
pub struct PointerButtons {
    buttons: Vec<PointerButton>,
}

impl PointerButtons {
    /// Creates a sorted, duplicate-free button set.
    #[must_use]
    pub fn new(buttons: impl IntoIterator<Item = PointerButton>) -> Self {
        let mut buttons = buttons.into_iter().collect::<Vec<_>>();
        buttons.sort_unstable();
        buttons.dedup();
        Self { buttons }
    }

    /// Returns whether this set is empty.
    #[must_use]
    pub const fn is_empty(&self) -> bool {
        self.buttons.is_empty()
    }

    /// Returns whether a button is active.
    #[must_use]
    pub fn contains(&self, button: PointerButton) -> bool {
        self.buttons.binary_search(&button).is_ok()
    }

    /// Iterates active buttons in deterministic order.
    #[must_use]
    pub fn iter(&self) -> impl ExactSizeIterator<Item = PointerButton> + '_ {
        self.buttons.iter().copied()
    }
}

/// Pointer ingress phase.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum PointerPhase {
    Down,
    Move,
    Up,
    Cancel,
    Wheel,
}

/// Complete host-neutral pointer ingress payload.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct PointerEvent {
    pointer_id: PointerId,
    device_id: Option<InputDeviceId>,
    device_kind: PointerDeviceKind,
    phase: PointerPhase,
    position: LogicalPoint,
    movement_delta: LogicalDelta,
    scroll_delta: LogicalDelta,
    buttons: PointerButtons,
    changed_button: Option<PointerButton>,
    modifiers: KeyModifiers,
    surface_context: SurfaceInputContext,
}

impl PointerEvent {
    /// Creates a pointer event with empty optional facts and zero deltas.
    #[must_use]
    pub fn new(
        pointer_id: PointerId,
        device_kind: PointerDeviceKind,
        phase: PointerPhase,
        position: LogicalPoint,
        surface_context: SurfaceInputContext,
    ) -> Self {
        Self {
            pointer_id,
            device_id: None,
            device_kind,
            phase,
            position,
            movement_delta: LogicalDelta::ZERO,
            scroll_delta: LogicalDelta::ZERO,
            buttons: PointerButtons::default(),
            changed_button: None,
            modifiers: KeyModifiers::NONE,
            surface_context,
        }
    }

    #[must_use]
    pub const fn with_device_id(mut self, device_id: InputDeviceId) -> Self {
        self.device_id = Some(device_id);
        self
    }

    #[must_use]
    pub const fn with_movement_delta(mut self, delta: LogicalDelta) -> Self {
        self.movement_delta = delta;
        self
    }

    #[must_use]
    pub const fn with_scroll_delta(mut self, delta: LogicalDelta) -> Self {
        self.scroll_delta = delta;
        self
    }

    /// Replaces the complete active-button snapshot carried by this event.
    #[must_use]
    pub fn with_buttons(mut self, buttons: PointerButtons) -> Self {
        self.buttons = buttons;
        self
    }

    /// Sets the button whose state changed for a down or up event.
    #[must_use]
    pub const fn with_changed_button(mut self, button: PointerButton) -> Self {
        self.changed_button = Some(button);
        self
    }

    #[must_use]
    pub const fn with_modifiers(mut self, modifiers: KeyModifiers) -> Self {
        self.modifiers = modifiers;
        self
    }

    #[must_use]
    pub const fn pointer_id(&self) -> PointerId {
        self.pointer_id
    }

    #[must_use]
    pub const fn device_id(&self) -> Option<InputDeviceId> {
        self.device_id
    }

    #[must_use]
    pub const fn device_kind(&self) -> PointerDeviceKind {
        self.device_kind
    }

    #[must_use]
    pub const fn phase(&self) -> PointerPhase {
        self.phase
    }

    #[must_use]
    pub const fn position(&self) -> LogicalPoint {
        self.position
    }

    #[must_use]
    pub const fn movement_delta(&self) -> LogicalDelta {
        self.movement_delta
    }

    #[must_use]
    pub const fn scroll_delta(&self) -> LogicalDelta {
        self.scroll_delta
    }

    /// Returns the complete active-button snapshot after this event's transition.
    #[must_use]
    pub const fn buttons(&self) -> &PointerButtons {
        &self.buttons
    }

    /// Returns the button whose state changed for a down or up event, when any.
    #[must_use]
    pub const fn changed_button(&self) -> Option<PointerButton> {
        self.changed_button
    }

    #[must_use]
    pub const fn modifiers(&self) -> KeyModifiers {
        self.modifiers
    }

    #[must_use]
    pub const fn surface_context(&self) -> &SurfaceInputContext {
        &self.surface_context
    }
}

/// Pointer boundary transition kind.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum PointerBoundaryKind {
    Enter,
    Leave,
}

/// Runtime-derived target-only pointer boundary payload.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct PointerBoundaryEvent {
    pointer_id: PointerId,
    kind: PointerBoundaryKind,
    target: MountedNodeId,
    related_target: Option<MountedNodeId>,
    surface_context: SurfaceInputContext,
}

impl PointerBoundaryEvent {
    #[doc(hidden)]
    #[must_use]
    pub const fn __runtime_new(
        pointer_id: PointerId,
        kind: PointerBoundaryKind,
        target: MountedNodeId,
        related_target: Option<MountedNodeId>,
        surface_context: SurfaceInputContext,
    ) -> Self {
        Self {
            pointer_id,
            kind,
            target,
            related_target,
            surface_context,
        }
    }

    #[must_use]
    pub const fn pointer_id(&self) -> PointerId {
        self.pointer_id
    }

    #[must_use]
    pub const fn kind(&self) -> PointerBoundaryKind {
        self.kind
    }

    #[must_use]
    pub const fn target(&self) -> &MountedNodeId {
        &self.target
    }

    #[must_use]
    pub const fn related_target(&self) -> Option<&MountedNodeId> {
        self.related_target.as_ref()
    }

    #[must_use]
    pub const fn surface_context(&self) -> &SurfaceInputContext {
        &self.surface_context
    }
}

/// Pointer capture transition kind.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum PointerCaptureKind {
    Gained,
    Lost,
}

/// Runtime-derived target-only pointer capture payload.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct PointerCaptureEvent {
    pointer_id: PointerId,
    kind: PointerCaptureKind,
    target: MountedNodeId,
    related_owner: Option<MountedNodeId>,
    surface_context: SurfaceInputContext,
}

impl PointerCaptureEvent {
    #[doc(hidden)]
    #[must_use]
    pub const fn __runtime_new(
        pointer_id: PointerId,
        kind: PointerCaptureKind,
        target: MountedNodeId,
        related_owner: Option<MountedNodeId>,
        surface_context: SurfaceInputContext,
    ) -> Self {
        Self {
            pointer_id,
            kind,
            target,
            related_owner,
            surface_context,
        }
    }

    #[must_use]
    pub const fn pointer_id(&self) -> PointerId {
        self.pointer_id
    }

    #[must_use]
    pub const fn kind(&self) -> PointerCaptureKind {
        self.kind
    }

    #[must_use]
    pub const fn target(&self) -> &MountedNodeId {
        &self.target
    }

    #[must_use]
    pub const fn related_owner(&self) -> Option<&MountedNodeId> {
        self.related_owner.as_ref()
    }

    #[must_use]
    pub const fn surface_context(&self) -> &SurfaceInputContext {
        &self.surface_context
    }
}

/// Device-independent route-only logical scrolling intent.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct LogicalScrollCommand {
    pointer_id: PointerId,
    delta: LogicalDelta,
}

impl LogicalScrollCommand {
    #[doc(hidden)]
    #[must_use]
    pub const fn __runtime_new(pointer_id: PointerId, delta: LogicalDelta) -> Self {
        Self { pointer_id, delta }
    }

    #[must_use]
    pub const fn pointer_id(self) -> PointerId {
        self.pointer_id
    }

    #[must_use]
    pub const fn delta(self) -> LogicalDelta {
        self.delta
    }
}

#[cfg(test)]
mod tests {
    use super::{
        InputDeviceId, KeyModifiers, LogicalDelta, LogicalPoint, PointerButton, PointerButtons,
        PointerId,
    };

    #[test]
    fn logical_values_reject_non_finite_input_and_canonicalize_zero() {
        assert!(LogicalPoint::new(f32::NAN, 0.0).is_err());
        assert!(LogicalDelta::new(0.0, f32::INFINITY).is_err());
        assert_eq!(LogicalPoint::new(-0.0, 0.0), LogicalPoint::new(0.0, -0.0));
    }

    #[test]
    fn identities_are_checked_non_zero_session_values() {
        assert_eq!(PointerId::new(0), None);
        assert_eq!(InputDeviceId::new(0), None);
        assert_eq!(PointerId::new(7).map(PointerId::get), Some(7));
    }

    #[test]
    fn buttons_and_modifiers_are_normalized() {
        let buttons = PointerButtons::new([
            PointerButton::Secondary,
            PointerButton::Primary,
            PointerButton::Primary,
        ]);
        assert_eq!(buttons.iter().count(), 2);
        assert!(buttons.contains(PointerButton::Primary));
        let modifiers = KeyModifiers::NONE.with_shift().with_control();
        assert!(modifiers.shift());
        assert!(modifiers.control());
        assert!(!modifiers.alt());
    }
}
