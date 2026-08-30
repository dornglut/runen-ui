use runenui_core::{StyleInteractionFacts, StyleInteractionState};

use crate::MountedNodeId;

/// Ephemeral runtime interaction facts for one surface publication attempt.
///
/// This is a derived value, never a second interaction authority. Runtime-owned
/// pointer and focus state project into it immediately before surface planning;
/// retained style facts keep only the previous value for cache compatibility.
#[derive(Clone, Debug, Default)]
pub(crate) struct SurfaceInteractionProjection {
    hovered: Vec<MountedNodeId>,
    active: Vec<MountedNodeId>,
    focused: Option<MountedNodeId>,
}

impl SurfaceInteractionProjection {
    pub(crate) const fn new(
        hovered: Vec<MountedNodeId>,
        active: Vec<MountedNodeId>,
        focused: Option<MountedNodeId>,
    ) -> Self {
        Self {
            hovered,
            active,
            focused,
        }
    }

    pub(crate) fn facts_for(&self, id: &MountedNodeId) -> StyleInteractionFacts {
        StyleInteractionFacts::NONE
            .with(StyleInteractionState::Hover, self.hovered.contains(id))
            .with(
                StyleInteractionState::Focus,
                self.focused.as_ref() == Some(id),
            )
            .with(StyleInteractionState::Active, self.active.contains(id))
    }

    pub(crate) fn content_differs(&self, other: &Self) -> bool {
        self.focused != other.focused
            || !same_membership(&self.hovered, &other.hovered)
            || !same_membership(&self.active, &other.active)
    }
}

fn same_membership(left: &[MountedNodeId], right: &[MountedNodeId]) -> bool {
    left.len() == right.len() && left.iter().all(|id| right.contains(id))
}

#[cfg(test)]
mod tests {
    use runenui_core::__runtime::RuntimeNamespace;

    use super::*;

    fn ids() -> (MountedNodeId, MountedNodeId) {
        let runtime = RuntimeNamespace::__runtime_new();
        (
            runtime.__runtime_mounted_id(0, 1),
            runtime.__runtime_mounted_id(1, 1),
        )
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
}
