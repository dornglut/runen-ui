//! File: domain/ui/ui_input/src/shortcut.rs
//! Purpose: Shortcut/chord definitions.

use crate::{Key, Modifiers};

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Shortcut {
    pub key: Key,
    pub modifiers: Modifiers,
}

impl Shortcut {
    pub fn new(key: Key, modifiers: Modifiers) -> Self {
        Self { key, modifiers }
    }
}
