//! Transitional keyboard focus policy result.

use crate::MountedNodeId;

/// Transitional M4C5 keyboard-focus proof result.
#[non_exhaustive]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum KeyboardFocusResult {
    Moved(MountedNodeId),
    NoFocusableNode,
    Ignored,
}
