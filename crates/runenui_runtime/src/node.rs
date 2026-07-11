//! Runtime node identity and indexing.

use core::fmt;
use std::collections::BTreeMap;

use runenui_core::{Element, ElementId, ElementKey, ElementKind};

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
    pub(crate) const ROOT: Self = Self(0);

    /// Creates a runtime node ID from a traversal index.
    #[must_use]
    pub(crate) const fn from_index(index: usize) -> Self {
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

    /// Returns the optional authored element key.
    #[must_use]
    pub const fn element_key(&self) -> Option<&'a ElementKey> {
        self.element.element_key()
    }

    /// Returns whether this node can receive focus in the current tree.
    #[must_use]
    pub const fn is_focusable(&self) -> bool {
        match self.element.kind() {
            ElementKind::Button(button) => button.enabled(),
            ElementKind::Text(_) | ElementKind::Container(_) => false,
        }
    }

    pub(crate) fn trace_target(&self) -> TraceTarget {
        TraceTarget::new(self.id, self.authored_id().cloned())
    }
}

/// Indexed borrowed view over one built runtime tree.
pub struct RuntimeTreeIndex<'a, Action> {
    nodes: Vec<RuntimeNodeRef<'a, Action>>,
    diagnostics: Vec<IdentityDiagnostic>,
}

/// Duplicate authored-identity category.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DuplicateIdentityKind {
    InvalidElementId,
    InvalidElementKey,
    ElementId,
    SiblingKey,
}

/// Deterministic duplicate-authored-identity diagnostic.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct IdentityDiagnostic {
    kind: DuplicateIdentityKind,
    value: String,
    first_path: String,
    duplicate_path: String,
}

impl IdentityDiagnostic {
    #[must_use]
    pub const fn kind(&self) -> DuplicateIdentityKind {
        self.kind
    }
    #[must_use]
    pub const fn value(&self) -> &str {
        self.value.as_str()
    }
    #[must_use]
    pub const fn first_path(&self) -> &str {
        self.first_path.as_str()
    }
    #[must_use]
    pub const fn duplicate_path(&self) -> &str {
        self.duplicate_path.as_str()
    }
}

impl fmt::Display for IdentityDiagnostic {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.kind {
            DuplicateIdentityKind::InvalidElementId | DuplicateIdentityKind::InvalidElementKey => {
                write!(
                    formatter,
                    "invalid {:?} {:?} at {}",
                    self.kind, self.value, self.duplicate_path
                )
            }
            DuplicateIdentityKind::ElementId | DuplicateIdentityKind::SiblingKey => {
                write!(
                    formatter,
                    "duplicate {:?} {:?}: first at {}, duplicate at {}",
                    self.kind, self.value, self.first_path, self.duplicate_path
                )
            }
        }
    }
}

impl<'a, Action> RuntimeTreeIndex<'a, Action> {
    /// Builds an index for the provided root element tree.
    #[must_use]
    pub(crate) fn new(root: &'a Element<Action>) -> Self {
        let mut index = Self {
            nodes: Vec::new(),
            diagnostics: Vec::new(),
        };
        let mut ids = BTreeMap::new();
        index.push_node(None, root, "root", &mut ids);
        index.diagnostics.sort_by(|left, right| {
            left.duplicate_path
                .cmp(&right.duplicate_path)
                .then_with(|| diagnostic_priority(left.kind).cmp(&diagnostic_priority(right.kind)))
                .then_with(|| left.value.cmp(&right.value))
        });
        index
    }

