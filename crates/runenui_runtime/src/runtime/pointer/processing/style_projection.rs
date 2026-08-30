use crate::{MountedNodeId, surface::SurfaceInteractionProjection};

use super::super::PointerRegistry;

impl PointerRegistry {
    /// Projects effective pointer interaction facts without retaining a second
    /// pointer state machine. Multiple active streams combine by boolean OR.
    pub(in crate::runtime) fn surface_interaction_projection(
        &self,
        focused: Option<&MountedNodeId>,
    ) -> SurfaceInteractionProjection {
        let mut hovered = Vec::new();
        let mut active = Vec::new();
        for stream in self.streams.values() {
            for id in stream.physical_path() {
                push_unique(&mut hovered, id);
            }
            if stream.pressed_inside
                && let Some(owner) = stream.pressed_owner()
            {
                push_unique(&mut active, owner);
            }
        }
        SurfaceInteractionProjection::new(hovered, active, focused.cloned())
    }
}

fn push_unique(ids: &mut Vec<MountedNodeId>, id: &MountedNodeId) {
    if !ids.contains(id) {
        ids.push(id.clone());
    }
}

#[cfg(test)]
mod tests {
    use runenui_core::{
        __runtime::RuntimeNamespace, LogicalPoint, PointerButtons, PointerDeviceKind, PointerId,
        StyleInteractionFacts, StyleInteractionState,
    };

    use super::*;

    fn ids() -> (MountedNodeId, MountedNodeId) {
        let runtime = RuntimeNamespace::__runtime_new();
        (
            runtime.__runtime_mounted_id(0, 1),
            runtime.__runtime_mounted_id(1, 1),
        )
    }

    fn pointer(value: u64) -> PointerId {
        PointerId::new(value)
            .unwrap_or_else(|| unreachable!("test pointer identities are non-zero"))
    }

    fn point(value: f32) -> LogicalPoint {
        LogicalPoint::new(value, value).unwrap_or_else(|_| unreachable!("test point is finite"))
    }

    #[test]
    fn membership_comparison_ignores_projection_order() {
        let (a, b) = ids();
        let left = SurfaceInteractionProjection::new(
            vec![a.clone(), b.clone()],
            vec![a.clone(), b.clone()],
            Some(a.clone()),
        );
        let right = SurfaceInteractionProjection::new(
            vec![b.clone(), a.clone()],
            vec![b, a.clone()],
            Some(a),
        );

        assert!(!left.content_differs(&right));
    }

    #[test]
    fn pointer_and_focus_authorities_project_exact_style_facts() {
        let namespace = RuntimeNamespace::__runtime_new();
        let surface = namespace.__runtime_surface_id(0, 1);
        let hovered_active = namespace.__runtime_mounted_id(0, 1);
        let hovered_focused = namespace.__runtime_mounted_id(1, 1);
        let untouched = namespace.__runtime_mounted_id(2, 1);
        let mut registry = PointerRegistry::new(2);

        let first = registry
            .register(
                pointer(1),
                surface.clone(),
                None,
                PointerDeviceKind::Mouse,
                point(1.0),
                PointerButtons::default(),
            )
            .unwrap_or_else(|_| unreachable!("first pointer stream fits"));
        first.update_observation(
            point(1.0),
            vec![hovered_active.clone()],
            PointerButtons::default(),
        );
        first.set_pressed_owner(Some(hovered_active.clone()));

        let second = registry
            .register(
                pointer(2),
                surface,
                None,
                PointerDeviceKind::Touch,
                point(2.0),
                PointerButtons::default(),
            )
            .unwrap_or_else(|_| unreachable!("second pointer stream fits"));
        second.update_observation(
            point(2.0),
            vec![hovered_focused.clone()],
            PointerButtons::default(),
        );

        let projection = registry.surface_interaction_projection(Some(&hovered_focused));
        assert_eq!(
            projection.facts_for(&hovered_active),
            StyleInteractionFacts::NONE
                .with(StyleInteractionState::Hover, true)
                .with(StyleInteractionState::Active, true)
        );
        assert_eq!(
            projection.facts_for(&hovered_focused),
            StyleInteractionFacts::NONE
                .with(StyleInteractionState::Hover, true)
                .with(StyleInteractionState::Focus, true)
        );
        assert_eq!(
            projection.facts_for(&untouched),
            StyleInteractionFacts::NONE
        );
    }

    #[test]
    fn active_projection_ors_streams_and_requires_pressed_inside() {
        let namespace = RuntimeNamespace::__runtime_new();
        let surface = namespace.__runtime_surface_id(0, 1);
        let owner = namespace.__runtime_mounted_id(0, 1);
        let mut registry = PointerRegistry::new(2);

        for (value, position) in [(1, 1.0), (2, 2.0)] {
            let stream = registry
                .register(
                    pointer(value),
                    surface.clone(),
                    None,
                    PointerDeviceKind::Touch,
                    point(position),
                    PointerButtons::default(),
                )
                .unwrap_or_else(|_| unreachable!("pointer stream fits"));
            stream.set_pressed_owner(Some(owner.clone()));
        }
        registry
            .stream_mut(pointer(1))
            .unwrap_or_else(|| unreachable!("first stream remains registered"))
            .set_pressed_inside(false);

        assert!(
            registry
                .surface_interaction_projection(None)
                .facts_for(&owner)
                .active(),
            "one pressed-inside stream keeps the shared owner active"
        );

        registry
            .stream_mut(pointer(2))
            .unwrap_or_else(|| unreachable!("second stream remains registered"))
            .set_pressed_inside(false);
        assert!(
            !registry
                .surface_interaction_projection(None)
                .facts_for(&owner)
                .active(),
            "pressed ownership without pressed-inside membership is not active"
        );
    }
}
