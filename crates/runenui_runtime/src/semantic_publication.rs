//! Renderer-independent semantic publication and incremental-update vocabulary.
//!
//! Widgets author [`runenui_core::SemanticNodeContribution`] values. These types
//! are runtime-issued products: live semantic identity, absolute bounds,
//! composed state/support, resolved relationships, tree shape, focus, and
//! revision authority are never widget-authored.

mod state;

use core::num::NonZeroU64;
use std::collections::HashMap;

use runenui_core::{
    LogicalRect, SemanticAction, SemanticRelationshipKind, SemanticRole, SemanticText,
    SemanticValue, SurfaceId,
};

use crate::SemanticNodeId;

pub use state::{
    SemanticPublicationPlan, SemanticPublicationPlanError, SemanticPublicationState,
};

/// Non-zero, non-wrapping revision of one exact surface semantic product.
///
/// Revision `1` is the first committed semantic snapshot. Revisions advance only
/// when the adapter-visible semantic product changes; diagnostics and unrelated
/// surface-input generations do not participate.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct SemanticRevision(NonZeroU64);

impl SemanticRevision {
    /// Revision assigned to the first committed semantic snapshot.
    pub const FIRST: Self = Self(NonZeroU64::MIN);

    /// Returns the numeric revision.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0.get()
    }
}

/// Runtime-composed semantic state visible in a published node.
///
/// Hidden nodes are absent from the published semantic tree, so hiddenness is
/// represented structurally rather than as a flag here. Disabled state already
/// includes owner-wide widget disablement; inertness remains authored semantic
/// state. Supported actions remain observable independently from both fields.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct SemanticNodeState {
    disabled: bool,
    inert: bool,
}

impl SemanticNodeState {
    /// Returns whether execution is disabled for this published node.
    #[must_use]
    pub const fn disabled(self) -> bool {
        self.disabled
    }

    /// Returns whether this published node is semantically inert.
    #[must_use]
    pub const fn inert(self) -> bool {
        self.inert
    }
}

/// One runtime-resolved semantic relationship.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SemanticRelationship {
    kind: SemanticRelationshipKind,
    target: SemanticNodeId,
}

impl SemanticRelationship {
    /// Returns the platform-neutral relationship category.
    #[must_use]
    pub const fn kind(&self) -> SemanticRelationshipKind {
        self.kind
    }

    /// Returns the exact current semantic target.
    #[must_use]
    pub const fn target(&self) -> &SemanticNodeId {
        &self.target
    }
}

/// One immutable node in a committed renderer-independent semantic snapshot.
///
/// Mounted ownership and owner-local [`runenui_core::SemanticKey`] bindings stay
/// runtime-private. Public consumers receive only semantic identity and semantic
/// tree/content facts.
#[derive(Clone, Debug, PartialEq)]
pub struct SemanticNode {
    id: SemanticNodeId,
    parent: Option<SemanticNodeId>,
    children: Vec<SemanticNodeId>,
    role: SemanticRole,
    name: Option<String>,
    description: Option<String>,
    value: Option<SemanticValue>,
    state: SemanticNodeState,
    supported_actions: Vec<SemanticAction>,
    relationships: Vec<SemanticRelationship>,
    bounds: LogicalRect,
    text: Option<SemanticText>,
}

impl SemanticNode {
    /// Returns the exact runtime-issued semantic identity.
    #[must_use]
    pub const fn id(&self) -> &SemanticNodeId {
        &self.id
    }

    /// Returns the semantic parent, or `None` for a semantic root.
    #[must_use]
    pub const fn parent(&self) -> Option<&SemanticNodeId> {
        self.parent.as_ref()
    }

    /// Returns semantic children in deterministic published order.
    #[must_use]
    pub const fn children(&self) -> &[SemanticNodeId] {
        self.children.as_slice()
    }

    /// Returns the platform-neutral semantic role.
    #[must_use]
    pub const fn role(&self) -> SemanticRole {
        self.role
    }

