//! Host-neutral keyboard shortcut vocabulary.
//!
//! Shortcuts are immutable authored declarations. They match raw keyboard
//! identity only; committed text, native virtual-key values, localized display
//! strings, and platform accelerator tables are deliberately outside this
//! contract.

use crate::{ApplicationCommand, KeyModifiers, KeyboardEvent, LogicalKey, PhysicalKey};

/// Explicit keyboard identity used by one shortcut chord.
#[non_exhaustive]
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub enum ShortcutKey {
    /// Interpreted host-neutral logical key identity.
    Logical(LogicalKey),
    /// Layout-independent host-neutral physical key identity.
    Physical(PhysicalKey),
}

impl ShortcutKey {
    /// Returns whether this identity matches the corresponding identity carried
    /// by `event`. Logical and physical identities never cross-match.
    #[must_use]
    pub fn matches(&self, event: &KeyboardEvent) -> bool {
        match self {
            Self::Logical(key) => key == event.logical_key(),
            Self::Physical(key) => key == event.physical_key(),
        }
    }
}

/// One exact host-neutral shortcut chord.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct ShortcutChord {
    key: ShortcutKey,
    modifiers: KeyModifiers,
}

impl ShortcutChord {
    #[must_use]
    pub const fn new(key: ShortcutKey, modifiers: KeyModifiers) -> Self {
        Self { key, modifiers }
    }

    #[must_use]
    pub fn logical(key: LogicalKey, modifiers: KeyModifiers) -> Self {
        Self::new(ShortcutKey::Logical(key), modifiers)
    }

    #[must_use]
    pub fn physical(key: PhysicalKey, modifiers: KeyModifiers) -> Self {
        Self::new(ShortcutKey::Physical(key), modifiers)
    }

    #[must_use]
    pub const fn key(&self) -> &ShortcutKey {
        &self.key
    }

    #[must_use]
    pub const fn modifiers(&self) -> KeyModifiers {
        self.modifiers
    }

    /// Matches only exact key identity and exact modifier state.
    ///
    /// Phase, repeat policy, composition state, default prevention, and editor
    /// ownership are evaluated by the runtime's keyboard-default authority.
    #[must_use]
    pub fn matches(&self, event: &KeyboardEvent) -> bool {
        self.modifiers == event.modifiers() && self.key.matches(event)
    }
}

/// Whether repeated key-down events may invoke one shortcut declaration.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum ShortcutRepeatPolicy {
    IgnoreRepeat,
    AllowRepeat,
}

impl ShortcutRepeatPolicy {
    #[must_use]
    pub const fn allows(self, repeated: bool) -> bool {
        !repeated || matches!(self, Self::AllowRepeat)
    }
}

/// One immutable scoped shortcut declaration.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct ShortcutBinding {
    chord: ShortcutChord,
    repeat_policy: ShortcutRepeatPolicy,
    command: ApplicationCommand,
}

impl ShortcutBinding {
    #[must_use]
    pub const fn new(
        chord: ShortcutChord,
        repeat_policy: ShortcutRepeatPolicy,
        command: ApplicationCommand,
    ) -> Self {
        Self {
            chord,
            repeat_policy,
            command,
        }
    }

    #[must_use]
    pub const fn chord(&self) -> &ShortcutChord {
        &self.chord
    }

    #[must_use]
    pub const fn repeat_policy(&self) -> ShortcutRepeatPolicy {
        self.repeat_policy
    }

    #[must_use]
    pub const fn command(&self) -> &ApplicationCommand {
        &self.command
    }
}

#[cfg(test)]
mod tests {
    use crate::{ApplicationCommandId, KeyLocation, KeyboardCompositionState, KeyboardPhase};

    use super::*;

    fn command(enabled: bool) -> ApplicationCommand {
        ApplicationCommand::new(
            ApplicationCommandId::from_static("document.save")
                .unwrap_or_else(|_| unreachable!("static command id is valid")),
            enabled,
        )
    }

    #[test]
    fn logical_and_physical_shortcut_identity_remain_distinct() {
        let event = KeyboardEvent::new(
            KeyboardPhase::Down,
            PhysicalKey::Code(String::from("KeyS")),
            LogicalKey::Character(String::from("s")),
            KeyModifiers::NONE.with_control(),
            false,
            KeyLocation::Standard,
            KeyboardCompositionState::Inactive,
            None,
        );
        assert!(
            ShortcutChord::logical(
                LogicalKey::Character(String::from("s")),
                KeyModifiers::NONE.with_control()
            )
            .matches(&event)
        );
        assert!(
            ShortcutChord::physical(
                PhysicalKey::Code(String::from("KeyS")),
                KeyModifiers::NONE.with_control()
            )
            .matches(&event)
        );
        assert!(
            !ShortcutChord::logical(
                LogicalKey::Character(String::from("S")),
                KeyModifiers::NONE.with_control()
            )
            .matches(&event)
        );
        assert!(
            !ShortcutChord::physical(
                PhysicalKey::Code(String::from("KeyZ")),
                KeyModifiers::NONE.with_control()
            )
            .matches(&event)
        );
    }

    #[test]
    fn shortcut_modifiers_and_repeat_policy_are_exact() {
        let event = KeyboardEvent::new(
            KeyboardPhase::Down,
            PhysicalKey::Code(String::from("KeyS")),
            LogicalKey::Character(String::from("s")),
            KeyModifiers::NONE.with_control().with_shift(),
            true,
            KeyLocation::Standard,
            KeyboardCompositionState::Inactive,
            None,
        );
        let chord = ShortcutChord::logical(
            LogicalKey::Character(String::from("s")),
            KeyModifiers::NONE.with_control(),
        );
        assert!(!chord.matches(&event));
        assert!(!ShortcutRepeatPolicy::IgnoreRepeat.allows(event.is_repeat()));
        assert!(ShortcutRepeatPolicy::AllowRepeat.allows(event.is_repeat()));

        let binding = ShortcutBinding::new(
            ShortcutChord::physical(PhysicalKey::Code(String::from("KeyS")), event.modifiers()),
            ShortcutRepeatPolicy::AllowRepeat,
            command(false),
        );
        assert!(!binding.command().enabled());
    }
}