    fn push_node(
        &mut self,
        parent: Option<RuntimeNodeId>,
        element: &'a Element<Action>,
        path: &str,
        ids: &mut BTreeMap<ElementId, String>,
    ) -> RuntimeNodeId {
        let id = RuntimeNodeId::from_index(self.nodes.len());
        self.nodes.push(RuntimeNodeRef::new(id, parent, element));

        for authoring in element.authoring_diagnostics() {
            self.diagnostics.push(IdentityDiagnostic {
                kind: if authoring.field() == "id" {
                    DuplicateIdentityKind::InvalidElementId
                } else {
                    DuplicateIdentityKind::InvalidElementKey
                },
                value: authoring.value().to_owned(),
                first_path: path.to_owned(),
                duplicate_path: path.to_owned(),
            });
        }

        if let Some(authored_id) = element.element_id() {
            if let Some(first_path) = ids.get(authored_id) {
                self.diagnostics.push(IdentityDiagnostic {
                    kind: DuplicateIdentityKind::ElementId,
                    value: authored_id.as_str().to_owned(),
                    first_path: first_path.clone(),
                    duplicate_path: path.to_owned(),
                });
            } else {
                ids.insert(authored_id.clone(), path.to_owned());
            }
        }

        if let ElementKind::Container(container) = element.kind() {
            let mut sibling_keys: BTreeMap<ElementKey, String> = BTreeMap::new();
            for (child_index, child) in container.children().iter().enumerate() {
                let child_path = format!("{path}/{child_index}");
                if let Some(key) = child.element_key() {
                    if let Some(first_path) = sibling_keys.get(key) {
                        self.diagnostics.push(IdentityDiagnostic {
                            kind: DuplicateIdentityKind::SiblingKey,
                            value: key.as_str().to_owned(),
                            first_path: first_path.clone(),
                            duplicate_path: child_path.clone(),
                        });
                    } else {
                        sibling_keys.insert(key.clone(), child_path.clone());
                    }
                }
                self.push_node(Some(id), child, &child_path, ids);
            }
        }

        id
    }

    /// Returns all indexed runtime nodes in pre-order traversal order.
    #[must_use]
    pub const fn nodes(&self) -> &[RuntimeNodeRef<'a, Action>] {
        self.nodes.as_slice()
    }

    /// Returns duplicate diagnostics in stable preorder discovery order.
    #[must_use]
    pub const fn diagnostics(&self) -> &[IdentityDiagnostic] {
        self.diagnostics.as_slice()
    }

    /// Returns all focusable runtime nodes in traversal order.
    pub fn focusable_nodes(&self) -> impl Iterator<Item = &RuntimeNodeRef<'a, Action>> {
        self.nodes.iter().filter(|node| node.is_focusable())
    }

    /// Returns the first focusable node in traversal order.
    #[must_use]
    pub fn first_focusable_node(&self) -> Option<&RuntimeNodeRef<'a, Action>> {
        self.focusable_nodes().next()
    }

    /// Returns the last focusable node in traversal order.
    #[must_use]
    pub fn last_focusable_node(&self) -> Option<&RuntimeNodeRef<'a, Action>> {
        self.focusable_nodes().last()
    }

    /// Returns the next focusable node after the provided runtime node ID.
    #[must_use]
    pub fn next_focusable_after(&self, id: RuntimeNodeId) -> Option<&RuntimeNodeRef<'a, Action>> {
        self.focusable_nodes()
            .find(|node| node.id().as_usize() > id.as_usize())
    }

    /// Returns the previous focusable node before the provided runtime node ID.
    #[must_use]
    pub fn previous_focusable_before(
        &self,
        id: RuntimeNodeId,
    ) -> Option<&RuntimeNodeRef<'a, Action>> {
        self.focusable_nodes()
            .take_while(|node| node.id().as_usize() < id.as_usize())
            .last()
    }

    /// Returns the node with the generated runtime node ID.
    #[must_use]
    pub fn node(&self, id: RuntimeNodeId) -> Option<&RuntimeNodeRef<'a, Action>> {
        self.nodes.get(id.as_usize())
    }

    /// Returns the first node with the matching authored element ID.
    #[must_use]
    pub fn node_by_authored_id(&self, id: &ElementId) -> Option<&RuntimeNodeRef<'a, Action>> {
        self.nodes
            .iter()
            .find(|node| matches!(node.authored_id(), Some(element_id) if element_id == id))
    }
}

const fn diagnostic_priority(kind: DuplicateIdentityKind) -> u8 {
    match kind {
        DuplicateIdentityKind::InvalidElementId => 0,
        DuplicateIdentityKind::InvalidElementKey => 1,
        DuplicateIdentityKind::ElementId => 2,
        DuplicateIdentityKind::SiblingKey => 3,
    }
}