    /// Returns the semantic name when authored.
    #[must_use]
    pub fn name(&self) -> Option<&str> {
        self.name.as_deref()
    }

    /// Returns the semantic description when authored.
    #[must_use]
    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    /// Returns the semantic value when authored.
    #[must_use]
    pub const fn value(&self) -> Option<&SemanticValue> {
        self.value.as_ref()
    }

    /// Returns runtime-composed semantic state.
    #[must_use]
    pub const fn state(&self) -> SemanticNodeState {
        self.state
    }

    /// Returns supported semantic actions in deterministic order.
    ///
    /// Support is distinct from current execution availability. Disabled or
    /// inert nodes may therefore retain entries here.
    #[must_use]
    pub const fn supported_actions(&self) -> &[SemanticAction] {
        self.supported_actions.as_slice()
    }

    /// Returns exact runtime-resolved relationships in authored order.
    #[must_use]
    pub const fn relationships(&self) -> &[SemanticRelationship] {
        self.relationships.as_slice()
    }

    /// Returns absolute logical bounds in this surface's logical coordinate space.
    #[must_use]
    pub const fn bounds(&self) -> LogicalRect {
        self.bounds
    }

    /// Returns platform-neutral semantic text when authored.
    #[must_use]
    pub const fn text(&self) -> Option<&SemanticText> {
        self.text.as_ref()
    }
}

/// Immutable semantic snapshot for one exact logical surface.
///
/// [`Self::nodes`] is deterministic semantic preorder. [`Self::node`] performs
/// exact-ID lookup without exposing mutable runtime authority or mounted routing
/// identity.
#[derive(Clone, Debug)]
pub struct SemanticSnapshot {
    surface: SurfaceId,
    revision: SemanticRevision,
    roots: Vec<SemanticNodeId>,
    nodes: Vec<SemanticNode>,
    focused: Option<SemanticNodeId>,
    index: HashMap<SemanticNodeId, usize>,
}

impl PartialEq for SemanticSnapshot {
    fn eq(&self, other: &Self) -> bool {
        self.surface == other.surface
            && self.revision == other.revision
            && self.roots == other.roots
            && self.nodes == other.nodes
            && self.focused == other.focused
    }
}

impl SemanticSnapshot {
    /// Returns the exact logical surface owning this semantic product.
    #[must_use]
    pub const fn surface_id(&self) -> &SurfaceId {
        &self.surface
    }

    /// Returns the current semantic revision for this surface.
    #[must_use]
    pub const fn revision(&self) -> SemanticRevision {
        self.revision
    }

    /// Returns semantic roots in deterministic published order.
    #[must_use]
    pub const fn roots(&self) -> &[SemanticNodeId] {
        self.roots.as_slice()
    }

    /// Returns all published semantic nodes in deterministic preorder.
    #[must_use]
    pub const fn nodes(&self) -> &[SemanticNode] {
        self.nodes.as_slice()
    }

    /// Returns the exact current node when `id` belongs to this snapshot.
    #[must_use]
    pub fn node(&self, id: &SemanticNodeId) -> Option<&SemanticNode> {
        self.index.get(id).and_then(|index| self.nodes.get(*index))
    }

    /// Returns the semantic focus projection, if the focused mounted owner has a
    /// currently published visible PRIMARY semantic node.
    #[must_use]
    pub const fn focused(&self) -> Option<&SemanticNodeId> {
        self.focused.as_ref()
    }
}

/// Explicit semantic-focus replacement carried by an incremental update.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SemanticFocusChange {
    previous: Option<SemanticNodeId>,
    current: Option<SemanticNodeId>,
}

impl SemanticFocusChange {
    /// Returns the focus identity from the previous semantic revision.
    #[must_use]
    pub const fn previous(&self) -> Option<&SemanticNodeId> {
        self.previous.as_ref()
    }

