//! Transitional keyboard proof vocabulary.
//!
//! M4C3 moves the complete public pointer protocol into `runenui_core` and
//! removes the unchecked pointer-target proof path. Keyboard routing remains an
//! explicitly deferred M4C5 concern.

use runenui_core::{KeyModifiers, MountedNodeId};

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

/// Transitional keyboard input after optional focus target resolution.
///
/// M4C5 replaces this proof surface with the complete core-owned keyboard,
/// committed-text, and IME protocol. It carries no pointer variant or pointer
/// authority.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct KeyboardEvent {
    phase: KeyPhase,
    key: Key,
    modifiers: KeyModifiers,
    target: Option<MountedNodeId>,
}

impl KeyboardEvent {
    /// Creates a transitional keyboard proof event.
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

    /// Returns the resolved runtime target, if the transitional proof assigned one.
    #[must_use]
    pub const fn target(&self) -> Option<&MountedNodeId> {
        self.target.as_ref()
    }
}
