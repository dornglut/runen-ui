use runenui_core::{
    PointerBoundaryEvent, PointerBoundaryKind, PointerCaptureEvent, PointerCaptureKind, PointerId,
    SurfaceInputContext,
};

use crate::{MountedNodeId, TraceDeliveryOutcome};

pub(super) struct PointerBoundaryNotification {
    pub(super) event: PointerBoundaryEvent,
    pub(super) delivery: TraceDeliveryOutcome,
}

pub(super) struct PointerBoundaryPlan {
    pub(super) previous_target: Option<MountedNodeId>,
    pub(super) current_target: Option<MountedNodeId>,
    pub(super) notifications: Vec<PointerBoundaryNotification>,
}

impl PointerBoundaryPlan {
    pub(super) fn unchanged(
        previous_target: Option<MountedNodeId>,
        current_target: Option<MountedNodeId>,
    ) -> Self {
        Self {
            previous_target,
            current_target,
            notifications: Vec::new(),
        }
    }

    pub(super) fn delivered_targets(&self) -> Vec<MountedNodeId> {
        self.notifications
            .iter()
            .filter(|notification| notification.delivery == TraceDeliveryOutcome::Delivered)
            .map(|notification| notification.event.target().clone())
            .collect()
    }
}

pub(super) fn plan_boundary_transition(
    pointer_id: PointerId,
    previous_path: &[MountedNodeId],
    physical_path: &[MountedNodeId],
    surface_context: &SurfaceInputContext,
    mut is_live: impl FnMut(&MountedNodeId) -> bool,
) -> PointerBoundaryPlan {
    let shared = previous_path
        .iter()
        .zip(physical_path)
        .take_while(|(previous, current)| previous == current)
        .count();
    let previous_target = previous_path.last().cloned();
    let current_target = physical_path.last().cloned();
    let capacity = previous_path
        .len()
        .saturating_sub(shared)
        .saturating_add(physical_path.len().saturating_sub(shared));
    let leaving = previous_path[shared..].iter().rev().cloned().map(|target| {
        PointerBoundaryEvent::__runtime_new(
            pointer_id,
            PointerBoundaryKind::Leave,
            target,
            current_target.clone(),
            surface_context.clone(),
        )
    });
    let entering = physical_path[shared..].iter().cloned().map(|target| {
        PointerBoundaryEvent::__runtime_new(
            pointer_id,
            PointerBoundaryKind::Enter,
            target,
            previous_target.clone(),
            surface_context.clone(),
        )
    });
    let notifications = leaving
        .chain(entering)
        .map(|event| PointerBoundaryNotification {
            delivery: if is_live(event.target()) {
                TraceDeliveryOutcome::Delivered
            } else {
                TraceDeliveryOutcome::Suppressed
            },
            event,
        })
        .collect::<Vec<_>>();
    debug_assert!(notifications.len() <= capacity);
    PointerBoundaryPlan {
        previous_target,
        current_target,
        notifications,
    }
}

pub(super) fn plan_capture_events(
    pointer_id: PointerId,
    previous_owner: Option<&MountedNodeId>,
    current_owner: Option<&MountedNodeId>,
    surface_context: &SurfaceInputContext,
) -> Vec<PointerCaptureEvent> {
    if previous_owner == current_owner {
        return Vec::new();
    }
    let mut events = Vec::with_capacity(2);
    if let Some(previous_owner) = previous_owner {
        events.push(PointerCaptureEvent::__runtime_new(
            pointer_id,
            PointerCaptureKind::Lost,
            previous_owner.clone(),
            current_owner.cloned(),
            surface_context.clone(),
        ));
    }
    if let Some(current_owner) = current_owner {
        events.push(PointerCaptureEvent::__runtime_new(
            pointer_id,
            PointerCaptureKind::Gained,
            current_owner.clone(),
            previous_owner.cloned(),
            surface_context.clone(),
        ));
    }
    events
}

#[cfg(test)]
mod tests {
    use runenui_core::{
        __runtime::RuntimeNamespace, PointerBoundaryKind, PointerCaptureKind, PointerId,
    };

    use super::{plan_boundary_transition, plan_capture_events};
    use crate::TraceDeliveryOutcome;

