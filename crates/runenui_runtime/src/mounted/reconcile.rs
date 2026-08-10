use std::collections::{BTreeMap, BTreeSet, HashSet};

use runenui_core::{
    __runtime::MountedWidget, Element, ElementId, ElementKey, FocusScope, Focusability,
    LayoutStyle, StyleIntent, WidgetInvalidation, WidgetMountContext, WidgetUnmountReason,
    WidgetUpdateContext,
};

use crate::ReconciliationDiagnostic;

use super::{
    CachedCapability, CapabilityCaches, DirtyPhases, InteractionState, MountedNodeId,
    apply_invalidation,
    invalidation::invalidate_semantic_structure,
    node::{MountedNode, state_is_corrupted},
    tree::{MountedIdentityExhausted, MountedTree, ReconcileStats, checked_public_slot},
};

/// Why a presently mounted lifetime must be revoked before reconciliation
/// applies its structural changes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum PlannedLifetimeReason {
    Removal,
    Replacement,
}

/// One exact mounted lifetime invalidated by a private reconciliation plan.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PlannedInvalidation {
    id: MountedNodeId,
    reason: PlannedLifetimeReason,
}

impl PlannedInvalidation {
    pub(crate) const fn id(&self) -> &MountedNodeId {
        &self.id
    }

    pub(crate) const fn reason(&self) -> PlannedLifetimeReason {
        self.reason
    }
}

/// Ephemeral reconciliation authority. It is deliberately neither mounted
/// state nor a public projection: planning consumes transient element input and
/// applying consumes this plan exactly once.
pub(crate) struct ReconciliationPlan<Action> {
    root: PlannedNode<Action>,
    invalidated: Vec<PlannedInvalidation>,
    diagnostics: Vec<ReconciliationDiagnostic>,
    moved: usize,
}

impl<Action> ReconciliationPlan<Action> {
    pub(crate) const fn invalidated_lifetimes(&self) -> &[PlannedInvalidation] {
        self.invalidated.as_slice()
    }
}

/// A checked bridge failure after planning has verified compatibility. This is
/// an integrity failure, not a silent fallback to a second matching path.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ReconciliationApplyError;

pub(super) struct IncomingNode<Action> {
    authored_id: Option<ElementId>,
    key: Option<ElementKey>,
    layout: LayoutStyle,
    style: StyleIntent,
    focusability: Focusability,
    focus_scope: Option<FocusScope>,
    authoring_diagnostics: Vec<runenui_core::AuthoringDiagnostic>,
    widget: MountedWidget<Action>,
    children: Vec<Self>,
}

impl<Action> IncomingNode<Action> {
    pub(super) fn from_element(element: Element<Action>) -> Self {
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
        ) = element.into_runtime_parts().into_parts();
        Self {
            authored_id,
            key,
            layout,
            style,
            focusability,
            focus_scope,
            authoring_diagnostics,
            widget,
            children: children.into_iter().map(Self::from_element).collect(),
        }
    }

    pub(super) fn node_count(&self) -> Result<usize, MountedIdentityExhausted> {
        self.children.iter().try_fold(1usize, |count, child| {
            count
                .checked_add(child.node_count()?)
                .ok_or(MountedIdentityExhausted)
        })
    }
}

enum PlannedNode<Action> {
    Retain {
        id: MountedNodeId,
        path: String,
        incoming: IncomingNode<Action>,
        old_children: Vec<MountedNodeId>,
        removals: Vec<PlannedRemoval>,
        children: Vec<Self>,
    },
    Mount {
        parent: Option<MountedNodeId>,
        incoming: IncomingNode<Action>,
    },
    Replace {
        old: MountedNodeId,
        parent: Option<MountedNodeId>,
        path: String,
        incoming: IncomingNode<Action>,
    },
}

struct PlannedRemoval {
    id: MountedNodeId,
    path: String,
}

/// Scratch authority accumulated while one transient tree is planned. Keeping
/// these facts together prevents the planning recursion from acquiring a
/// second, partially independent reconciliation state.
struct PlanningState {
    invalidated: Vec<PlannedInvalidation>,
    diagnostics: Vec<ReconciliationDiagnostic>,
    moved: usize,
}

