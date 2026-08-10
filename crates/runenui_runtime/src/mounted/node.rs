#![allow(clippy::option_option, clippy::redundant_pub_crate)]

use core::fmt;

use runenui_core::{
    __runtime::{MountedWidget, MountedWidgetState},
    AuthoringDiagnostic, ElementId, ElementKey, FocusScope, Focusability, LayoutStyle, StyleIntent,
    WidgetActivation, WidgetStateTypeId, WidgetTypeId,
};

use super::{
    CapabilityCaches, DirtyPhases, IdentityDiagnostic, InteractionState, InteractionStateRef,
    MountedNodeId, semantic::SemanticBinding,
};

pub(crate) struct MountedNode<Action> {
    pub(crate) id: MountedNodeId,
    pub(super) semantic_bindings: Vec<SemanticBinding>,
    pub(crate) parent: Option<MountedNodeId>,
    pub(crate) children: Vec<MountedNodeId>,
    pub(crate) authored_id: Option<ElementId>,
    pub(crate) key: Option<ElementKey>,
    pub(crate) layout: LayoutStyle,
    pub(crate) style: StyleIntent,
    pub(crate) focusability: Focusability,
    pub(crate) focus_scope: Option<FocusScope>,
    pub(crate) authoring_diagnostics: Vec<AuthoringDiagnostic>,
    pub(crate) widget: MountedWidget<Action>,
    pub(crate) state: MountedWidgetState,
    #[cfg(any(test, feature = "internal-test-seams"))]
    pub(crate) state_corrupted: bool,
    pub(crate) interaction: InteractionState,
    pub(crate) integrity_failed: bool,
    pub(crate) caches: CapabilityCaches,
    pub(crate) dirty_phases: DirtyPhases,
}

#[cfg(any(test, feature = "internal-test-seams"))]
pub(crate) const fn state_is_corrupted<Action>(node: &MountedNode<Action>) -> bool {
    node.state_corrupted
}

#[cfg(not(any(test, feature = "internal-test-seams")))]
pub(crate) const fn state_is_corrupted<Action>(_: &MountedNode<Action>) -> bool {
    false
}

impl<Action> fmt::Debug for MountedNode<Action> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MountedNode")
            .field("id", &self.id)
            .field("semantic_binding_count", &self.semantic_bindings.len())
            .field("parent", &self.parent)
            .field("children", &self.children)
            .field("authored_id", &self.authored_id)
            .field("key", &self.key)
            .field("widget", &self.widget)
            .finish_non_exhaustive()
    }
}

/// Borrowed read-only mounted node inspection.
pub struct MountedNodeRef<'a, Action> {
    pub(crate) node: &'a MountedNode<Action>,
}

impl<Action> Clone for MountedNodeRef<'_, Action> {
    fn clone(&self) -> Self {
        *self
    }
}
impl<Action> Copy for MountedNodeRef<'_, Action> {}

impl<'a, Action> MountedNodeRef<'a, Action> {
    #[must_use]
    pub const fn id(&self) -> &'a MountedNodeId {
        &self.node.id
    }
    #[must_use]
    pub const fn parent(&self) -> Option<&'a MountedNodeId> {
        self.node.parent.as_ref()
    }
    #[must_use]
    pub fn children(&self) -> &'a [MountedNodeId] {
        &self.node.children
    }
    #[must_use]
    pub const fn authored_id(&self) -> Option<&'a ElementId> {
        self.node.authored_id.as_ref()
    }
    #[must_use]
    pub const fn element_key(&self) -> Option<&'a ElementKey> {
        self.node.key.as_ref()
    }
    #[must_use]
    pub fn widget_type_id(&self) -> WidgetTypeId {
        self.node.widget.widget_type_id()
    }
    #[must_use]
    pub fn widget_state_type_id(&self) -> WidgetStateTypeId {
        self.node.widget.state_type_id()
    }
    #[must_use]
    pub fn widget_type_name(&self) -> &'static str {
        self.node.widget.widget_type_name()
    }
    #[must_use]
    pub fn activation(&self) -> WidgetActivation {
        self.node.caches.activation.ready().unwrap_or_default()
    }
    #[must_use]
    pub fn is_focusable(&self) -> bool {
        let a = self.activation();
        a.enabled()
            && match self.node.focusability {
                Focusability::Automatic => a.is_actionable(),
                Focusability::Focusable => true,
                _ => false,
            }
    }
    #[must_use]
    pub const fn focusability(&self) -> Focusability {
        self.node.focusability
    }
    #[must_use]
    pub const fn focus_scope(&self) -> Option<FocusScope> {
        self.node.focus_scope
    }
    #[must_use]
    pub const fn interaction(&self) -> InteractionStateRef<'a> {
        InteractionStateRef(&self.node.interaction)
    }
    #[must_use]
    pub const fn layout(&self) -> &'a LayoutStyle {
        &self.node.layout
    }
    #[must_use]
    pub const fn style(&self) -> &'a StyleIntent {
        &self.node.style
    }
    #[must_use]
    pub const fn authoring_diagnostics(&self) -> &'a [AuthoringDiagnostic] {
        self.node.authoring_diagnostics.as_slice()
    }
}

/// Deterministic logical-preorder borrowed mounted-tree view.
pub struct MountedTreeIndex<'a, Action> {
    pub(crate) nodes: Vec<MountedNodeRef<'a, Action>>,
    pub(crate) diagnostics: Vec<IdentityDiagnostic>,
}

impl<'a, Action> MountedTreeIndex<'a, Action> {
    #[must_use]
    pub fn nodes(&self) -> &[MountedNodeRef<'a, Action>] {
        &self.nodes
    }
    #[must_use]
    pub fn diagnostics(&self) -> &[IdentityDiagnostic] {
        &self.diagnostics
    }
    pub fn focusable_nodes(&self) -> impl Iterator<Item = &MountedNodeRef<'a, Action>> {
        self.nodes.iter().filter(|n| n.is_focusable())
    }
    #[must_use]
    pub fn first_focusable_node(&self) -> Option<&MountedNodeRef<'a, Action>> {
        self.focusable_nodes().next()
    }
    #[must_use]
    pub fn last_focusable_node(&self) -> Option<&MountedNodeRef<'a, Action>> {
        self.focusable_nodes().last()
    }
    #[must_use]
    pub fn node(&self, id: &MountedNodeId) -> Option<&MountedNodeRef<'a, Action>> {
        self.nodes.iter().find(|n| n.id() == id)
    }
    #[must_use]
    pub fn next_focusable_after(&self, id: &MountedNodeId) -> Option<&MountedNodeRef<'a, Action>> {
        self.nodes
            .iter()
            .skip_while(|n| n.id() != id)
            .skip(1)
            .find(|n| n.is_focusable())
    }
    #[must_use]
    pub fn previous_focusable_before(
        &self,
        id: &MountedNodeId,
    ) -> Option<&MountedNodeRef<'a, Action>> {
        self.nodes
            .iter()
            .take_while(|n| n.id() != id)
            .filter(|n| n.is_focusable())
            .last()
    }
}
