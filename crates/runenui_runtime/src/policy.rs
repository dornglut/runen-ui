//! Runtime input policy result types.

use crate::{ActivationResult, MountedNodeId};

#[non_exhaustive]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum KeyboardFocusResult {
    Moved(MountedNodeId),
    NoFocusableNode,
    Ignored,
}

#[non_exhaustive]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum KeyboardActivationResult {
    Handled(ActivationResult),
    NoFocusedNode,
    Ignored,
}

#[non_exhaustive]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PointerFocusResult {
    Moved(MountedNodeId),
    NoTarget,
    NotFound,
    NotFocusable,
    Ignored,
}

#[non_exhaustive]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum PointerActivationResult {
    Handled(ActivationResult),
    NoTarget,
    Ignored,
}

#[non_exhaustive]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum InputEventResult {
    Pointer {
        focus: PointerFocusResult,
        activation: PointerActivationResult,
    },
    KeyboardFocus(KeyboardFocusResult),
    KeyboardActivation(KeyboardActivationResult),
    Ignored,
}