impl<Action> MountedTree<Action> {
    pub(crate) fn plan_reconciliation(
        &self,
        root: Element<Action>,
        public_slot_limit: u64,
    ) -> Result<ReconciliationPlan<Action>, MountedIdentityExhausted> {
        let root = IncomingNode::from_element(root);
        self.arena
            .preflight_live_count(root.node_count()?, public_slot_limit)?;
        let old_root = self
            .root
            .as_ref()
            .unwrap_or_else(|| unreachable!("mounted tree has a root before shutdown"));
        let mut planning = PlanningState {
            invalidated: Vec::new(),
            diagnostics: Vec::new(),
            moved: 0,
        };
        let root = self.plan_existing(Some(old_root), None, root, "root".to_owned(), &mut planning);
        Ok(ReconciliationPlan {
            root,
            invalidated: planning.invalidated,
            diagnostics: planning.diagnostics,
            moved: planning.moved,
        })
    }

    fn plan_existing(
        &self,
        existing: Option<&MountedNodeId>,
        parent: Option<MountedNodeId>,
        incoming: IncomingNode<Action>,
        path: String,
        planning: &mut PlanningState,
    ) -> PlannedNode<Action> {
        let Some(existing) = existing else {
            return PlannedNode::Mount { parent, incoming };
        };
        if !self.plan_compatible(existing, &incoming) {
            if self.plan_bridge_incompatible(existing, &incoming) {
                planning
                    .diagnostics
                    .push(ReconciliationDiagnostic::StatePayloadMismatch { path: path.clone() });
            }
            self.collect_invalidated_subtree(
                existing,
                PlannedLifetimeReason::Replacement,
                &mut planning.invalidated,
            );
            return PlannedNode::Replace {
                old: existing.clone(),
                parent,
                path,
                incoming,
            };
        }

        let old_children = self.node(existing).map_or_else(
            || unreachable!("planned retained node remains live"),
            |node| node.children.clone(),
        );
        let IncomingNode {
            authored_id,
            key,
            layout,
            style,
            focusability,
            focus_scope,
            authoring_diagnostics,
            widget,
            children,
        } = incoming;
        let (removals, children) =
            self.plan_children(existing, &old_children, children, &path, planning);
        PlannedNode::Retain {
            id: existing.clone(),
            path,
            incoming: IncomingNode {
                authored_id,
                key,
                layout,
                style,
                focusability,
                focus_scope,
                authoring_diagnostics,
                widget,
                children: Vec::new(),
            },
            old_children,
            removals,
            children,
        }
    }

    fn plan_children(
        &self,
        parent: &MountedNodeId,
        old_children: &[MountedNodeId],
        children: Vec<IncomingNode<Action>>,
        parent_path: &str,
        planning: &mut PlanningState,
    ) -> (Vec<PlannedRemoval>, Vec<PlannedNode<Action>>) {
        let SiblingMatches {
            old_keys,
            old_unkeyed,
            new_keys,
        } = analyze_sibling_keys(
            self,
            old_children,
            &children,
            parent_path,
            &mut planning.diagnostics,
        );

        let mut unkeyed_ordinal = 0usize;
        let candidates: Vec<_> = children
            .iter()
            .map(|incoming| {
                incoming.key.as_ref().map_or_else(
                    || {
                        let candidate = old_unkeyed.get(unkeyed_ordinal).cloned();
                        unkeyed_ordinal += 1;
                        candidate
                    },
                    |key| match (old_keys.get(key), new_keys.get(key)) {
                        (Some(old), Some(new)) if old.len() == 1 && new.len() == 1 => {
                            Some(old[0].clone())
                        }
                        _ => None,
                    },
                )
            })
            .collect();
        let used: HashSet<_> = candidates
            .iter()
            .flatten()
            .map(|(_, id)| id.clone())
            .collect();
        let mut removals = Vec::new();
        for (old_position, old) in old_children.iter().enumerate() {
            if !used.contains(old) {
                self.collect_invalidated_subtree(
                    old,
                    PlannedLifetimeReason::Removal,
                    &mut planning.invalidated,
                );
                removals.push(PlannedRemoval {
                    id: old.clone(),
                    path: format!("{parent_path}/{old_position}"),
                });
            }
        }
        let children = children
            .into_iter()
            .zip(candidates)
            .enumerate()
            .map(|(new_position, (incoming, candidate))| {
                let path = format!("{parent_path}/{new_position}");
                let planned = self.plan_existing(
                    candidate.as_ref().map(|(_, id)| id),
                    Some(parent.clone()),
                    incoming,
                    path,
                    planning,
                );
                if candidate.is_some_and(|(old_position, _)| old_position != new_position)
                    && matches!(&planned, PlannedNode::Retain { .. })
                {
                    planning.moved += 1;
                }
                planned
            })
            .collect();
        (removals, children)
    }

