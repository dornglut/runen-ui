//! Runtime input policy result types.

use crate::{ActivationResult, RuntimeNodeId};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum KeyboardFocusResult {
    Moved(RuntimeNodeId),
    NoFocusableNode,
    Ignored,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum KeyboardActivationResult {
    Handled(ActivationResult),
    NoFocusedNode,
    Ignored,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PointerFocusResult {
    Moved(RuntimeNodeId),
    NoTarget,
    NotFound,
    NotFocusable,
    Ignored,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PointerActivationResult {
    Handled(ActivationResult),
    NoTarget,
    Ignored,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum InputEventResult {
    Pointer {
        focus: PointerFocusResult,
        activation: PointerActivationResult,
    },
    KeyboardFocus(KeyboardFocusResult),
    KeyboardActivation(KeyboardActivationResult),
    Ignored,
}
