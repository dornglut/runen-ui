//! Runtime-owned focus state, scope membership, and candidate selection.

use std::collections::{HashMap, HashSet};

use runenui_core::{
    FocusBoundaryPolicy, FocusDirection, FocusGroupActivationPolicy, FocusGroupBoundaryPolicy,
    FocusGroupEntry, FocusReason, FocusScope, FocusScopePolicy, Focusability, InputModality,
};

use crate::{LogicalRect, MountedNodeId, mounted::MountedTree};

/// Read-only inspection of the runtime's single focus authority.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct FocusState {
    focused_node_id: Option<MountedNodeId>,
    focused_route: Vec<MountedNodeId>,
    remembered: HashMap<MountedNodeId, MountedNodeId>,
    modality: Option<InputModality>,
    reason: Option<FocusReason>,
}

impl FocusState {
    #[must_use]
    pub(crate) fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub const fn focused_node(&self) -> Option<&MountedNodeId> {
        self.focused_node_id.as_ref()
    }

    #[must_use]
    pub fn is_focused(&self, id: &MountedNodeId) -> bool {
        self.focused_node_id.as_ref() == Some(id)
    }

    /// Returns whether this exact live route member currently contains focus.
    #[must_use]
    pub fn is_focus_within(&self, id: &MountedNodeId) -> bool {
        self.focused_route.iter().any(|ancestor| ancestor == id)
    }

    /// Returns the last accepted input modality, if any.
    #[must_use]
    pub const fn modality(&self) -> Option<InputModality> {
        self.modality
    }

    /// Returns the reason of the latest committed focus transition.
    #[must_use]
    pub const fn reason(&self) -> Option<FocusReason> {
        self.reason
    }

    pub(crate) fn set_modality(&mut self, modality: InputModality) -> Option<InputModality> {
        let old = self.modality.replace(modality);
        (old != Some(modality)).then_some(modality)
    }

    pub(crate) fn commit(
        &mut self,
        target: Option<MountedNodeId>,
        route: Vec<MountedNodeId>,
        reason: FocusReason,
    ) {
        self.focused_node_id = target;
        self.focused_route = route;
        self.reason = Some(reason);
    }

    pub(crate) fn remember(&mut self, scope: MountedNodeId, target: MountedNodeId) {
        self.remembered.insert(scope, target);
    }

    pub(crate) fn remembered(&self, scope: &MountedNodeId) -> Option<&MountedNodeId> {
        self.remembered.get(scope)
    }

    pub(crate) fn retain_remembered(
        &mut self,
        mut keep: impl FnMut(&MountedNodeId, &MountedNodeId) -> bool,
    ) {
        self.remembered.retain(|scope, target| keep(scope, target));
    }

    pub(crate) fn clear_all(&mut self, reason: FocusReason) {
        self.focused_node_id = None;
        self.focused_route.clear();
        self.remembered.clear();
        self.reason = Some(reason);
    }

    pub(crate) const fn route(&self) -> &[MountedNodeId] {
        self.focused_route.as_slice()
    }

