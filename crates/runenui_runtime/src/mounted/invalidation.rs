use runenui_core::WidgetInvalidation;

use super::{CachedCapability, CachedSemanticContribution, node::MountedNode, tree::MountedTree};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct DirtyPhases(u16);

impl DirtyPhases {
    pub(crate) const TREE: Self = Self(1 << 0);
    pub(crate) const STYLE: Self = Self(1 << 1);
    pub(crate) const LAYOUT: Self = Self(1 << 2);
    pub(crate) const HIT_TEST: Self = Self(1 << 3);
    pub(crate) const PAINT: Self = Self(1 << 4);
    pub(crate) const SEMANTICS: Self = Self(1 << 5);
    pub(crate) const DIAGNOSTICS: Self = Self(1 << 6);
    pub(crate) const FOCUS_VALIDATION: Self = Self(1 << 7);
    pub(crate) const ALL: Self = Self((1 << 8) - 1);

    pub(crate) const fn insert(&mut self, phases: Self) {
        self.0 |= phases.0;
    }

    pub(crate) const fn contains(self, phases: Self) -> bool {
        self.0 & phases.0 == phases.0
    }

    pub(crate) const fn remove(&mut self, phases: Self) {
        self.0 &= !phases.0;
    }
}

impl core::ops::BitOrAssign for DirtyPhases {
    fn bitor_assign(&mut self, rhs: Self) {
        self.insert(rhs);
    }
}

pub(crate) const fn publication_is_dirty(invalidation: WidgetInvalidation) -> bool {
    let phases = publication_phases(invalidation);
    phases.0 != 0
}

pub(crate) fn invalidate_semantic_structure<Action>(node: &mut MountedNode<Action>) {
    node.caches.semantics = CachedSemanticContribution::Unresolved;
    node.dirty_phases.insert(DirtyPhases::SEMANTICS);
}

impl<Action> MountedTree<Action> {
    /// Marks the surface semantic product dirty after runtime-owned focus changes
    /// without invalidating any owner semantic contribution capability.
    pub(crate) fn mark_semantic_focus_product_dirty(&mut self) {
        let Some(root) = self.root.clone() else {
            return;
        };
        let Some(root) = self.node_mut(&root) else {
            return;
        };
        root.dirty_phases.insert(DirtyPhases::SEMANTICS);
    }
}

pub(crate) fn apply_invalidation<Action>(
    node: &mut MountedNode<Action>,
    invalidation: WidgetInvalidation,
) {
    if invalidation.contains(WidgetInvalidation::INTERACTION) {
        node.caches.activation = CachedCapability::Unresolved;
        node.caches.text_input = CachedCapability::Unresolved;
    }
    if invalidation.contains(WidgetInvalidation::LAYOUT) {
        node.caches.measurement = CachedCapability::Unresolved;
        node.caches.child_layout = CachedCapability::Unresolved;
        node.caches.paint = CachedCapability::Unresolved;
        node.caches.paint_context = None;
        node.caches.hit_test = CachedCapability::Unresolved;
        node.caches.hit_test_context = None;
    }
    if invalidation.contains(WidgetInvalidation::HIT_TEST) {
        node.caches.hit_test = CachedCapability::Unresolved;
        node.caches.hit_test_context = None;
    }
    if invalidation.contains(WidgetInvalidation::PAINT) {
        node.caches.paint = CachedCapability::Unresolved;
        node.caches.paint_context = None;
    }
    if invalidation.contains(WidgetInvalidation::SEMANTICS) {
        invalidate_semantic_structure(node);
    }
    if invalidation.contains(WidgetInvalidation::DIAGNOSTICS) {
        node.caches.diagnostics = CachedCapability::Unresolved;
    }
    node.dirty_phases.insert(publication_phases(invalidation));
}

const fn publication_phases(invalidation: WidgetInvalidation) -> DirtyPhases {
    let mut phases = DirtyPhases(0);
    if invalidation.contains(WidgetInvalidation::INTERACTION) {
        phases.insert(DirtyPhases::FOCUS_VALIDATION);
        phases.insert(DirtyPhases::PAINT);
        phases.insert(DirtyPhases::SEMANTICS);
    }
    if invalidation.contains(WidgetInvalidation::LAYOUT) {
        phases.insert(DirtyPhases::LAYOUT);
        phases.insert(DirtyPhases::HIT_TEST);
        phases.insert(DirtyPhases::PAINT);
    }
    if invalidation.contains(WidgetInvalidation::HIT_TEST) {
        phases.insert(DirtyPhases::HIT_TEST);
    }
    if invalidation.contains(WidgetInvalidation::PAINT) {
        phases.insert(DirtyPhases::PAINT);
    }
    if invalidation.contains(WidgetInvalidation::SEMANTICS) {
        phases.insert(DirtyPhases::SEMANTICS);
    }
    if invalidation.contains(WidgetInvalidation::DIAGNOSTICS) {
        phases.insert(DirtyPhases::DIAGNOSTICS);
    }
    phases
}