    /// Returns the focus identity in the new semantic revision.
    #[must_use]
    pub const fn current(&self) -> Option<&SemanticNodeId> {
        self.current.as_ref()
    }
}

/// Delta between two consecutive committed semantic revisions of one surface.
///
/// Removals preserve previous semantic order. Additions and changed nodes
/// preserve new semantic order. `roots` and `focus` are present only when those
/// facts changed.
#[derive(Clone, Debug, PartialEq)]
pub struct SemanticUpdate {
    surface: SurfaceId,
    previous_revision: SemanticRevision,
    revision: SemanticRevision,
    removed: Vec<SemanticNodeId>,
    added: Vec<SemanticNode>,
    changed: Vec<SemanticNode>,
    roots: Option<Vec<SemanticNodeId>>,
    focus: Option<SemanticFocusChange>,
}

impl SemanticUpdate {
    /// Returns the exact surface whose semantic product changed.
    #[must_use]
    pub const fn surface_id(&self) -> &SurfaceId {
        &self.surface
    }

    /// Returns the semantic revision this delta requires as its base.
    #[must_use]
    pub const fn previous_revision(&self) -> SemanticRevision {
        self.previous_revision
    }

    /// Returns the resulting semantic revision.
    #[must_use]
    pub const fn revision(&self) -> SemanticRevision {
        self.revision
    }

    /// Returns removed identities in previous semantic order.
    #[must_use]
    pub const fn removed(&self) -> &[SemanticNodeId] {
        self.removed.as_slice()
    }

    /// Returns newly published nodes in new semantic order.
    #[must_use]
    pub const fn added(&self) -> &[SemanticNode] {
        self.added.as_slice()
    }

    /// Returns changed retained nodes in new semantic order.
    #[must_use]
    pub const fn changed(&self) -> &[SemanticNode] {
        self.changed.as_slice()
    }

    /// Returns replacement roots when semantic roots changed.
    #[must_use]
    pub const fn roots(&self) -> Option<&[SemanticNodeId]> {
        match &self.roots {
            Some(roots) => Some(roots.as_slice()),
            None => None,
        }
    }

    /// Returns the explicit focus transition when semantic focus changed.
    #[must_use]
    pub const fn focus(&self) -> Option<&SemanticFocusChange> {
        self.focus.as_ref()
    }
}

/// Read-only result of asking a committed semantic publication for changes from
/// one declared surface/revision base.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SemanticUpdateResult<'a> {
    /// The declared base already names the current semantic product.
    Unchanged,
    /// The publication contains the exact consecutive delta from the declared base.
    Delta(&'a SemanticUpdate),
    /// The declared surface or revision cannot consume the retained delta safely.
    FullResync(&'a SemanticSnapshot),
}

/// Independently typed semantic sibling published beside renderer/input products.
///
/// Only the immediately preceding committed delta is retained here. A consumer
/// that skips a revision receives [`SemanticUpdateResult::FullResync`] rather
/// than an ambiguous or reconstructed multi-hop delta.
#[derive(Clone, Debug, PartialEq)]
pub struct SemanticPublication {
    snapshot: SemanticSnapshot,
    update: Option<SemanticUpdate>,
}

impl SemanticPublication {
    /// Returns the complete current semantic snapshot.
    #[must_use]
    pub const fn snapshot(&self) -> &SemanticSnapshot {
        &self.snapshot
    }

    /// Returns the consecutive delta that produced this snapshot, if one exists.
    ///
    /// The first committed snapshot has no synthetic `0 -> 1` update, and an
    /// unchanged semantic product produces no new publication/update revision.
    #[must_use]
    pub const fn update(&self) -> Option<&SemanticUpdate> {
        self.update.as_ref()
    }

    /// Selects an exact consecutive delta or full resynchronization for a
    /// consumer-declared surface/revision base.
    #[must_use]
    pub fn update_from(
        &self,
        surface: &SurfaceId,
        revision: SemanticRevision,
    ) -> SemanticUpdateResult<'_> {
        if surface != self.snapshot.surface_id() {
            return SemanticUpdateResult::FullResync(&self.snapshot);
        }
        if revision == self.snapshot.revision() {
            return SemanticUpdateResult::Unchanged;
        }
        match &self.update {
            Some(update) if update.previous_revision() == revision => {
                SemanticUpdateResult::Delta(update)
            }
            _ => SemanticUpdateResult::FullResync(&self.snapshot),
        }
    }
}

