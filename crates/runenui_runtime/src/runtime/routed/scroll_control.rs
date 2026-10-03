use runenui_core::{
    Axis, HostProtocol, OverflowPolicy, ScrollControlBinding, ScrollControlSnapshot,
};

use super::super::Runtime;
use crate::{
    MountedNodeId, TraceScrollControlBindingOutcome, mounted::DirtyPhases,
    surface::ScrollControlProjectionLookup,
};

pub(super) struct ResolvedScrollControl {
    pub(super) owner: MountedNodeId,
    pub(super) binding: ScrollControlBinding,
    pub(super) snapshot: ScrollControlSnapshot,
}

struct AcceptedScrollControlProjection {
    owner: MountedNodeId,
    binding: ScrollControlBinding,
    snapshot: ScrollControlSnapshot,
}

pub(super) struct ScrollControlResolutionFailure {
    pub(super) axis: Option<Axis>,
    pub(super) outcome: TraceScrollControlBindingOutcome,
    pub(super) owner: Option<MountedNodeId>,
}

const fn scroll_control_failure(
    axis: Axis,
    owner: Option<MountedNodeId>,
    outcome: TraceScrollControlBindingOutcome,
) -> ScrollControlResolutionFailure {
    ScrollControlResolutionFailure {
        axis: Some(axis),
        outcome,
        owner,
    }
}

impl<State, Action, Protocol: HostProtocol> Runtime<State, Action, Protocol> {
    pub(super) fn resolve_scroll_control(
        &self,
        target: &MountedNodeId,
    ) -> Result<ResolvedScrollControl, ScrollControlResolutionFailure> {
        let accepted = self.resolve_scroll_control_projection(target)?;
        let snapshot =
            self.resolve_scroll_control_snapshot(&accepted.owner, accepted.binding)?;
        Ok(ResolvedScrollControl {
            owner: accepted.owner,
            binding: accepted.binding,
            snapshot,
        })
    }

    pub(super) fn resolve_scroll_control_context_snapshot(
        &self,
        target: &MountedNodeId,
    ) -> Option<ScrollControlSnapshot> {
        self.resolve_scroll_control_projection(target)
            .ok()
            .map(|projection| projection.snapshot)
    }

    fn resolve_scroll_control_projection(
        &self,
        target: &MountedNodeId,
    ) -> Result<AcceptedScrollControlProjection, ScrollControlResolutionFailure> {
        let Some(binding) = self
            .tree
            .node(target)
            .and_then(|node| node.scroll_control_binding)
        else {
            return Err(ScrollControlResolutionFailure {
                axis: None,
                outcome: TraceScrollControlBindingOutcome::MissingBinding,
                owner: None,
            });
        };
        let route = self.tree.event_route(target).map_err(|_| {
            scroll_control_failure(
                binding.axis(),
                None,
                TraceScrollControlBindingOutcome::MetricsUnavailable,
            )
        })?;
        let current_owner = self.current_scroll_owner(&route, binding.axis());
        let projection = self.resolve_published_scroll_control_projection(
            target,
            binding,
            current_owner.as_ref(),
        )?;
        if self.scroll_control_metrics_are_stale() {
            return Err(scroll_control_failure(
                binding.axis(),
                Some(projection.owner),
                TraceScrollControlBindingOutcome::MetricsUnavailable,
            ));
        }
        Ok(projection)
    }

    fn scroll_control_metrics_are_stale(&self) -> bool {
        let pending = self.tree.pending_phases();
        pending.contains(DirtyPhases::TREE)
            || pending.contains(DirtyPhases::STYLE)
            || pending.contains(DirtyPhases::LAYOUT)
            || pending.contains(DirtyPhases::MOTION)
    }

    fn current_scroll_owner(&self, route: &[MountedNodeId], axis: Axis) -> Option<MountedNodeId> {
        route.iter().rev().skip(1).find_map(|candidate| {
            self.tree.node(candidate).and_then(|node| {
                let policy = match axis {
                    Axis::Horizontal => node.layout.overflow().horizontal(),
                    Axis::Vertical => node.layout.overflow().vertical(),
                };
                (policy == OverflowPolicy::Scroll).then(|| candidate.clone())
            })
        })
    }

