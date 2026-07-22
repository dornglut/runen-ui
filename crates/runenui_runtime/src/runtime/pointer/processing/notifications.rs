use runenui_core::{
    PointerBoundaryEvent, PointerBoundaryKind, PointerCaptureEvent, PointerCaptureKind, PointerId,
    SurfaceInputContext,
};

use crate::MountedNodeId;

pub(super) fn plan_boundary_events(
    pointer_id: PointerId,
    previous_path: &[MountedNodeId],
    physical_path: &[MountedNodeId],
    surface_context: &SurfaceInputContext,
) -> Vec<PointerBoundaryEvent> {
    let shared = previous_path
        .iter()
        .zip(physical_path)
        .take_while(|(previous, current)| previous == current)
        .count();
    let previous_target = previous_path.last().cloned();
    let physical_target = physical_path.last().cloned();
    let mut events = Vec::with_capacity(
        previous_path
            .len()
            .saturating_sub(shared)
            .saturating_add(physical_path.len().saturating_sub(shared)),
    );
    events.extend(previous_path[shared..].iter().rev().cloned().map(|target| {
        PointerBoundaryEvent::__runtime_new(
            pointer_id,
            PointerBoundaryKind::Leave,
            target,
            physical_target.clone(),
            surface_context.clone(),
        )
    }));
    events.extend(physical_path[shared..].iter().cloned().map(|target| {
        PointerBoundaryEvent::__runtime_new(
            pointer_id,
            PointerBoundaryKind::Enter,
            target,
            previous_target.clone(),
            surface_context.clone(),
        )
    }));
    events
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

    use super::{plan_boundary_events, plan_capture_events};

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

        let events = plan_boundary_events(
            pointer_id,
            &[root.clone(), old_parent.clone(), old_target.clone()],
            &[root, new_parent.clone(), new_target.clone()],
            &context,
        );

        assert_eq!(events.len(), 4);
        assert_eq!(events[0].kind(), PointerBoundaryKind::Leave);
        assert_eq!(events[0].target(), &old_target);
        assert_eq!(events[0].related_target(), Some(&new_target));
        assert_eq!(events[1].kind(), PointerBoundaryKind::Leave);
        assert_eq!(events[1].target(), &old_parent);
        assert_eq!(events[2].kind(), PointerBoundaryKind::Enter);
        assert_eq!(events[2].target(), &new_parent);
        assert_eq!(events[2].related_target(), Some(&old_target));
        assert_eq!(events[3].kind(), PointerBoundaryKind::Enter);
        assert_eq!(events[3].target(), &new_target);
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
