#![allow(
    clippy::option_if_let_else,
    clippy::option_option,
    clippy::redundant_pub_crate,
    clippy::single_match_else
)]

use runenui_core::{
    __runtime::{RuntimeNamespace, WidgetBridgeError},
    Element, ElementId, SubscriptionSet,
};

use super::{
    DirtyPhases, MountedNodeId,
    arena::{MountedArena, MountedArenaCapacityError},
    node::MountedNode,
    reconcile::IncomingNode,
};
use crate::ReconciliationDiagnostic;

pub(crate) struct ReconcileStats<Action> {
    pub(crate) mounted: usize,
    pub(crate) updated: usize,
    pub(crate) unmounted: usize,
    pub(crate) moved: usize,
    pub(crate) diagnostics: Vec<ReconciliationDiagnostic>,
    pub(crate) mounted_owners: Vec<MountedNodeId>,
    pub(crate) unmounted_owners: Vec<MountedNodeId>,
    pub(crate) subscription_invalidated: Vec<MountedNodeId>,
    pub(crate) mounted_outputs: Vec<(
        MountedNodeId,
        Vec<runenui_core::__runtime::MountedEffect<Action>>,
    )>,
}

impl<Action> Default for ReconcileStats<Action> {
    fn default() -> Self {
        Self {
            mounted: 0,
            updated: 0,
            unmounted: 0,
            moved: 0,
            diagnostics: Vec::new(),
            mounted_owners: Vec::new(),
            unmounted_owners: Vec::new(),
            subscription_invalidated: Vec::new(),
            mounted_outputs: Vec::new(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct MountedIdentityExhausted;

impl From<MountedArenaCapacityError> for MountedIdentityExhausted {
    fn from(_: MountedArenaCapacityError) -> Self {
        Self
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum TargetStatus {
    Live,
    Stale,
    Missing,
    Foreign,
}

/// Redacted deterministic evidence for one authored-ID automation match.
///
/// The logical preorder is stable for one mounted tree and the mounted identity
/// is opaque and generation-scoped. Neither field exposes widget state or input
/// content.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AutomationMatchDiagnostic {
    logical_preorder: usize,
    mounted_node_id: MountedNodeId,
}

impl AutomationMatchDiagnostic {
    pub(crate) const fn new(logical_preorder: usize, mounted_node_id: MountedNodeId) -> Self {
        Self {
            logical_preorder,
            mounted_node_id,
        }
    }

    /// Returns this candidate's stable logical preorder position.
    #[must_use]
    pub const fn logical_preorder(&self) -> usize {
        self.logical_preorder
    }

    /// Returns the opaque exact mounted lifetime that matched.
    #[must_use]
    pub const fn mounted_node_id(&self) -> &MountedNodeId {
        &self.mounted_node_id
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum AutomationResolution {
    Unique(MountedNodeId),
    Missing,
    Ambiguous {
        candidates: Vec<AutomationMatchDiagnostic>,
    },
}

pub(crate) struct MountedTree<Action> {
    pub(super) runtime: RuntimeNamespace,
    pub(super) arena: MountedArena<MountedNode<Action>>,
    pub(super) root: Option<MountedNodeId>,
    pub(super) shutdown: bool,
}

impl<Action> MountedTree<Action> {
    pub(crate) fn empty() -> Self {
        Self {
            runtime: RuntimeNamespace::__runtime_new(),
            arena: MountedArena::new(),
            root: None,
            shutdown: false,
        }
    }

    #[cfg(test)]
    pub(crate) fn mount(root: Element<Action>) -> (Self, ReconcileStats<Action>) {
        Self::mount_with_public_slot_limit(root, u64::from(u32::MAX) + 1)
            .unwrap_or_else(|_| unreachable!("test tree remains within public identity capacity"))
    }

    pub(crate) fn mount_with_public_slot_limit(
        root: Element<Action>,
        public_slot_limit: u64,
    ) -> Result<(Self, ReconcileStats<Action>), MountedIdentityExhausted> {
        let root = IncomingNode::from_element(root);
        let mut tree = Self::empty();
        tree.arena
            .preflight_live_count(root.node_count()?, public_slot_limit)?;
        let mut stats = ReconcileStats::default();
        let root_id = tree.mount_incoming(None, root, &mut stats);
        tree.root = Some(root_id);
        Ok((tree, stats))
    }

    #[cfg(test)]
    pub(crate) fn reconcile(&mut self, root: Element<Action>) -> ReconcileStats<Action> {
        let mut before_unmount = |_: &MountedNodeId| {};
        self.reconcile_with_before_unmount(root, &mut before_unmount)
    }

    #[cfg(test)]
    pub(crate) fn reconcile_with_before_unmount(
        &mut self,
        root: Element<Action>,
        before_unmount: &mut dyn FnMut(&MountedNodeId),
    ) -> ReconcileStats<Action> {
        self.reconcile_with_before_unmount_and_public_slot_limit(
            root,
            u64::from(u32::MAX) + 1,
            before_unmount,
        )
        .unwrap_or_else(|_| unreachable!("test tree remains within public identity capacity"))
    }

    #[cfg(test)]
    pub(crate) fn reconcile_with_before_unmount_and_public_slot_limit(
        &mut self,
        root: Element<Action>,
        public_slot_limit: u64,
        before_unmount: &mut dyn FnMut(&MountedNodeId),
    ) -> Result<ReconcileStats<Action>, MountedIdentityExhausted> {
        let plan = self.plan_reconciliation(root, public_slot_limit)?;
        self.apply_reconciliation(plan, before_unmount)
            .map_err(|_| MountedIdentityExhausted)
    }

    pub(crate) fn target_status(&self, id: &MountedNodeId) -> TargetStatus {
        let Some((slot, generation)) = self.runtime.__runtime_mounted_parts(id) else {
            return TargetStatus::Foreign;
        };
        let slot = slot as usize;
        if self.arena.get(slot, generation).is_some() {
            TargetStatus::Live
        } else if self.arena.contains_slot(slot) {
            TargetStatus::Stale
        } else {
            TargetStatus::Missing
        }
    }

    pub(crate) fn composition_generation(&self, value: u64) -> runenui_core::CompositionGeneration {
        self.runtime.__runtime_composition_generation(value)
    }

    pub(crate) fn composition_generation_is_local(
        &self,
        generation: &runenui_core::CompositionGeneration,
    ) -> bool {
        self.runtime
            .__runtime_composition_generation_is_local(generation)
    }

    pub(crate) fn resolve_authored_id(&self, authored_id: &ElementId) -> AutomationResolution {
        let matches: Vec<_> = self
            .preorder_ids()
            .into_iter()
            .enumerate()
            .filter(|id| {
                self.node(&id.1).and_then(|node| node.authored_id.as_ref()) == Some(authored_id)
            })
            .map(|(logical_preorder, id)| AutomationMatchDiagnostic::new(logical_preorder, id))
            .collect();
        match matches.as_slice() {
            [] => AutomationResolution::Missing,
            [candidate] => AutomationResolution::Unique(candidate.mounted_node_id.clone()),
            _ => AutomationResolution::Ambiguous {
                candidates: matches,
            },
        }
    }

    pub(crate) fn node(&self, id: &MountedNodeId) -> Option<&MountedNode<Action>> {
        let (slot, generation) = self.runtime.__runtime_mounted_parts(id)?;
        self.arena.get(slot as usize, generation)
    }

    pub(crate) fn trace_target(&self, id: &MountedNodeId) -> crate::TraceTarget {
        crate::TraceTarget::new(
            id.clone(),
            self.node(id).and_then(|node| node.authored_id.clone()),
        )
    }
    pub(crate) fn declare_subscriptions(
        &self,
        id: &MountedNodeId,
        subscriptions: &mut SubscriptionSet<Action>,
    ) -> Result<(), WidgetBridgeError> {
        let node = self
            .node(id)
            .ok_or(WidgetBridgeError::StatePayloadMismatch)?;
        node.widget.subscriptions(&node.state, subscriptions)
    }
    pub(crate) fn node_mut(&mut self, id: &MountedNodeId) -> Option<&mut MountedNode<Action>> {
        let (slot, generation) = self.runtime.__runtime_mounted_parts(id)?;
        self.arena.get_mut(slot as usize, generation)
    }
    pub(crate) const fn live_count(&self) -> usize {
        self.arena.live_count()
    }

    #[cfg(test)]
    pub(crate) fn set_interaction_for_test(
        &mut self,
        id: &MountedNodeId,
        hovered: bool,
        pressed: bool,
        capture_placeholder: bool,
        scroll_offset: (f32, f32),
    ) {
        let node = self
            .node_mut(id)
            .unwrap_or_else(|| unreachable!("test target remains live"));
        node.interaction = super::InteractionState {
            hovered,
            pressed,
            capture_placeholder,
            scroll_offset,
        };
    }

    #[cfg(any(test, feature = "internal-test-seams"))]
    pub(crate) fn corrupt_state_for_test(&mut self, id: &MountedNodeId) {
        let node = self
            .node_mut(id)
            .unwrap_or_else(|| unreachable!("test target remains live"));
        node.state_corrupted = true;
        node.caches = super::CapabilityCaches::default();
        node.dirty_phases = DirtyPhases::ALL;
    }

    #[cfg(feature = "internal-test-seams")]
    pub(crate) fn missing_target_for_test(&self) -> MountedNodeId {
        let slot = u32::try_from(self.arena.slot_count())
            .unwrap_or_else(|_| unreachable!("mounted arena slots fit public identity"));
        self.runtime.__runtime_mounted_id(slot, 1)
    }

    #[cfg(feature = "internal-test-seams")]
    pub(crate) fn stale_target_for_test(&self, live: &MountedNodeId) -> MountedNodeId {
        let (slot, generation) = self
            .runtime
            .__runtime_mounted_parts(live)
            .unwrap_or_else(|| unreachable!("test target belongs to this runtime"));
        self.runtime
            .__runtime_mounted_id(slot, generation.saturating_add(1))
    }

    #[cfg(feature = "internal-test-seams")]
    pub(crate) fn break_parent_link_for_test(&mut self, id: &MountedNodeId) {
        let missing = self.missing_target_for_test();
        let node = self
            .node_mut(id)
            .unwrap_or_else(|| unreachable!("test target remains live"));
        node.parent = Some(missing);
    }

    pub(crate) fn pending_phases(&self) -> DirtyPhases {
        let mut pending = DirtyPhases::default();
        for id in self.preorder_ids() {
            if let Some(node) = self.node(&id) {
                pending |= node.dirty_phases;
            }
        }
        pending
    }

    pub(crate) fn finish_publication(&mut self, completed: DirtyPhases) {
        for id in self.preorder_ids() {
            if let Some(node) = self.node_mut(&id) {
                node.dirty_phases.remove(completed);
            }
        }
    }

    pub(crate) fn finish_focus_validation(&mut self) {
        for id in self.preorder_ids() {
            if let Some(node) = self.node_mut(&id) {
                node.dirty_phases.remove(DirtyPhases::FOCUS_VALIDATION);
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct PublicSlotOverflow;

pub(super) fn checked_public_slot(slot: usize) -> Result<u32, PublicSlotOverflow> {
    u32::try_from(slot).map_err(|_| PublicSlotOverflow)
}

#[cfg(test)]
mod tests {
    use std::{cell::Cell, rc::Rc};

    use runenui_core::{
        Element, ElementId, View, Widget, WidgetUnmountContext, children, column, text,
    };

    use super::{
        AutomationResolution, MountedNodeId, MountedTree, PublicSlotOverflow, ReconcileStats,
        TargetStatus, checked_public_slot,
    };
    use crate::ReconciliationDiagnostic;

    #[test]
    fn public_slot_conversion_rejects_overflow_without_truncation() {
        assert_eq!(checked_public_slot(u32::MAX as usize), Ok(u32::MAX));
        if usize::BITS > u32::BITS {
            assert_eq!(
                checked_public_slot((u32::MAX as usize) + 1),
                Err(PublicSlotOverflow)
            );
        }
    }

    fn mount_tree(root: Element<()>) -> (MountedTree<()>, ReconcileStats<()>) {
        MountedTree::mount(root)
    }

    fn tree(order: [&str; 2], a_key: &str, a_id: &str) -> Element<()> {
        column(
            order
                .into_iter()
                .map(|name| {
                    text(name)
                        .id(if name == "a" { a_id } else { "b" })
                        .key(if name == "a" { a_key } else { "b" })
                        .into_element()
                })
                .collect::<Vec<_>>(),
        )
        .key("root")
        .into_element()
    }

    fn authored_id(tree: &MountedTree<()>, authored: &str) -> MountedNodeId {
        let authored = ElementId::new(authored).unwrap_or_else(|_| unreachable!());
        match tree.resolve_authored_id(&authored) {
            AutomationResolution::Unique(id) => id,
            AutomationResolution::Missing | AutomationResolution::Ambiguous { .. } => {
                unreachable!()
            }
        }
    }

    #[derive(Debug)]
    struct UnmountOrderingProbe(Rc<Cell<bool>>);

    impl Widget<()> for UnmountOrderingProbe {
        type State = ();

        fn create_state(&self) -> Self::State {}

        fn unmount(&self, (): &mut Self::State, _: &mut WidgetUnmountContext) {
            assert!(
                self.0.get(),
                "owner invalidation must precede the unmount callback"
            );
        }
    }

    #[test]
    fn owner_invalidation_callback_precedes_unmount_hook() {
        let invalidated = Rc::new(Cell::new(false));
        let (mut mounted, _) =
            mount_tree(Element::new(UnmountOrderingProbe(Rc::clone(&invalidated))).key("probe"));

        mounted.reconcile_with_before_unmount(text("replacement").into_element(), &mut |_| {
            invalidated.set(true);
        });
        assert!(invalidated.get());
    }

    #[test]
    fn every_interaction_slot_is_retained_and_replacement_starts_fresh() {
        let (mut mounted, _) = mount_tree(tree(["a", "b"], "a", "a"));
        let a = authored_id(&mounted, "a");
        mounted.set_interaction_for_test(&a, true, true, true, (13.0, 21.0));

        mounted.reconcile(tree(["b", "a"], "a", "renamed-a"));
        let retained = authored_id(&mounted, "renamed-a");
        assert_eq!(retained, a);
        let index = mounted.index();
        let interaction = index
            .node(&retained)
            .unwrap_or_else(|| unreachable!())
            .interaction();
        assert!(interaction.hovered());
        assert!(interaction.pressed());
        assert!(interaction.capture_placeholder());
        assert_eq!(interaction.scroll_offset(), (13.0, 21.0));
        drop(index);

        mounted.reconcile(tree(["b", "a"], "replacement", "renamed-a"));
        let replacement = authored_id(&mounted, "renamed-a");
        assert_ne!(replacement, retained);
        let index = mounted.index();
        let interaction = index
            .node(&replacement)
            .unwrap_or_else(|| unreachable!())
            .interaction();
        assert!(!interaction.hovered());
        assert!(!interaction.pressed());
        assert!(!interaction.capture_placeholder());
        assert_eq!(interaction.scroll_offset(), (0.0, 0.0));
        drop(index);
        assert_eq!(mounted.target_status(&retained), TargetStatus::Stale);

        let old_root = mounted.publication_preorder_ids()[0].clone();
        mounted.set_interaction_for_test(&old_root, true, true, true, (13.0, 21.0));
        mounted.reconcile(
            column(vec![
                text("b").id("b").key("b").into_element(),
                text("a").id("renamed-a").key("replacement").into_element(),
            ])
            .key("replacement-root")
            .into_element(),
        );
        let new_root = mounted.publication_preorder_ids()[0].clone();
        assert_ne!(new_root, old_root);
        assert_eq!(mounted.target_status(&old_root), TargetStatus::Stale);
        let index = mounted.index();
        let interaction = index.nodes()[0].interaction();
        assert!(!interaction.hovered());
        assert!(!interaction.pressed());
        assert!(!interaction.capture_placeholder());
        assert_eq!(interaction.scroll_offset(), (0.0, 0.0));
    }

    #[test]
    fn removed_interaction_slots_are_cleared_before_generational_arena_reuse() {
        let (mut mounted, _) = mount_tree(tree(["a", "b"], "a", "a"));
        let removed = authored_id(&mounted, "a");
        mounted.set_interaction_for_test(&removed, true, true, true, (31.0, 47.0));

        mounted.reconcile(
            column(vec![text("b").id("b").key("b").into_element()])
                .key("root")
                .into_element(),
        );
        assert_eq!(mounted.target_status(&removed), TargetStatus::Stale);
        assert!(mounted.node(&removed).is_none());

        mounted.reconcile(
            column(vec![
                text("b").id("b").key("b").into_element(),
                text("c").id("c").key("c").into_element(),
            ])
            .key("root")
            .into_element(),
        );
        let replacement = authored_id(&mounted, "c");
        let replacement_parts = mounted
            .runtime
            .__runtime_mounted_parts(&replacement)
            .unwrap_or_else(|| unreachable!());
        let removed_parts = mounted
            .runtime
            .__runtime_mounted_parts(&removed)
            .unwrap_or_else(|| unreachable!());
        assert_eq!(replacement_parts.0, removed_parts.0);
        assert!(replacement_parts.1 > removed_parts.1);
        let index = mounted.index();
        let interaction = index
            .node(&replacement)
            .unwrap_or_else(|| unreachable!())
            .interaction();
        assert!(!interaction.hovered());
        assert!(!interaction.pressed());
        assert!(!interaction.capture_placeholder());
        assert_eq!(interaction.scroll_offset(), (0.0, 0.0));
        assert_eq!(mounted.target_status(&removed), TargetStatus::Stale);
        assert!(mounted.node(&removed).is_none());
    }

    fn parented(child_on_left: bool) -> Element<()> {
        let child = || text("a").id("a").key("a").into_element();
        column(vec![
            column(if child_on_left {
                vec![child()]
            } else {
                Vec::new()
            })
            .id("left")
            .key("left")
            .into_element(),
            column(if child_on_left {
                Vec::new()
            } else {
                vec![child()]
            })
            .id("right")
            .key("right")
            .into_element(),
        ])
        .key("root")
        .into_element()
    }

    #[test]
    fn cross_parent_remount_resets_every_interaction_slot() {
        let (mut mounted, _) = mount_tree(parented(true));
        let old = authored_id(&mounted, "a");
        mounted.set_interaction_for_test(&old, true, true, true, (9.0, 12.0));
        mounted.reconcile(parented(false));
        let remounted = authored_id(&mounted, "a");
        assert_ne!(remounted, old);
        assert_eq!(mounted.target_status(&old), TargetStatus::Stale);
        let index = mounted.index();
        let interaction = index
            .node(&remounted)
            .unwrap_or_else(|| unreachable!())
            .interaction();
        assert!(!interaction.hovered());
        assert!(!interaction.pressed());
        assert!(!interaction.capture_placeholder());
        assert_eq!(interaction.scroll_offset(), (0.0, 0.0));
    }

    #[test]
    fn shutdown_clears_all_lifetimes_and_is_idempotent() {
        let (mut mounted, _) = mount_tree(tree(["a", "b"], "a", "a"));
        let ids: Vec<_> = mounted.publication_preorder_ids().into_iter().collect();
        for id in &ids {
            mounted.set_interaction_for_test(id, true, true, true, (5.0, 8.0));
        }
        let first = mounted.shutdown();
        assert_eq!(first.unmounted, ids.len());
        assert_eq!(mounted.live_count(), 0);
        assert!(mounted.publication_preorder_ids().is_empty());
        for id in &ids {
            assert_eq!(mounted.target_status(id), TargetStatus::Stale);
            assert!(mounted.node(id).is_none());
        }
        let second = mounted.shutdown();
        assert_eq!(second.unmounted, 0);
        assert_eq!(mounted.live_count(), 0);
    }

    #[test]
    fn compatible_update_payload_mismatch_replaces_in_the_same_generation() {
        let (mut mounted, _) = mount_tree(tree(["a", "b"], "a", "a"));
        let root = mounted.index().nodes()[0].id().clone();
        let old_child = authored_id(&mounted, "a");
        mounted.corrupt_state_for_test(&root);

        let stats = mounted.reconcile(tree(["a", "b"], "a", "a"));
        let new_root = mounted.index().nodes()[0].id().clone();
        let new_child = authored_id(&mounted, "a");
        assert_ne!(new_root, root);
        assert_ne!(new_child, old_child);
        assert_eq!(stats.mounted, 3);
        assert_eq!(stats.updated, 0);
        assert_eq!(stats.unmounted, 3);
        assert_eq!(
            stats.diagnostics,
            vec![
                ReconciliationDiagnostic::StatePayloadMismatch {
                    path: "root".to_owned(),
                },
                ReconciliationDiagnostic::StatePayloadMismatch {
                    path: "root".to_owned(),
                },
            ]
        );
        assert_eq!(mounted.target_status(&root), TargetStatus::Stale);
    }

    #[test]
    fn child_payload_mismatch_replaces_descendants_without_updating_them() {
        let authored = || {
            column(children![
                column(children![text("grandchild").id("grandchild")])
                    .id("corrupted-child")
                    .key("corrupted-child")
            ])
            .key("root")
            .into_element()
        };
        let (mut mounted, _) = mount_tree(authored());
        let child = authored_id(&mounted, "corrupted-child");
        let grandchild = authored_id(&mounted, "grandchild");
        mounted.corrupt_state_for_test(&child);

        let stats = mounted.reconcile(authored());
        let replacement_child = authored_id(&mounted, "corrupted-child");
        let replacement_grandchild = authored_id(&mounted, "grandchild");
        assert_ne!(replacement_child, child);
        assert_ne!(replacement_grandchild, grandchild);
        assert_eq!(stats.mounted, 2);
        assert_eq!(stats.updated, 1);
        assert_eq!(stats.unmounted, 2);
        assert_eq!(mounted.target_status(&child), TargetStatus::Stale);
        assert_eq!(mounted.target_status(&grandchild), TargetStatus::Stale);
        assert_eq!(
            stats.diagnostics,
            vec![
                ReconciliationDiagnostic::StatePayloadMismatch {
                    path: "root/0".to_owned(),
                },
                ReconciliationDiagnostic::StatePayloadMismatch {
                    path: "root/0".to_owned(),
                },
            ]
        );
    }
}