    fn plan_compatible(&self, id: &MountedNodeId, incoming: &IncomingNode<Action>) -> bool {
        let Some(node) = self.node(id) else {
            return false;
        };
        !node.integrity_failed
            && !state_is_corrupted(node)
            && node.key.as_ref() == incoming.key.as_ref()
            && node.widget.widget_type_id() == incoming.widget.widget_type_id()
            && node.widget.state_type_id() == incoming.widget.state_type_id()
            && node.widget.event_bridge_matches(&node.state)
            && incoming.widget.event_bridge_matches(&node.state)
    }

    fn plan_bridge_incompatible(
        &self,
        id: &MountedNodeId,
        incoming: &IncomingNode<Action>,
    ) -> bool {
        self.node(id).is_some_and(|node| {
            state_is_corrupted(node)
                || (node.widget.widget_type_id() == incoming.widget.widget_type_id()
                    && node.widget.state_type_id() == incoming.widget.state_type_id()
                    && (!node.widget.event_bridge_matches(&node.state)
                        || !incoming.widget.event_bridge_matches(&node.state)))
        })
    }

    fn collect_invalidated_subtree(
        &self,
        id: &MountedNodeId,
        reason: PlannedLifetimeReason,
        invalidated: &mut Vec<PlannedInvalidation>,
    ) {
        let Some(node) = self.node(id) else {
            return;
        };
        invalidated.push(PlannedInvalidation {
            id: id.clone(),
            reason,
        });
        for child in &node.children {
            self.collect_invalidated_subtree(child, reason, invalidated);
        }
    }

    pub(crate) fn apply_reconciliation(
        &mut self,
        plan: ReconciliationPlan<Action>,
        before_unmount: &mut dyn FnMut(&MountedNodeId),
    ) -> Result<ReconcileStats<Action>, ReconciliationApplyError> {
        let ReconciliationPlan {
            root,
            invalidated: _,
            diagnostics,
            moved,
        } = plan;
        let mut stats = ReconcileStats {
            moved,
            diagnostics,
            ..ReconcileStats::default()
        };
        let root = self.apply_planned_node(root, &mut stats, before_unmount)?;
        self.root = Some(root);
        Ok(stats)
    }

    fn apply_planned_node(
        &mut self,
        plan: PlannedNode<Action>,
        stats: &mut ReconcileStats<Action>,
        before_unmount: &mut dyn FnMut(&MountedNodeId),
    ) -> Result<MountedNodeId, ReconciliationApplyError> {
        match plan {
            PlannedNode::Mount { parent, incoming } => {
                Ok(self.mount_incoming(parent, incoming, stats))
            }
            PlannedNode::Replace {
                old,
                parent,
                path,
                incoming,
            } => {
                self.unmount_subtree(
                    &old,
                    WidgetUnmountReason::Replaced,
                    &path,
                    stats,
                    before_unmount,
                );
                Ok(self.mount_incoming(parent, incoming, stats))
            }
            PlannedNode::Retain {
                id,
                path,
                incoming,
                old_children,
                removals,
                children,
            } => {
                self.update_retained_node(&id, incoming, &path, stats)?;
                for removal in removals {
                    self.unmount_subtree(
                        &removal.id,
                        WidgetUnmountReason::Removed,
                        &removal.path,
                        stats,
                        before_unmount,
                    );
                }
                let new_children = children
                    .into_iter()
                    .map(|child| self.apply_planned_node(child, stats, before_unmount))
                    .collect::<Result<Vec<_>, _>>()?;
                let structural = old_children != new_children;
                let node = self
                    .node_mut(&id)
                    .unwrap_or_else(|| unreachable!("retained node remains live during apply"));
                node.children = new_children;
                if structural {
                    apply_invalidation(node, WidgetInvalidation::LAYOUT);
                    invalidate_semantic_structure(node);
                    node.dirty_phases.insert(DirtyPhases::TREE);
                }
                Ok(id)
            }
        }
    }

