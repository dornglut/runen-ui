//! File: domain/ui/ui_input/src/keyboard.rs
//! Purpose: Keyboard key and modifier contracts.

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub struct Modifiers {
    pub shift: bool,
    pub ctrl: bool,
    pub alt: bool,
    pub meta: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Key {
    Character(String),
    Enter,
    Escape,
    Backspace,
    Delete,
    Tab,
    Space,
    Left,
    Right,
    Up,
    Down,
    Home,
    End,
    PageUp,
    PageDown,
    Insert,
    F(u8),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyState {
    Pressed,
    Released,
    Repeated,
}
