use runenui_core::WidgetInvalidation;

use super::{CachedCapability, node::MountedNode};

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

pub(crate) fn apply_invalidation<Action>(
    node: &mut MountedNode<Action>,
    invalidation: WidgetInvalidation,
) {
    if invalidation.contains(WidgetInvalidation::INTERACTION) {
        node.caches.activation = CachedCapability::Unresolved;
        node.dirty_phases.insert(DirtyPhases::FOCUS_VALIDATION);
        node.dirty_phases.insert(DirtyPhases::PAINT);
        node.dirty_phases.insert(DirtyPhases::SEMANTICS);
    }
    if invalidation.contains(WidgetInvalidation::LAYOUT) {
        node.caches.measurement = CachedCapability::Unresolved;
        node.caches.child_layout = CachedCapability::Unresolved;
        node.dirty_phases.insert(DirtyPhases::LAYOUT);
        node.dirty_phases.insert(DirtyPhases::HIT_TEST);
    }
    if invalidation.contains(WidgetInvalidation::PAINT) {
        node.caches.paint = CachedCapability::Unresolved;
        node.dirty_phases.insert(DirtyPhases::PAINT);
    }
    if invalidation.contains(WidgetInvalidation::SEMANTICS) {
        node.caches.semantics = CachedCapability::Unresolved;
        node.dirty_phases.insert(DirtyPhases::SEMANTICS);
    }
    if invalidation.contains(WidgetInvalidation::DIAGNOSTICS) {
        node.caches.diagnostics = CachedCapability::Unresolved;
        node.dirty_phases.insert(DirtyPhases::DIAGNOSTICS);
    }
}
