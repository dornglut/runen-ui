//! Runtime input policy result types.

use crate::{ActivationResult, RuntimeNodeId};

#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum KeyboardFocusResult {
    Moved(RuntimeNodeId),
    NoFocusableNode,
    Ignored,
}

#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum KeyboardActivationResult {
    Handled(ActivationResult),
    NoFocusedNode,
    Ignored,
}

#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PointerFocusResult {
    Moved(RuntimeNodeId),
    NoTarget,
    NotFound,
    NotFocusable,
    Ignored,
}

#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PointerActivationResult {
    Handled(ActivationResult),
    NoTarget,
    Ignored,
}

#[non_exhaustive]
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
