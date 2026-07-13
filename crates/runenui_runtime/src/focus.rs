//! Mounted focus state.

use crate::MountedNodeId;

#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FocusTargetResult {
    Focused,
    NotFocusable,
    StaleTarget,
    ForeignRuntime,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct FocusState {
    focused_node_id: Option<MountedNodeId>,
}

impl FocusState {
    #[must_use]
    pub(crate) const fn new() -> Self {
        Self {
            focused_node_id: None,
        }
    }
    #[must_use]
    pub const fn focused_node(&self) -> Option<&MountedNodeId> {
        self.focused_node_id.as_ref()
    }
    #[must_use]
    pub fn is_focused(&self, id: &MountedNodeId) -> bool {
        self.focused_node_id.as_ref() == Some(id)
    }
    pub(crate) fn set(&mut self, id: MountedNodeId) {
        self.focused_node_id = Some(id);
    }
    pub(crate) fn clear(&mut self) {
        self.focused_node_id = None;
    }
}
