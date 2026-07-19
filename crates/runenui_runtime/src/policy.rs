//! Runtime input policy result types.

use crate::MountedNodeId;

#[non_exhaustive]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum KeyboardFocusResult {
    Moved(MountedNodeId),
    NoFocusableNode,
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