    fn update_retained_node(
        &mut self,
        id: &MountedNodeId,
        incoming: IncomingNode<Action>,
        path: &str,
        stats: &mut ReconcileStats<Action>,
    ) -> Result<(), ReconciliationApplyError> {
        let IncomingNode {
            authored_id,
            key,
            layout,
            style,
            focusability,
            focus_scope,
            authoring_diagnostics,
            widget,
            children: _,
        } = incoming;
        let mut update_context = WidgetUpdateContext::__runtime_new();
        {
            let node = self
                .node_mut(id)
                .unwrap_or_else(|| unreachable!("planned retained node remains live"));
            if state_is_corrupted(node)
                || !widget.event_bridge_matches(&node.state)
                || widget.update(&mut node.state, &mut update_context).is_err()
            {
                node.integrity_failed = true;
                stats
                    .diagnostics
                    .push(ReconciliationDiagnostic::StatePayloadMismatch {
                        path: path.to_owned(),
                    });
                return Err(ReconciliationApplyError);
            }
        }
        let common_invalidation;
        {
            let node = self
                .node_mut(id)
                .unwrap_or_else(|| unreachable!("planned retained node remains live"));
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
            node.authored_id = authored_id;
            node.key = key;
            node.layout = layout;
            node.style = style;
            node.focusability = focusability;
            node.focus_scope = focus_scope;
            node.authoring_diagnostics = authoring_diagnostics;
            node.widget = widget;
            // Capability declarations belong to the incoming widget instance,
            // not the retained state. A compatible update therefore cannot
            // reuse stale input or semantic contribution caches even when the
            // widget omitted those invalidations.
            node.caches.activation = CachedCapability::Unresolved;
            node.caches.text_input = CachedCapability::Unresolved;
            apply_invalidation(
                node,
                update_context.__runtime_take_invalidation() | common_invalidation,
            );
            invalidate_semantic_structure(node);
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
        Ok(())
    }

    pub(super) fn mount_incoming(
        &mut self,
        parent: Option<MountedNodeId>,
        incoming: IncomingNode<Action>,
        stats: &mut ReconcileStats<Action>,
    ) -> MountedNodeId {
        let IncomingNode {
            authored_id,
            key,
            layout,
            style,
            focusability,
            focus_scope,
            authoring_diagnostics,
            widget,
            children,
        } = incoming;
        let widget_state = widget.create_state();
        let runtime = self.runtime.clone();
        let (slot, generation) = self
            .arena
            .insert_with(|slot, generation| {
                let slot = checked_public_slot(slot)
                    .unwrap_or_else(|_| unreachable!("mounted arena exceeded public slot range"));
                let id = runtime.__runtime_mounted_id(slot, generation);
                MountedNode {
                    id,
                    semantic_bindings: Vec::new(),
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
            .map(|child| self.mount_incoming(Some(id.clone()), child, stats))
            .collect();
        self.node_mut(&id)
            .unwrap_or_else(|| unreachable!("new mounted node remains live"))
            .children = mounted_children;
        id
    }
}

struct SiblingMatches {
    old_keys: BTreeMap<ElementKey, Vec<(usize, MountedNodeId)>>,
    old_unkeyed: Vec<(usize, MountedNodeId)>,
    new_keys: BTreeMap<ElementKey, Vec<usize>>,
}

fn analyze_sibling_keys<Action>(
    tree: &MountedTree<Action>,
    old_children: &[MountedNodeId],
    new_children: &[IncomingNode<Action>],
    parent_path: &str,
    diagnostics: &mut Vec<ReconciliationDiagnostic>,
) -> SiblingMatches {
    let mut old_keys: BTreeMap<ElementKey, Vec<(usize, MountedNodeId)>> = BTreeMap::new();
    let mut old_unkeyed = Vec::new();
    for (position, id) in old_children.iter().enumerate() {
        match tree.node(id).and_then(|node| node.key.as_ref()) {
            Some(key) => old_keys
                .entry(key.clone())
                .or_default()
                .push((position, id.clone())),
            None => old_unkeyed.push((position, id.clone())),
        }
    }
    let mut new_keys: BTreeMap<ElementKey, Vec<usize>> = BTreeMap::new();
    for (position, incoming) in new_children.iter().enumerate() {
        if let Some(key) = incoming.key.as_ref() {
            new_keys.entry(key.clone()).or_default().push(position);
        }
    }
    let all_keys: BTreeSet<_> = old_keys.keys().chain(new_keys.keys()).cloned().collect();
    for key in all_keys {
        let old = old_keys.get(&key).map(Vec::as_slice).unwrap_or_default();
        let new = new_keys.get(&key).map(Vec::as_slice).unwrap_or_default();
        if old.len() > 1 || new.len() > 1 {
            diagnostics.push(ReconciliationDiagnostic::DuplicateSiblingKey {
                key,
                parent_path: parent_path.to_owned(),
                old_occurrence_paths: old
                    .iter()
                    .map(|(position, _)| format!("{parent_path}/{position}"))
                    .collect(),
                new_occurrence_paths: new
                    .iter()
                    .map(|position| format!("{parent_path}/{position}"))
                    .collect(),
            });
        }
    }
    SiblingMatches {
        old_keys,
        old_unkeyed,
        new_keys,
    }
}

fn common_field_invalidation<Action>(
    node: &MountedNode<Action>,
    authored_id: Option<&ElementId>,
    layout: LayoutStyle,
    style: &StyleIntent,
    focusability: Focusability,
    focus_scope: Option<FocusScope>,
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
