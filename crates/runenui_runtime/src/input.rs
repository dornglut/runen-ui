//! Runtime input vocabulary.

use core::{error::Error, fmt};

use crate::{MountedNodeId, SurfaceFrame};

/// Error returned for a non-finite logical coordinate.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LogicalPointError;

impl fmt::Display for LogicalPointError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("logical point coordinates must be finite")
    }
}

impl Error for LogicalPointError {}

/// Logical position in UI coordinate space.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LogicalPoint {
    x: f32,
    y: f32,
}

impl LogicalPoint {
    /// Creates a logical point.
    ///
    /// # Errors
    ///
    /// Returns [`LogicalPointError`] if either coordinate is non-finite.
    pub const fn new(x: f32, y: f32) -> Result<Self, LogicalPointError> {
        if x.is_nan()
            || y.is_nan()
            || x == f32::INFINITY
            || x == f32::NEG_INFINITY
            || y == f32::INFINITY
            || y == f32::NEG_INFINITY
        {
            Err(LogicalPointError)
        } else {
            Ok(Self { x, y })
        }
    }

    pub(crate) const fn from_finite(x: f32, y: f32) -> Self {
        Self { x, y }
    }

    /// Returns the horizontal coordinate.
    #[must_use]
    pub const fn x(&self) -> f32 {
        self.x
    }

    /// Returns the vertical coordinate.
    #[must_use]
    pub const fn y(&self) -> f32 {
        self.y
    }
}

/// Keyboard modifier state carried by host input.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
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

    /// Creates a modifier state from raw modifier bits.
    #[must_use]
    pub const fn from_bits(bits: u8) -> Self {
        Self {
            bits: bits & Self::ALL_BITS,
        }
    }

    /// Returns the raw modifier bits.
    #[must_use]
    pub const fn bits(&self) -> u8 {
        self.bits
    }

    /// Returns a modifier state with Shift added.
    #[must_use]
    pub const fn with_shift(self) -> Self {
        Self::from_bits(self.bits | Self::SHIFT_BIT)
    }

    /// Returns a modifier state with Control added.
    #[must_use]
    pub const fn with_control(self) -> Self {
        Self::from_bits(self.bits | Self::CONTROL_BIT)
    }

    /// Returns a modifier state with Alt added.
    #[must_use]
    pub const fn with_alt(self) -> Self {
        Self::from_bits(self.bits | Self::ALT_BIT)
    }

    /// Returns a modifier state with Meta, Command, or Windows added.
    #[must_use]
    pub const fn with_meta(self) -> Self {
        Self::from_bits(self.bits | Self::META_BIT)
    }

    /// Returns whether Shift is active.
    #[must_use]
    pub const fn shift(&self) -> bool {
        self.bits & Self::SHIFT_BIT != 0
    }

    /// Returns whether Control is active.
    #[must_use]
    pub const fn control(&self) -> bool {
        self.bits & Self::CONTROL_BIT != 0
    }

    /// Returns whether Alt is active.
    #[must_use]
    pub const fn alt(&self) -> bool {
        self.bits & Self::ALT_BIT != 0
    }

    /// Returns whether Meta, Command, or Windows is active.
    #[must_use]
    pub const fn meta(&self) -> bool {
        self.bits & Self::META_BIT != 0
    }
}

/// Pointer button reported by a host.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PointerButton {
    /// Primary activation button, usually left mouse or primary touch.
    Primary,
    /// Secondary button, usually right mouse.
    Secondary,
    /// Middle mouse button.
    Middle,
    /// Host-specific pointer button.
    Other(u16),
}

/// Pointer input phase.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PointerPhase {
    /// Pointer moved without a button state transition.
    Moved,
    /// Pointer button was pressed.
    Pressed,
    /// Pointer button was released.
    Released,
    /// Pointer stream was cancelled by the host.
    Cancelled,
}

/// Pointer input after optional host hit-test resolution.
#[derive(Clone, Debug, PartialEq)]
pub struct PointerEvent {
    phase: PointerPhase,
    position: LogicalPoint,
    button: Option<PointerButton>,
    modifiers: KeyModifiers,
    target: Option<MountedNodeId>,
}