#[cfg(test)]
mod tests {
    use core::num::NonZeroU64;
    use std::collections::HashMap;

    use runenui_core::{
        __runtime::RuntimeNamespace, LogicalPoint, LogicalRect, LogicalSize, SemanticRole,
    };

    use super::{
        SemanticNode, SemanticNodeState, SemanticPublication, SemanticRevision, SemanticSnapshot,
        SemanticUpdate, SemanticUpdateResult,
    };

    fn rect() -> LogicalRect {
        LogicalRect::new(
            LogicalPoint::new(0.0, 0.0).unwrap_or_else(|_| unreachable!("finite test point")),
            LogicalSize::try_new(10.0, 10.0).unwrap_or_else(|_| unreachable!("valid test size")),
        )
    }

    fn snapshot(namespace: &RuntimeNamespace, revision: SemanticRevision) -> SemanticSnapshot {
        let surface = namespace.__runtime_surface_id(0, 1);
        let id = namespace.__runtime_semantic_id(0, 1);
        let node = SemanticNode {
            id: id.clone(),
            parent: None,
            children: Vec::new(),
            role: SemanticRole::Button,
            name: Some("Save".to_owned()),
            description: None,
            value: None,
            state: SemanticNodeState::default(),
            supported_actions: Vec::new(),
            relationships: Vec::new(),
            bounds: rect(),
            text: None,
        };
        SemanticSnapshot {
            surface,
            revision,
            roots: vec![id.clone()],
            nodes: vec![node],
            focused: Some(id.clone()),
            index: HashMap::from([(id, 0)]),
        }
    }

    #[test]
    fn exact_id_lookup_is_independent_from_tree_position() {
        let namespace = RuntimeNamespace::__runtime_new();
        let snapshot = snapshot(&namespace, SemanticRevision::FIRST);
        let id = snapshot.roots()[0].clone();
        assert_eq!(snapshot.node(&id).map(SemanticNode::id), Some(&id));
        assert_eq!(snapshot.focused(), Some(&id));
    }

    #[test]
    fn update_selection_requires_exact_surface_and_previous_revision() {
        let namespace = RuntimeNamespace::__runtime_new();
        let current_revision = SemanticRevision(
            NonZeroU64::new(2).unwrap_or_else(|| unreachable!("test revision is non-zero")),
        );
        let snapshot = snapshot(&namespace, current_revision);
        let previous_revision = SemanticRevision::FIRST;
        let update = SemanticUpdate {
            surface: snapshot.surface_id().clone(),
            previous_revision,
            revision: current_revision,
            removed: Vec::new(),
            added: Vec::new(),
            changed: snapshot.nodes().to_vec(),
            roots: None,
            focus: None,
        };
        let publication = SemanticPublication {
            snapshot,
            update: Some(update),
        };
        assert!(matches!(
            publication.update_from(publication.snapshot().surface_id(), previous_revision),
            SemanticUpdateResult::Delta(_)
        ));
        assert_eq!(
            publication.update_from(publication.snapshot().surface_id(), current_revision),
            SemanticUpdateResult::Unchanged
        );
        let foreign_surface = RuntimeNamespace::__runtime_new().__runtime_surface_id(0, 1);
        assert!(matches!(
            publication.update_from(&foreign_surface, previous_revision),
            SemanticUpdateResult::FullResync(_)
        ));
    }
}