    fn resolve_published_scroll_control_projection(
        &self,
        target: &MountedNodeId,
        binding: ScrollControlBinding,
        current_owner: Option<&MountedNodeId>,
    ) -> Result<AcceptedScrollControlProjection, ScrollControlResolutionFailure> {
        let axis = binding.axis();
        let (owner, published_binding, snapshot) = match self
            .surface_publication
            .current_scroll_control_projection(target)
        {
            ScrollControlProjectionLookup::Unavailable => {
                return Err(scroll_control_failure(
                    axis,
                    None,
                    TraceScrollControlBindingOutcome::MetricsUnavailable,
                ));
            }
            ScrollControlProjectionLookup::Unbound => {
                let outcome = if current_owner.is_some() {
                    TraceScrollControlBindingOutcome::MetricsUnavailable
                } else {
                    TraceScrollControlBindingOutcome::NonScrollable
                };
                return Err(scroll_control_failure(axis, None, outcome));
            }
            ScrollControlProjectionLookup::Bound {
                owner,
                binding,
                snapshot,
            } => (owner, binding, snapshot),
        };
        if published_binding != binding {
            return Err(scroll_control_failure(
                axis,
                Some(owner),
                TraceScrollControlBindingOutcome::MetricsUnavailable,
            ));
        }
        match current_owner {
            None => Err(scroll_control_failure(
                axis,
                Some(owner),
                TraceScrollControlBindingOutcome::NonScrollable,
            )),
            Some(current) if current != &owner => Err(scroll_control_failure(
                axis,
                Some(owner),
                TraceScrollControlBindingOutcome::Stale,
            )),
            Some(_) => Ok(AcceptedScrollControlProjection {
                owner,
                binding: published_binding,
                snapshot,
            }),
        }
    }

    fn resolve_scroll_control_snapshot(
        &self,
        owner: &MountedNodeId,
        binding: ScrollControlBinding,
    ) -> Result<ScrollControlSnapshot, ScrollControlResolutionFailure> {
        let axis = binding.axis();
        let owner_node = self.tree.node(owner).ok_or_else(|| {
            scroll_control_failure(
                axis,
                Some(owner.clone()),
                TraceScrollControlBindingOutcome::Stale,
            )
        })?;
        let owner_policy = match axis {
            Axis::Horizontal => owner_node.layout.overflow().horizontal(),
            Axis::Vertical => owner_node.layout.overflow().vertical(),
        };
        if owner_policy != OverflowPolicy::Scroll {
            return Err(scroll_control_failure(
                axis,
                Some(owner.clone()),
                TraceScrollControlBindingOutcome::NonScrollable,
            ));
        }
        let metrics = self
            .surface_publication
            .current_scroll_metrics(owner)
            .ok_or_else(|| {
                scroll_control_failure(
                    axis,
                    Some(owner.clone()),
                    TraceScrollControlBindingOutcome::MetricsUnavailable,
                )
            })?;
        let metrics_policy = match axis {
            Axis::Horizontal => metrics.overflow.horizontal(),
            Axis::Vertical => metrics.overflow.vertical(),
        };
        if metrics_policy != OverflowPolicy::Scroll {
            return Err(scroll_control_failure(
                axis,
                Some(owner.clone()),
                TraceScrollControlBindingOutcome::MetricsUnavailable,
            ));
        }
        let (offset, viewport, content) = match axis {
            Axis::Horizontal => (
                owner_node.interaction.scroll_offset.0,
                metrics.viewport.width(),
                metrics.content.width(),
            ),
            Axis::Vertical => (
                owner_node.interaction.scroll_offset.1,
                metrics.viewport.height(),
                metrics.content.height(),
            ),
        };
        ScrollControlSnapshot::__runtime_from_metrics(axis, offset, viewport, content).ok_or_else(
            || {
                scroll_control_failure(
                    axis,
                    Some(owner.clone()),
                    TraceScrollControlBindingOutcome::MetricsUnavailable,
                )
            },
        )
    }
}