    #[test]
    fn path_diff_leaves_inner_to_outer_before_entering_outer_to_inner() {
        let namespace = RuntimeNamespace::__runtime_new();
        let root = namespace.__runtime_mounted_id(0, 1);
        let old_parent = namespace.__runtime_mounted_id(1, 1);
        let old_target = namespace.__runtime_mounted_id(2, 1);
        let new_parent = namespace.__runtime_mounted_id(3, 1);
        let new_target = namespace.__runtime_mounted_id(4, 1);
        let surface = namespace.__runtime_surface_id(0, 1);
        let context = namespace
            .__runtime_surface_context(surface, 1, 1)
            .unwrap_or_else(|| unreachable!("the surface belongs to the namespace"));
        let pointer_id = PointerId::new(7)
            .unwrap_or_else(|| unreachable!("the test pointer identity is non-zero"));

        let plan = plan_boundary_transition(
            pointer_id,
            &[root.clone(), old_parent.clone(), old_target.clone()],
            &[root, new_parent.clone(), new_target.clone()],
            &context,
            |_| true,
        );

        assert_eq!(plan.previous_target.as_ref(), Some(&old_target));
        assert_eq!(plan.current_target.as_ref(), Some(&new_target));
        assert_eq!(plan.notifications.len(), 4);
        assert_eq!(
            plan.notifications[0].event.kind(),
            PointerBoundaryKind::Leave
        );
        assert_eq!(plan.notifications[0].event.target(), &old_target);
        assert_eq!(
            plan.notifications[0].event.related_target(),
            Some(&new_target)
        );
        assert_eq!(
            plan.notifications[1].event.kind(),
            PointerBoundaryKind::Leave
        );
        assert_eq!(plan.notifications[1].event.target(), &old_parent);
        assert_eq!(
            plan.notifications[2].event.kind(),
            PointerBoundaryKind::Enter
        );
        assert_eq!(plan.notifications[2].event.target(), &new_parent);
        assert_eq!(
            plan.notifications[2].event.related_target(),
            Some(&old_target)
        );
        assert_eq!(
            plan.notifications[3].event.kind(),
            PointerBoundaryKind::Enter
        );
        assert_eq!(plan.notifications[3].event.target(), &new_target);
    }

    #[test]
    fn non_live_boundary_targets_are_retained_as_suppressed() {
        let namespace = RuntimeNamespace::__runtime_new();
        let stale = namespace.__runtime_mounted_id(1, 1);
        let live = namespace.__runtime_mounted_id(2, 1);
        let surface = namespace.__runtime_surface_id(0, 1);
        let context = namespace
            .__runtime_surface_context(surface, 1, 1)
            .unwrap_or_else(|| unreachable!("the surface belongs to the namespace"));
        let pointer_id = PointerId::new(8)
            .unwrap_or_else(|| unreachable!("the test pointer identity is non-zero"));

        let plan = plan_boundary_transition(
            pointer_id,
            core::slice::from_ref(&stale),
            core::slice::from_ref(&live),
            &context,
            |target| target == &live,
        );

        assert_eq!(plan.notifications.len(), 2);
        assert_eq!(
            plan.notifications[0].delivery,
            TraceDeliveryOutcome::Suppressed
        );
        assert_eq!(
            plan.notifications[1].delivery,
            TraceDeliveryOutcome::Delivered
        );
        assert_eq!(plan.delivered_targets(), [live]);
    }

    #[test]
    fn capture_transfer_loses_before_it_gains() {
        let namespace = RuntimeNamespace::__runtime_new();
        let previous = namespace.__runtime_mounted_id(1, 1);
        let current = namespace.__runtime_mounted_id(2, 1);
        let surface = namespace.__runtime_surface_id(0, 1);
        let context = namespace
            .__runtime_surface_context(surface, 1, 1)
            .unwrap_or_else(|| unreachable!("the surface belongs to the namespace"));
        let pointer_id = PointerId::new(9)
            .unwrap_or_else(|| unreachable!("the test pointer identity is non-zero"));

        let events = plan_capture_events(pointer_id, Some(&previous), Some(&current), &context);

        assert_eq!(events.len(), 2);
        assert_eq!(events[0].kind(), PointerCaptureKind::Lost);
        assert_eq!(events[0].target(), &previous);
        assert_eq!(events[0].related_owner(), Some(&current));
        assert_eq!(events[1].kind(), PointerCaptureKind::Gained);
        assert_eq!(events[1].target(), &current);
        assert_eq!(events[1].related_owner(), Some(&previous));
    }
}
