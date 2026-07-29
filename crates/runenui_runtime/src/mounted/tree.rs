#![allow(
    clippy::option_if_let_else,
    clippy::option_option,
    clippy::redundant_pub_crate,
    clippy::single_match_else
)]

use std::collections::HashSet;

use runenui_core::{
    __runtime::{ElementParts, RuntimeNamespace, WidgetBridgeError},
    Element, ElementId, SubscriptionSet, WidgetInvalidation, WidgetMountContext,
    WidgetUnmountReason, WidgetUpdateContext,
};

use super::{
    CapabilityCaches, DirtyPhases, InteractionState, MountedNodeId, apply_invalidation,
    arena::{MountedArena, MountedArenaCapacityError},
    node::{MountedNode, state_is_corrupted},
    reconcile::analyze_sibling_keys,
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

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum AutomationResolution {
    Unique(MountedNodeId),
    Missing,
    Ambiguous { matches: usize },
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
        let required = element_node_count(&root)?;
        let mut tree = Self::empty();
        tree.arena
            .preflight_live_count(required, public_slot_limit)?;
        let mut stats = ReconcileStats::default();
        let root_id = tree.mount_parts(None, root.into_runtime_parts(), &mut stats);
        tree.root = Some(root_id);
        Ok((tree, stats))
    }

    fn mount_parts(
        &mut self,
        parent: Option<MountedNodeId>,
        parts: ElementParts<Action>,
        stats: &mut ReconcileStats<Action>,
    ) -> MountedNodeId {
        let (
            authored_id,
            key,
            layout,
            style,
            focusability,
            focus_scope,
            authoring_diagnostics,
            widget,
            children,
        ) = parts.into_parts();
        let widget_state = widget.create_state();
        let runtime = self.runtime.clone();
        let (slot, generation) = self
            .arena
            .insert_with(|slot, generation| {
                let slot = checked_public_slot(slot)
                    .unwrap_or_else(|_| unreachable!("mounted arena exceeded public slot range"));
                let id = runtime.__runtime_mounted_id(slot, generation);
                MountedNode {
                    semantic_id: runtime.__runtime_semantic_id(slot, generation),
                    id,
                    parent,
                    children: Vec::new(),
                    authored_id,
                    key,
                    layout,
                    style,
                    focusability,
                    focus_scope,
                    authoring_diagnostics,
                    widget,
                    state: widget_state,
                    #[cfg(any(test, feature = "internal-test-seams"))]
                    state_corrupted: false,
                    interaction: InteractionState::default(),
                    integrity_failed: false,
                    caches: CapabilityCaches::default(),
                    dirty_phases: DirtyPhases::ALL,
                }
            })
            .unwrap_or_else(|_| unreachable!("mounted identity capacity was preflighted"));
        let public_slot = checked_public_slot(slot)
            .unwrap_or_else(|_| unreachable!("mounted arena exceeded public slot range"));
        let id = self.runtime.__runtime_mounted_id(public_slot, generation);
        stats.mounted += 1;

        let mut context = WidgetMountContext::__runtime_new();
        if let Some(node) = self.arena.get_mut(slot, generation) {
            if node.widget.mount(&mut node.state, &mut context).is_err() {
                mark_mismatch(node, "mount", stats);
            }
            apply_invalidation(node, context.__runtime_take_invalidation());
        }
        let outputs = context.__runtime_take_outputs();
        if !outputs.is_empty() {
            stats.mounted_outputs.push((id.clone(), outputs));
        }
        stats.mounted_owners.push(id.clone());

        let mounted_children = children
            .into_iter()
            .map(|child| self.mount_parts(Some(id.clone()), child.into_runtime_parts(), stats))
            .collect();
        self.node_mut(&id)
            .unwrap_or_else(|| unreachable!("new mounted node must remain live"))
            .children = mounted_children;
        id
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

    pub(crate) fn reconcile_with_before_unmount_and_public_slot_limit(
        &mut self,
        root: Element<Action>,
        public_slot_limit: u64,
        before_unmount: &mut dyn FnMut(&MountedNodeId),
    ) -> Result<ReconcileStats<Action>, MountedIdentityExhausted> {
        let required = element_node_count(&root)?;
        self.arena
            .preflight_live_count(required, public_slot_limit)?;
        let mut stats = ReconcileStats::default();
        let parts = root.into_runtime_parts();
        let old_root = self
            .root
            .clone()
            .unwrap_or_else(|| unreachable!("mounted tree has a root before shutdown"));
        if self.compatible(&old_root, &parts) {
            if let Err(parts) =
                self.update_node(&old_root, parts, "root", &mut stats, before_unmount)
            {
                self.unmount_subtree(
                    &old_root,
                    WidgetUnmountReason::Replaced,
                    "root",
                    &mut stats,
                    before_unmount,
                );
                self.root = Some(self.mount_parts(None, *parts, &mut stats));
            }
        } else {
            self.unmount_subtree(
                &old_root,
                WidgetUnmountReason::Replaced,
                "root",
                &mut stats,
                before_unmount,
            );
            self.root = Some(self.mount_parts(None, parts, &mut stats));
        }
        Ok(stats)
    }

    fn compatible(&self, id: &MountedNodeId, parts: &ElementParts<Action>) -> bool {
        let Some(node) = self.node(id) else {
            return false;
        };
        !node.integrity_failed
            && node.key.as_ref() == parts.key()
            && node.widget.widget_type_id() == parts.widget().widget_type_id()
            && node.widget.state_type_id() == parts.widget().state_type_id()
    }

    fn update_node(
        &mut self,
        id: &MountedNodeId,
        parts: ElementParts<Action>,
        path: &str,
        stats: &mut ReconcileStats<Action>,
        before_unmount: &mut dyn FnMut(&MountedNodeId),
    ) -> Result<(), Box<ElementParts<Action>>> {
        let mut update_context = WidgetUpdateContext::__runtime_new();
        {
            let node = self
                .node_mut(id)
                .unwrap_or_else(|| unreachable!("compatible node is live"));
            if state_is_corrupted(node)
                || parts
                    .widget()
                    .update(&mut node.state, &mut update_context)
                    .is_err()
            {
                node.integrity_failed = true;
                stats
                    .diagnostics
                    .push(ReconciliationDiagnostic::StatePayloadMismatch {
                        path: path.to_owned(),
                    });
                return Err(Box::new(parts));
            }
        }
        let (
            authored_id,
            key,
            layout,
            style,
            focusability,
            focus_scope,
            authoring_diagnostics,
            widget,
            children,
        ) = parts.into_parts();
        let old_children;
        let common_invalidation;
        {
            let node = self
                .node_mut(id)
                .unwrap_or_else(|| unreachable!("compatible node is live"));
            let tree_metadata_changed = node.authored_id != authored_id;
            let style_changed = node.style != style;
            common_invalidation = common_field_invalidation(
                node,
                authored_id.as_ref(),
                layout,
                &style,
                focusability,
                focus_scope,
                &authoring_diagnostics,
            );
            old_children = node.children.clone();
            node.authored_id = authored_id;
            node.key = key;
            node.layout = layout;
            node.style = style;
            node.focusability = focusability;
            node.focus_scope = focus_scope;
            node.authoring_diagnostics = authoring_diagnostics;
            node.widget = widget;
            apply_invalidation(
                node,
                update_context.__runtime_take_invalidation() | common_invalidation,
            );
            if tree_metadata_changed {
                node.dirty_phases.insert(DirtyPhases::TREE);
            }
            if style_changed {
                node.dirty_phases.insert(DirtyPhases::STYLE);
            }
        }
        if update_context.__runtime_take_subscription_invalidation() {
            stats.subscription_invalidated.push(id.clone());
        }
        let outputs = update_context.__runtime_take_outputs();
        if !outputs.is_empty() {
            stats.mounted_outputs.push((id.clone(), outputs));
        }
        stats.updated += 1;
        let new_children =
            self.reconcile_children(id, &old_children, children, path, stats, before_unmount);
        let structural = old_children != new_children;
        let node = self
            .node_mut(id)
            .unwrap_or_else(|| unreachable!("updated node remains live"));
        node.children = new_children;
        if structural {
            apply_invalidation(node, WidgetInvalidation::LAYOUT);
            node.dirty_phases.insert(DirtyPhases::TREE);
        }
        Ok(())
    }

    fn reconcile_children(
        &mut self,
        parent: &MountedNodeId,
        old_children: &[MountedNodeId],
        children: Vec<Element<Action>>,
        parent_path: &str,
        stats: &mut ReconcileStats<Action>,
        before_unmount: &mut dyn FnMut(&MountedNodeId),
    ) -> Vec<MountedNodeId> {
        let new_parts: Vec<_> = children
            .into_iter()
            .map(Element::into_runtime_parts)
            .collect();
        let sibling_matches =
            analyze_sibling_keys(self, old_children, &new_parts, parent_path, stats);
        let old_keys = sibling_matches.old_keys;
        let old_unkeyed = sibling_matches.old_unkeyed;
        let new_keys = sibling_matches.new_keys;

        let mut unkeyed_ordinal = 0usize;
        let candidates: Vec<_> = new_parts
            .iter()
            .map(|parts| {
                if let Some(key) = parts.key() {
                    let old = old_keys.get(key);
                    let new = new_keys.get(key);
                    match (old, new) {
                        (Some(old), Some(new)) if old.len() == 1 && new.len() == 1 => {
                            Some(old[0].clone())
                        }
                        _ => None,
                    }
                } else {
                    let candidate = old_unkeyed.get(unkeyed_ordinal).cloned();
                    unkeyed_ordinal += 1;
                    candidate
                }
            })
            .collect();
        let used: HashSet<_> = candidates
            .iter()
            .flatten()
            .map(|(_, id)| id.clone())
            .collect();

        // Release slots that cannot participate in the new sibling set before
        // mounting additions. Complete live-count preflight above guarantees
        // that subsequent insertions cannot exhaust public identity capacity.
        for (old_position, old) in old_children.iter().enumerate() {
            if !used.contains(old) {
                self.unmount_subtree(
                    old,
                    WidgetUnmountReason::Removed,
                    &format!("{parent_path}/{old_position}"),
                    stats,
                    before_unmount,
                );
            }
        }

        let mut final_children = Vec::with_capacity(new_parts.len());
        for (new_position, (parts, candidate)) in new_parts.into_iter().zip(candidates).enumerate()
        {
            let child_path = format!("{parent_path}/{new_position}");
            if let Some((old_position, old_id)) = candidate {
                if self.compatible(&old_id, &parts) {
                    match self.update_node(&old_id, parts, &child_path, stats, before_unmount) {
                        Ok(()) => {
                            if old_position != new_position {
                                stats.moved += 1;
                            }
                            final_children.push(old_id);
                        }
                        Err(parts) => {
                            self.unmount_subtree(
                                &old_id,
                                WidgetUnmountReason::Replaced,
                                &child_path,
                                stats,
                                before_unmount,
                            );
                            final_children.push(self.mount_parts(
                                Some(parent.clone()),
                                *parts,
                                stats,
                            ));
                        }
                    }
                } else {
                    self.unmount_subtree(
                        &old_id,
                        WidgetUnmountReason::Replaced,
                        &child_path,
                        stats,
                        before_unmount,
                    );
                    final_children.push(self.mount_parts(Some(parent.clone()), parts, stats));
                }
            } else {
                final_children.push(self.mount_parts(Some(parent.clone()), parts, stats));
            }
        }
        final_children
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
            .filter(|id| {
                self.node(id).and_then(|node| node.authored_id.as_ref()) == Some(authored_id)
            })
            .collect();
        match matches.as_slice() {
            [] => AutomationResolution::Missing,
            [id] => AutomationResolution::Unique(id.clone()),
            many => AutomationResolution::Ambiguous {
                matches: many.len(),
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
        node.interaction = InteractionState {
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
        node.caches = CapabilityCaches::default();
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

fn element_node_count<Action>(root: &Element<Action>) -> Result<usize, MountedIdentityExhausted> {
    root.children().iter().try_fold(1usize, |count, child| {
        count
            .checked_add(element_node_count(child)?)
            .ok_or(MountedIdentityExhausted)
    })
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct PublicSlotOverflow;

fn checked_public_slot(slot: usize) -> Result<u32, PublicSlotOverflow> {
    u32::try_from(slot).map_err(|_| PublicSlotOverflow)
}

fn common_field_invalidation<Action>(
    node: &MountedNode<Action>,
    authored_id: Option<&ElementId>,
    layout: runenui_core::LayoutStyle,
    style: &runenui_core::StyleIntent,
    focusability: runenui_core::Focusability,
    focus_scope: Option<runenui_core::FocusScope>,
    diagnostics: &[runenui_core::AuthoringDiagnostic],
) -> WidgetInvalidation {
    let mut invalidation = WidgetInvalidation::NONE;
    if node.layout != layout || node.style.padding() != style.padding() {
        invalidation |= WidgetInvalidation::LAYOUT;
    }
    if node.style.foreground() != style.foreground()
        || node.style.background() != style.background()
        || node.style.radius() != style.radius()
    {
        invalidation |= WidgetInvalidation::PAINT;
    }
    if node.authored_id.as_ref() != authored_id || node.authoring_diagnostics != diagnostics {
        invalidation |= WidgetInvalidation::DIAGNOSTICS;
    }
    if node.focusability != focusability || node.focus_scope != focus_scope {
        invalidation |= WidgetInvalidation::INTERACTION;
    }
    invalidation
}

fn mark_mismatch<Action>(
    node: &mut MountedNode<Action>,
    path: &str,
    stats: &mut ReconcileStats<Action>,
) {
    node.integrity_failed = true;
    stats
        .diagnostics
        .push(ReconciliationDiagnostic::StatePayloadMismatch {
            path: path.to_owned(),
        });
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