    pub(crate) const fn route_len(&self) -> usize {
        self.focused_route.len()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FocusNavigation {
    Next,
    Previous,
    Direction(FocusDirection),
    Restore,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FocusBoundaryOutcome {
    Candidate,
    Delegate,
    Trap,
    Stop,
    Wrap,
    LogicalScroll,
    Empty,
}

pub struct FocusSelection {
    pub active_scope: MountedNodeId,
    pub policy: FocusScopePolicy,
    pub target: Option<MountedNodeId>,
    pub outcome: FocusBoundaryOutcome,
    pub scroll: Option<FocusDirection>,
    pub remembered_rejected: bool,
}

#[derive(Clone)]
struct Candidate {
    id: MountedNodeId,
    order: usize,
    rect: Option<LogicalRect>,
    group: Option<MountedNodeId>,
}

pub fn root_scope<Action>(tree: &MountedTree<Action>) -> Option<MountedNodeId> {
    tree.publication_preorder_ids().into_iter().next()
}

pub fn nearest_scope<Action>(
    tree: &MountedTree<Action>,
    id: &MountedNodeId,
) -> Option<MountedNodeId> {
    let root = root_scope(tree)?;
    let mut current = id.clone();
    loop {
        let node = tree.node(&current)?;
        if current == root || node.focus_scope.is_some() {
            return Some(current);
        }
        current = node.parent.clone()?;
    }
}

fn parent_scope<Action>(
    tree: &MountedTree<Action>,
    scope: &MountedNodeId,
) -> Option<MountedNodeId> {
    let mut current = tree.node(scope)?.parent.clone()?;
    loop {
        let node = tree.node(&current)?;
        if node.focus_scope.is_some() || node.parent.is_none() {
            return Some(current);
        }
        current = node.parent.clone()?;
    }
}

fn scope_policy<Action>(tree: &MountedTree<Action>, scope: &MountedNodeId) -> FocusScopePolicy {
    if tree.node(scope).is_some_and(|node| node.parent.is_none()) {
        FocusScopePolicy::new(FocusBoundaryPolicy::Wrap, FocusBoundaryPolicy::Stop)
    } else {
        tree.node(scope)
            .and_then(|node| node.focus_scope)
            .unwrap_or_default()
            .policy()
    }
}

fn scope_remembers<Action>(tree: &MountedTree<Action>, scope: &MountedNodeId) -> bool {
    tree.node(scope)
        .and_then(|node| node.focus_scope)
        .is_none_or(FocusScope::remembers_last)
}

pub fn is_focus_eligible<Action>(tree: &mut MountedTree<Action>, id: &MountedNodeId) -> bool {
    let focusability = match tree.node(id) {
        Some(node) => node.focusability,
        None => return false,
    };
    let Ok(activation) = tree.activation(id) else {
        return false;
    };
    activation.enabled()
        && match focusability {
            Focusability::Automatic => activation.is_actionable(),
            Focusability::Focusable => true,
            _ => false,
        }
}

fn nearest_group<Action>(tree: &MountedTree<Action>, id: &MountedNodeId) -> Option<MountedNodeId> {
    let mut current = tree.node(id)?.parent.clone()?;
    loop {
        let node = tree.node(&current)?;
        if node.focus_group.is_some() {
            return Some(current);
        }
        current = node.parent.clone()?;
    }
}

#[derive(Clone)]
struct FocusGroupMember {
    anchor: MountedNodeId,
    target: MountedNodeId,
}

fn is_within_group<Action>(
    tree: &MountedTree<Action>,
    id: &MountedNodeId,
    group: &MountedNodeId,
) -> bool {
    let mut current = Some(id.clone());
    while let Some(id) = current {
        if &id == group {
            return true;
        }
        current = tree.node(&id).and_then(|node| node.parent.clone());
    }
    false
}

struct FocusGroupMembers {
    members: Vec<FocusGroupMember>,
    preferred: Option<MountedNodeId>,
}

fn collect_focus_group_members<Action>(
    tree: &mut MountedTree<Action>,
    id: &MountedNodeId,
    members: &mut Vec<FocusGroupMember>,
    preferred: &mut Option<MountedNodeId>,
    multiple_preferred: &mut bool,
) {
    let Some(node) = tree.node(id) else {
        return;
    };
    let nested_group = node.focus_group.is_some();
    let entry = node.focus_group_entry;
    let children = node.children.clone();

    if entry == FocusGroupEntry::Preferred {
        if preferred.is_some() {
            *multiple_preferred = true;
        } else {
            *preferred = Some(id.clone());
        }
    }

    if nested_group {
        if let Some(target) = focus_group_entry_target(tree, id) {
            members.push(FocusGroupMember {
                anchor: id.clone(),
                target,
            });
        }
        return;
    }

    if is_focus_eligible(tree, id) {
        members.push(FocusGroupMember {
            anchor: id.clone(),
            target: id.clone(),
        });
    }

    for child in children {
        collect_focus_group_members(tree, &child, members, preferred, multiple_preferred);
    }
}

fn focus_group_members<Action>(
    tree: &mut MountedTree<Action>,
    group: &MountedNodeId,
) -> Option<FocusGroupMembers> {
    let children = tree.node(group)?.children.clone();
    let mut members = Vec::new();
    let mut preferred = None;
    let mut multiple_preferred = false;

    for child in children {
        collect_focus_group_members(
            tree,
            &child,
            &mut members,
            &mut preferred,
            &mut multiple_preferred,
        );
    }

    if multiple_preferred {
        return None;
    }

    Some(FocusGroupMembers { members, preferred })
}

fn focus_group_entry_target<Action>(
    tree: &mut MountedTree<Action>,
    group: &MountedNodeId,
) -> Option<MountedNodeId> {
    let resolved = focus_group_members(tree, group)?;
    if let Some(preferred) = resolved.preferred.as_ref()
        && let Some(member) = resolved
            .members
            .iter()
            .find(|member| &member.anchor == preferred)
    {
        return Some(member.target.clone());
    }
    resolved.members.first().map(|member| member.target.clone())
}

fn candidate_contains<Action>(
    tree: &MountedTree<Action>,
    candidate: &Candidate,
    id: &MountedNodeId,
) -> bool {
    candidate.id == *id
        || candidate
            .group
            .as_ref()
            .is_some_and(|group| is_within_group(tree, id, group))
}

fn candidates<Action>(
    tree: &mut MountedTree<Action>,
    scope: &MountedNodeId,
    geometry: &[(MountedNodeId, LogicalRect)],
) -> Vec<Candidate> {
    let ids = tree.publication_preorder_ids();
    let mut seen_groups = HashSet::new();
    let mut output = Vec::new();
    for (order, id) in ids.iter().cloned().enumerate() {
        if nearest_scope(tree, &id).as_ref() != Some(scope) {
            continue;
        }
        if let Some(group) = nearest_group(tree, &id) {
            let outermost_group = {
                let mut current = group.clone();
                loop {
                    let Some(parent_group) = nearest_group(tree, &current) else {
                        break current;
                    };
                    current = parent_group;
                }
            };
            if !seen_groups.insert(outermost_group.clone()) {
                continue;
            }
            let Some(target) = focus_group_entry_target(tree, &outermost_group) else {
                continue;
            };
            let group_order = ids
                .iter()
                .position(|candidate| candidate == &outermost_group)
                .unwrap_or(order);
            let rect = geometry
                .iter()
                .find_map(|(geometry_id, rect)| (geometry_id == &outermost_group).then_some(*rect));
            output.push(Candidate {
                id: target,
                order: group_order,
                rect,
                group: Some(outermost_group),
            });
            continue;
        }
        if tree
            .node(&id)
            .is_some_and(|node| node.focus_group.is_some())
        {
            if !seen_groups.insert(id.clone()) {
                continue;
            }
            let Some(target) = focus_group_entry_target(tree, &id) else {
                continue;
            };
            let rect = geometry
                .iter()
                .find_map(|(geometry_id, rect)| (geometry_id == &id).then_some(*rect));
            output.push(Candidate {
                id: target,
                order,
                rect,
                group: Some(id),
            });
            continue;
        }
        if is_focus_eligible(tree, &id) {
            output.push(Candidate {
                rect: geometry
                    .iter()
                    .find_map(|(geometry_id, rect)| (geometry_id == &id).then_some(*rect)),
                id,
                order,
                group: None,
            });
        }
    }
    output
}

pub struct FocusGroupSelection {
    pub target: Option<MountedNodeId>,
    pub activation: FocusGroupActivationPolicy,
}

fn focus_group_member_contains<Action>(
    tree: &MountedTree<Action>,
    member: &FocusGroupMember,
    id: &MountedNodeId,
) -> bool {
    member.target == *id
        || member.anchor == *id
        || tree
            .node(&member.anchor)
            .is_some_and(|node| node.focus_group.is_some())
            && is_within_group(tree, id, &member.anchor)
}

pub fn select_focus_group_member<Action>(
    tree: &mut MountedTree<Action>,
    state: &FocusState,
    command_target: &MountedNodeId,
    forward: bool,
) -> Option<FocusGroupSelection> {
    let current = state.focused_node().unwrap_or(command_target);
    let group = if tree
        .node(command_target)
        .is_some_and(|node| node.focus_group.is_some())
    {
        command_target.clone()
    } else {
        nearest_group(tree, current).or_else(|| nearest_group(tree, command_target))?
    };
    let config = tree.node(&group)?.focus_group?;
    let resolved = focus_group_members(tree, &group)?;
    let members = resolved.members;
    if members.is_empty() {
        return Some(FocusGroupSelection {
            target: None,
            activation: config.activation(),
        });
    }
    let position = members
        .iter()
        .position(|member| focus_group_member_contains(tree, member, current));
    let target = position.and_then(|position| {
        if forward {
            members
                .get(position + 1)
                .map(|member| member.target.clone())
        } else {
            position
                .checked_sub(1)
                .and_then(|previous| members.get(previous))
                .map(|member| member.target.clone())
        }
    });
    if target.is_some() {
        return Some(FocusGroupSelection {
            target,
            activation: config.activation(),
        });
    }
    let target = match config.boundary() {
        FocusGroupBoundaryPolicy::Wrap => {
            if forward {
                members.first().map(|member| member.target.clone())
            } else {
                members.last().map(|member| member.target.clone())
            }
        }
        FocusGroupBoundaryPolicy::Stop => None,
        _ => None,
    };
    Some(FocusGroupSelection {
        target,
        activation: config.activation(),
    })
}

pub fn select_focus<Action>(
    tree: &mut MountedTree<Action>,
    state: &FocusState,
    command_target: &MountedNodeId,
    navigation: FocusNavigation,
    geometry: &[(MountedNodeId, LogicalRect)],
) -> Option<FocusSelection> {
    let initial_scope = match navigation {
        FocusNavigation::Restore => nearest_scope(tree, command_target)?,
        _ => state
            .focused_node()
            .and_then(|focused| nearest_scope(tree, focused))
            .or_else(|| nearest_scope(tree, command_target))?,
    };
    select_in_scope(
        tree,
        state,
        command_target,
        navigation,
        geometry,
        initial_scope,
    )
}

fn select_in_scope<Action>(
    tree: &mut MountedTree<Action>,
    state: &FocusState,
    command_target: &MountedNodeId,
    navigation: FocusNavigation,
    geometry: &[(MountedNodeId, LogicalRect)],
    scope: MountedNodeId,
) -> Option<FocusSelection> {
    let policy = scope_policy(tree, &scope);
    let candidates = candidates(tree, &scope, geometry);
    if navigation == FocusNavigation::Restore {
        return Some(restore_selection(tree, state, scope, policy, &candidates));
    }

    let current = state.focused_node().unwrap_or(command_target);
    let selected = match navigation {
        FocusNavigation::Next => linear_candidate(tree, &candidates, current, true),
        FocusNavigation::Previous => linear_candidate(tree, &candidates, current, false),
        FocusNavigation::Direction(direction) => {
            directional_candidate(tree, &candidates, current, direction, geometry)
        }
        FocusNavigation::Restore => unreachable!("restoration handled above"),
    };
    if let Some(target) = selected {
        return Some(FocusSelection {
            active_scope: scope,
            policy,
            target: Some(target),
            outcome: FocusBoundaryOutcome::Candidate,
            scroll: None,
            remembered_rejected: false,
        });
    }
    let boundary = match navigation {
        FocusNavigation::Next | FocusNavigation::Previous => policy.linear(),
        FocusNavigation::Direction(_) => policy.directional(),
        FocusNavigation::Restore => unreachable!("restoration handled above"),
    };
    match boundary {
        FocusBoundaryPolicy::Delegate => {
            if let Some(parent) = parent_scope(tree, &scope) {
                let mut delegated =
                    select_in_scope(tree, state, command_target, navigation, geometry, parent)?;
                if delegated.target.is_some() {
                    delegated.outcome = FocusBoundaryOutcome::Delegate;
                }
                Some(delegated)
            } else {
                Some(no_target(scope, policy, FocusBoundaryOutcome::Stop))
            }
        }
        FocusBoundaryPolicy::Trap => Some(no_target(scope, policy, FocusBoundaryOutcome::Trap)),
        FocusBoundaryPolicy::Wrap => {
            let target = match navigation {
                FocusNavigation::Next => candidates.first(),
                FocusNavigation::Previous => candidates.last(),
                FocusNavigation::Direction(direction) => directional_wrap(&candidates, direction),
                FocusNavigation::Restore => None,
            }
            .map(|candidate| candidate.id.clone());
            Some(FocusSelection {
                active_scope: scope,
                policy,
                target,
                outcome: FocusBoundaryOutcome::Wrap,
                scroll: None,
                remembered_rejected: false,
            })
        }
        FocusBoundaryPolicy::LogicalScroll => Some(FocusSelection {
            active_scope: scope,
            policy,
            target: None,
            outcome: FocusBoundaryOutcome::LogicalScroll,
            scroll: match navigation {
                FocusNavigation::Direction(direction) => Some(direction),
                FocusNavigation::Next => Some(FocusDirection::Down),
                FocusNavigation::Previous => Some(FocusDirection::Up),
                FocusNavigation::Restore => None,
            },
            remembered_rejected: false,
        }),
        _ => Some(no_target(scope, policy, FocusBoundaryOutcome::Stop)),
    }
}

fn restore_selection<Action>(
    tree: &mut MountedTree<Action>,
    state: &FocusState,
    scope: MountedNodeId,
    policy: FocusScopePolicy,
    candidates: &[Candidate],
) -> FocusSelection {
    let remembered = state.remembered(&scope).cloned();
    if scope_remembers(tree, &scope)
        && let Some(remembered) = remembered.as_ref()
        && is_focus_eligible(tree, remembered)
        && candidates
            .iter()
            .any(|candidate| candidate_contains(tree, candidate, remembered))
    {
        return FocusSelection {
            active_scope: scope,
            policy,
            target: Some(remembered.clone()),
            outcome: FocusBoundaryOutcome::Candidate,
            scroll: None,
            remembered_rejected: false,
        };
    }
    FocusSelection {
        active_scope: scope,
        policy,
        target: candidates.first().map(|candidate| candidate.id.clone()),
        outcome: if candidates.is_empty() {
            FocusBoundaryOutcome::Empty
        } else {
            FocusBoundaryOutcome::Candidate
        },
        scroll: None,
        remembered_rejected: remembered.is_some(),
    }
}

const fn no_target(
    active_scope: MountedNodeId,
    policy: FocusScopePolicy,
    outcome: FocusBoundaryOutcome,
) -> FocusSelection {
    FocusSelection {
        active_scope,
        policy,
        target: None,
        outcome,
        scroll: None,
        remembered_rejected: false,
    }
}

fn linear_candidate<Action>(
    tree: &MountedTree<Action>,
    candidates: &[Candidate],
    current: &MountedNodeId,
    forward: bool,
) -> Option<MountedNodeId> {
    let current_order = candidates
        .iter()
        .find(|candidate| candidate_contains(tree, candidate, current))
        .map(|candidate| candidate.order);
    if forward {
        candidates
            .iter()
            .find(|candidate| current_order.is_none_or(|order| candidate.order > order))
    } else {
        candidates
            .iter()
            .rev()
            .find(|candidate| current_order.is_none_or(|order| candidate.order < order))
    }
    .map(|candidate| candidate.id.clone())
}

fn directional_candidate<Action>(
    tree: &MountedTree<Action>,
    candidates: &[Candidate],
    current: &MountedNodeId,
    direction: FocusDirection,
    geometry: &[(MountedNodeId, LogicalRect)],
) -> Option<MountedNodeId> {
    let origin = candidates
        .iter()
        .find(|candidate| candidate_contains(tree, candidate, current))
        .and_then(|candidate| candidate.rect)
        .or_else(|| {
            geometry
                .iter()
                .find_map(|(id, rect)| (id == current).then_some(*rect))
        })?;
    candidates
        .iter()
        .filter(|candidate| !candidate_contains(tree, candidate, current))
        .filter_map(|candidate| {
            let rect = candidate.rect?;
            directional_rank(origin, rect, direction).map(|rank| (rank, candidate))
        })
        .min_by(|(left, left_candidate), (right, right_candidate)| {
            left.partial_cmp(right)
                .unwrap_or(core::cmp::Ordering::Equal)
                .then_with(|| left_candidate.order.cmp(&right_candidate.order))
        })
        .map(|(_, candidate)| candidate.id.clone())
}

fn directional_rank(
    origin: LogicalRect,
    candidate: LogicalRect,
    direction: FocusDirection,
) -> Option<(u8, OrderedF32, OrderedF32, OrderedF32)> {
    let (primary_gap, orthogonal_gap, overlap) = match direction {
        FocusDirection::Right => {
            if candidate.max_x() <= origin.max_x() {
                return None;
            }
            (
                (candidate.x() - origin.max_x()).max(0.0),
                axis_gap(origin.y(), origin.max_y(), candidate.y(), candidate.max_y()),
                axis_overlap(origin.y(), origin.max_y(), candidate.y(), candidate.max_y()),
            )
        }
        FocusDirection::Left => {
            if candidate.x() >= origin.x() {
                return None;
            }
            (
                (origin.x() - candidate.max_x()).max(0.0),
                axis_gap(origin.y(), origin.max_y(), candidate.y(), candidate.max_y()),
                axis_overlap(origin.y(), origin.max_y(), candidate.y(), candidate.max_y()),
            )
        }
        FocusDirection::Down => {
            if candidate.max_y() <= origin.max_y() {
                return None;
            }
            (
                (candidate.y() - origin.max_y()).max(0.0),
                axis_gap(origin.x(), origin.max_x(), candidate.x(), candidate.max_x()),
                axis_overlap(origin.x(), origin.max_x(), candidate.x(), candidate.max_x()),
            )
        }
        FocusDirection::Up => {
            if candidate.y() >= origin.y() {
                return None;
            }
            (
                (origin.y() - candidate.max_y()).max(0.0),
                axis_gap(origin.x(), origin.max_x(), candidate.x(), candidate.max_x()),
                axis_overlap(origin.x(), origin.max_x(), candidate.x(), candidate.max_x()),
            )
        }
        _ => return None,
    };
    // Touching rectangle edges are inside the beam. A positive orthogonal gap
    // is therefore the authoritative off-beam discriminator.
    let beam = u8::from(orthogonal_gap > 0.0);
    Some((
        beam,
        OrderedF32(primary_gap),
        OrderedF32(orthogonal_gap),
        OrderedF32(-overlap),
    ))
}

fn axis_gap(a0: f32, a1: f32, b0: f32, b1: f32) -> f32 {
    if a1 < b0 {
        b0 - a1
    } else if b1 < a0 {
        a0 - b1
    } else {
        0.0
    }
}

fn axis_overlap(a0: f32, a1: f32, b0: f32, b1: f32) -> f32 {
    (a1.min(b1) - a0.max(b0)).max(0.0)
}

fn directional_wrap(candidates: &[Candidate], direction: FocusDirection) -> Option<&Candidate> {
    candidates
        .iter()
        .filter(|candidate| candidate.rect.is_some())
        .min_by(|left, right| {
            let left_rect = left
                .rect
                .unwrap_or_else(|| unreachable!("filtered rectangle"));
            let right_rect = right
                .rect
                .unwrap_or_else(|| unreachable!("filtered rectangle"));
            let left_edge = match direction {
                FocusDirection::Right => left_rect.x(),
                FocusDirection::Left => -left_rect.max_x(),
                FocusDirection::Down => left_rect.y(),
                FocusDirection::Up => -left_rect.max_y(),
                _ => 0.0,
            };
            let right_edge = match direction {
                FocusDirection::Right => right_rect.x(),
                FocusDirection::Left => -right_rect.max_x(),
                FocusDirection::Down => right_rect.y(),
                FocusDirection::Up => -right_rect.max_y(),
                _ => 0.0,
            };
            left_edge
                .partial_cmp(&right_edge)
                .unwrap_or(core::cmp::Ordering::Equal)
                .then_with(|| left.order.cmp(&right.order))
        })
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct OrderedF32(f32);

impl Eq for OrderedF32 {}

impl PartialOrd for OrderedF32 {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for OrderedF32 {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.0
            .partial_cmp(&other.0)
            .unwrap_or(core::cmp::Ordering::Equal)
    }
}