impl PointerEvent {
    /// Creates a pointer event.
    #[must_use]
    pub const fn new(
        phase: PointerPhase,
        position: LogicalPoint,
        button: Option<PointerButton>,
        modifiers: KeyModifiers,
        target: Option<MountedNodeId>,
    ) -> Self {
        Self {
            phase,
            position,
            button,
            modifiers,
            target,
        }
    }

    /// Returns this pointer event with a replaced runtime target.
    #[must_use]
    pub fn with_target(mut self, target: Option<MountedNodeId>) -> Self {
        self.target = target;
        self
    }

    /// Returns the pointer phase.
    #[must_use]
    pub const fn phase(&self) -> PointerPhase {
        self.phase
    }

    /// Returns the logical pointer position.
    #[must_use]
    pub const fn position(&self) -> LogicalPoint {
        self.position
    }

    /// Returns the pointer button for button transitions.
    #[must_use]
    pub const fn button(&self) -> Option<PointerButton> {
        self.button
    }

    /// Returns keyboard modifiers active during the pointer event.
    #[must_use]
    pub const fn modifiers(&self) -> KeyModifiers {
        self.modifiers
    }

    /// Returns the resolved runtime target, if the host already hit-tested it.
    #[must_use]
    pub const fn target(&self) -> Option<&MountedNodeId> {
        self.target.as_ref()
    }
}

/// Keyboard key identity reported by a host.
#[non_exhaustive]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Key {
    /// Enter or Return.
    Enter,
    /// Space.
    Space,
    /// Tab.
    Tab,
    /// Escape.
    Escape,
    /// Text-producing character.
    Character(char),
    /// Host-specific named key.
    Named(String),
}

/// Keyboard input phase.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum KeyPhase {
    /// Key was pressed.
    Pressed,
    /// Key was released.
    Released,
}

/// Keyboard input after optional focus target resolution.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KeyboardEvent {
    phase: KeyPhase,
    key: Key,
    modifiers: KeyModifiers,
    target: Option<MountedNodeId>,
}

impl KeyboardEvent {
    /// Creates a keyboard event.
    #[must_use]
    pub const fn new(
        phase: KeyPhase,
        key: Key,
        modifiers: KeyModifiers,
        target: Option<MountedNodeId>,
    ) -> Self {
        Self {
            phase,
            key,
            modifiers,
            target,
        }
    }

    /// Returns the keyboard phase.
    #[must_use]
    pub const fn phase(&self) -> KeyPhase {
        self.phase
    }

    /// Returns the key identity.
    #[must_use]
    pub const fn key(&self) -> &Key {
        &self.key
    }

    /// Returns keyboard modifiers active during the key event.
    #[must_use]
    pub const fn modifiers(&self) -> KeyModifiers {
        self.modifiers
    }

    /// Returns the resolved runtime target, if the host already assigned one.
    #[must_use]
    pub const fn target(&self) -> Option<&MountedNodeId> {
        self.target.as_ref()
    }
}

/// Raw host input event accepted by the runtime boundary.
#[non_exhaustive]
#[derive(Clone, Debug, PartialEq)]
pub enum InputEvent {
    /// Pointer input.
    Pointer(PointerEvent),
    /// Keyboard input.
    Keyboard(KeyboardEvent),
}

/// Runtime-level intent resolved from input.
#[non_exhaustive]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum InputIntent {
    /// Activate an element by generated runtime node identity.
    ActivateNode(MountedNodeId),
}

impl InputIntent {
    /// Creates an activation intent for a runtime node.
    #[must_use]
    pub const fn activate_node(id: MountedNodeId) -> Self {
        Self::ActivateNode(id)
    }
}

/// Returns a pointer event targeted by hit testing its position against a surface frame.
#[must_use]
pub fn resolve_pointer_event_target(frame: &SurfaceFrame, event: PointerEvent) -> PointerEvent {
    let target = frame.hit_test_id(event.position());
    event.with_target(target)
}

/// Returns a pointer input event targeted by hit testing its position against a surface frame.
#[must_use]
pub fn resolve_pointer_input_event_target(frame: &SurfaceFrame, event: PointerEvent) -> InputEvent {
    InputEvent::Pointer(resolve_pointer_event_target(frame, event))
}
