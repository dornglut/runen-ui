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