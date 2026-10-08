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
    focus_visible: bool,
}

/// Runtime-owned mounted scroll offsets projected into one surface plan.
///
/// This is a cache-compatibility snapshot only. The mounted interaction slot is
/// the sole live authority; retaining this projection lets surface planning
/// detect offset changes without dirtying layout or recomputing text shaping.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct SurfaceScrollProjection {
    offsets: Vec<(MountedNodeId, (f32, f32))>,
}

impl SurfaceScrollProjection {
    pub(crate) const fn new(offsets: Vec<(MountedNodeId, (f32, f32))>) -> Self {
        Self { offsets }
    }

    pub(crate) fn offset(&self, id: &MountedNodeId) -> (f32, f32) {
        self.offsets
            .iter()
            .find_map(|(owner, offset)| (owner == id).then_some(*offset))
            .unwrap_or((0.0, 0.0))
    }

    pub(crate) fn offsets(&self) -> &[(MountedNodeId, (f32, f32))] {
        &self.offsets
    }

    pub(crate) fn content_differs(&self, other: &Self) -> bool {
        self.offsets != other.offsets
    }
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
            focus_visible: false,
        }
    }

    /// Derives a same-owner visibility fact for this ephemeral surface snapshot.
    #[must_use]
    pub(crate) const fn with_focus_visible(mut self, focus_visible: bool) -> Self {
        self.focus_visible = self.focused.is_some() && focus_visible;
        self
    }

    pub(crate) fn facts_for(&self, id: &MountedNodeId) -> StyleInteractionFacts {
        StyleInteractionFacts::NONE
            .with(StyleInteractionState::Hover, self.hovered.contains(id))
            .with(
                StyleInteractionState::Focus,
                self.focused.as_ref() == Some(id),
            )
            .with(
                StyleInteractionState::FocusVisible,
                self.focus_visible && self.focused.as_ref() == Some(id),
            )
            .with(StyleInteractionState::Active, self.active.contains(id))
    }

    pub(crate) fn content_differs(&self, other: &Self) -> bool {
        self.focused != other.focused
            || self.focus_visible != other.focus_visible
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
    #[test]
    fn focused_owner_visibility_projects_focus_and_focus_visible_without_new_identity() {
        let (focused, other) = ids();
        let projection =
            SurfaceInteractionProjection::new(vec![focused.clone()], Vec::new(), Some(focused.clone()))
                .with_focus_visible(true);
        let facts = projection.facts_for(&focused);
        assert!(facts.focused() && facts.focus_visible() && facts.hovered());
        assert_eq!(projection.facts_for(&other), StyleInteractionFacts::NONE);

        let hidden =
            SurfaceInteractionProjection::new(vec![focused.clone()], Vec::new(), Some(focused))
                .with_focus_visible(false);
        assert!(projection.content_differs(&hidden));
        assert!(!hidden.facts_for(&other).focus_visible());

        let unfocused =
            SurfaceInteractionProjection::new(Vec::new(), Vec::new(), None).with_focus_visible(true);
        assert!(!unfocused.facts_for(&other).focus_visible());
    }

}
