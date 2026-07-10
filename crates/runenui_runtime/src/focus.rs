//! Runtime focus state.

use crate::RuntimeNodeId;

/// Runtime focus state for one built tree.
///
/// Focus stores generated runtime node identity. Runtime node IDs are tree-local,
/// so the runtime clears focus whenever a dispatch rebuilds the root tree.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct FocusState {
    focused_node_id: Option<RuntimeNodeId>,
}

impl FocusState {
    /// Creates an empty focus state.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            focused_node_id: None,
        }
    }

    /// Returns the currently focused runtime node ID, if any.
    #[must_use]
    pub const fn focused_node(&self) -> Option<RuntimeNodeId> {
        self.focused_node_id
    }

    /// Returns whether the provided runtime node ID is focused.
    #[must_use]
    pub const fn is_focused(&self, id: RuntimeNodeId) -> bool {
        match self.focused_node_id {
            Some(focused) => focused.as_usize() == id.as_usize(),
            None => false,
        }
    }

    /// Sets focus to the provided runtime node ID.
    pub const fn set(&mut self, id: RuntimeNodeId) {
        self.focused_node_id = Some(id);
    }

    /// Clears the focused runtime node ID.
    pub const fn clear(&mut self) {
        self.focused_node_id = None;
    }
}
