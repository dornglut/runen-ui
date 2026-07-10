//! Runtime node identity and indexing.

use runenui_core::{Element, ElementId, ElementKind};

use crate::TraceTarget;

/// Generated runtime identity for an element in one built tree.
///
/// Runtime node IDs are assigned by pre-order traversal. They are stable for a
/// specific built tree and are intended for runtime internals such as hit-test,
/// focus, tracing, and renderer feedback. They are not authored public handles.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct RuntimeNodeId(usize);

impl RuntimeNodeId {
    /// Root node ID for a built runtime tree.
    pub const ROOT: Self = Self(0);

    /// Creates a runtime node ID from a traversal index.
    #[must_use]
    pub const fn from_index(index: usize) -> Self {
        Self(index)
    }

    /// Returns the traversal index backing this node ID.
    #[must_use]
    pub const fn as_usize(self) -> usize {
        self.0
    }
}

/// Borrowed runtime node view into the current element tree.
pub struct RuntimeNodeRef<'a, Action> {
    id: RuntimeNodeId,
    parent: Option<RuntimeNodeId>,
    element: &'a Element<Action>,
}

impl<'a, Action> RuntimeNodeRef<'a, Action> {
    const fn new(
        id: RuntimeNodeId,
        parent: Option<RuntimeNodeId>,
        element: &'a Element<Action>,
    ) -> Self {
        Self {
            id,
            parent,
            element,
        }
    }

    /// Returns the generated runtime node ID.
    #[must_use]
    pub const fn id(&self) -> RuntimeNodeId {
        self.id
    }

    /// Returns the generated runtime parent ID, if this node is not the root.
    #[must_use]
    pub const fn parent(&self) -> Option<RuntimeNodeId> {
        self.parent
    }

    /// Returns the borrowed element for this runtime node.
    #[must_use]
    pub const fn element(&self) -> &'a Element<Action> {
        self.element
    }

    /// Returns the optional authored element ID.
    #[must_use]
    pub const fn authored_id(&self) -> Option<&'a ElementId> {
        self.element.element_id()
    }

    pub(crate) fn trace_target(&self) -> TraceTarget {
        TraceTarget::new(self.id, self.authored_id().cloned())
    }
}

/// Indexed borrowed view over one built runtime tree.
pub struct RuntimeTreeIndex<'a, Action> {
    nodes: Vec<RuntimeNodeRef<'a, Action>>,
}

impl<'a, Action> RuntimeTreeIndex<'a, Action> {
    /// Builds an index for the provided root element tree.
    #[must_use]
    pub fn new(root: &'a Element<Action>) -> Self {
        let mut index = Self { nodes: Vec::new() };
        index.push_node(None, root);
        index
    }

    fn push_node(
        &mut self,
        parent: Option<RuntimeNodeId>,
        element: &'a Element<Action>,
    ) -> RuntimeNodeId {
        let id = RuntimeNodeId::from_index(self.nodes.len());
        self.nodes.push(RuntimeNodeRef::new(id, parent, element));

        if let ElementKind::Container(container) = element.kind() {
            for child in container.children() {
                self.push_node(Some(id), child);
            }
        }

        id
    }

    /// Returns all indexed runtime nodes in pre-order traversal order.
    #[must_use]
    pub const fn nodes(&self) -> &[RuntimeNodeRef<'a, Action>] {
        self.nodes.as_slice()
    }

    /// Returns the node with the generated runtime node ID.
    #[must_use]
    pub fn node(&self, id: RuntimeNodeId) -> Option<&RuntimeNodeRef<'a, Action>> {
        self.nodes.get(id.as_usize())
    }

    /// Returns the first node with the matching authored element ID.
    #[must_use]
    pub fn node_by_authored_id(&self, id: impl AsRef<str>) -> Option<&RuntimeNodeRef<'a, Action>> {
        let id = id.as_ref();
        self.nodes.iter().find(
            |node| matches!(node.authored_id(), Some(element_id) if element_id.as_str() == id),
        )
    }
}
