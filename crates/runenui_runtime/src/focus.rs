//! Runtime-owned focus state, scope membership, and candidate selection.

use std::collections::HashMap;

use runenui_core::{
    FocusBoundaryPolicy, FocusDirection, FocusReason, FocusScope, FocusScopePolicy, Focusability,
    InputModality,
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

fn candidates<Action>(
    tree: &mut MountedTree<Action>,
    scope: &MountedNodeId,
    geometry: &[(MountedNodeId, LogicalRect)],
) -> Vec<Candidate> {
    tree.publication_preorder_ids()
        .into_iter()
        .enumerate()
        .filter_map(|(order, id)| {
            (nearest_scope(tree, &id).as_ref() == Some(scope) && is_focus_eligible(tree, &id)).then(
                || Candidate {
                    rect: geometry
                        .iter()
                        .find_map(|(geometry_id, rect)| (geometry_id == &id).then_some(*rect)),
                    id,
                    order,
                },
            )
        })
        .collect()
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
        FocusNavigation::Next => linear_candidate(&candidates, current, true),
        FocusNavigation::Previous => linear_candidate(&candidates, current, false),
        FocusNavigation::Direction(direction) => {
            directional_candidate(&candidates, current, direction, geometry)
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
    tree: &MountedTree<Action>,
    state: &FocusState,
    scope: MountedNodeId,
    policy: FocusScopePolicy,
    candidates: &[Candidate],
) -> FocusSelection {
    let remembered = state.remembered(&scope).cloned();
    if scope_remembers(tree, &scope)
        && let Some(remembered) = remembered.as_ref()
        && candidates
            .iter()
            .any(|candidate| &candidate.id == remembered)
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

fn linear_candidate(
    candidates: &[Candidate],
    current: &MountedNodeId,
    forward: bool,
) -> Option<MountedNodeId> {
    let current_order = candidates
        .iter()
        .find(|candidate| &candidate.id == current)
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

fn directional_candidate(
    candidates: &[Candidate],
    current: &MountedNodeId,
    direction: FocusDirection,
    geometry: &[(MountedNodeId, LogicalRect)],
) -> Option<MountedNodeId> {
    let origin = candidates
        .iter()
        .find(|candidate| &candidate.id == current)
        .and_then(|candidate| candidate.rect)
        .or_else(|| {
            geometry
                .iter()
                .find_map(|(id, rect)| (id == current).then_some(*rect))
        })?;
    candidates
        .iter()
        .filter(|candidate| &candidate.id != current)
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
