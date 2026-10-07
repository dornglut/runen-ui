use std::{
    collections::{HashMap, HashSet},
    mem,
};

use runenui_core::{
    FocusReason, PresentationDismissReason, PresentationFocusEntry, PresentationFocusPolicy,
    PresentationOutsidePointerPolicy, SurfacePresentation, SurfacePresentationAnchor,
};

use super::{HostProtocol, Runtime};
use crate::{
    MountedNodeId,
    focus::{FocusNavigation, is_focus_eligible, select_focus},
    mounted::TargetStatus,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::runtime) struct PresentationPointerBlock {
    pub(in crate::runtime) root: MountedNodeId,
    pub(in crate::runtime) dismiss: bool,
}

#[derive(Clone, Debug)]
struct PresentationLifetime {
    owner: Option<MountedNodeId>,
    focus_policy: PresentationFocusPolicy,
    restoration_target: Option<MountedNodeId>,
    restoration_scopes: Vec<MountedNodeId>,
    published: bool,
    anchor_unavailable_requested: bool,
    ambiguous_preferred_focus: bool,
}

#[derive(Default)]
pub(crate) struct PresentationLifecycleState {
    lifetimes: HashMap<MountedNodeId, PresentationLifetime>,
}

impl PresentationLifecycleState {
    pub(crate) fn new() -> Self {
        Self::default()
    }
}

#[derive(Clone, Debug)]
pub(in crate::runtime) struct PresentationRestorationPlan {
    pub(in crate::runtime) routing_target: MountedNodeId,
    pub(in crate::runtime) focus_target: Option<MountedNodeId>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum InitialFocusSelection {
    None,
    Target(MountedNodeId),
    AmbiguousPreferred,
}

impl<State, Action, Protocol: HostProtocol> Runtime<State, Action, Protocol> {
    pub(in crate::runtime) fn presentation_pointer_block(
        &self,
        physical_target: Option<&MountedNodeId>,
        include_nonmodal_policy: bool,
    ) -> Option<PresentationPointerBlock> {
        for candidate in self
            .surface_publication
            .current_presentation_interaction_roots()
        {
            if physical_target.is_some_and(|target| {
                self.tree
                    .presentation_family_contains(&candidate.root, target)
            }) {
                continue;
            }

            let policy = candidate.presentation.outside_pointer();
            if candidate.presentation.is_modal() {
                return Some(PresentationPointerBlock {
                    root: candidate.root,
                    dismiss: include_nonmodal_policy
                        && policy == PresentationOutsidePointerPolicy::DismissAndBlock,
                });
            }
            if !include_nonmodal_policy {
                continue;
            }
            match policy {
                PresentationOutsidePointerPolicy::Block => {
                    return Some(PresentationPointerBlock {
                        root: candidate.root,
                        dismiss: false,
                    });
                }
                PresentationOutsidePointerPolicy::DismissAndBlock => {
                    return Some(PresentationPointerBlock {
                        root: candidate.root,
                        dismiss: true,
                    });
                }
                _ => {}
            }
        }
        None
    }

    pub(in crate::runtime) fn topmost_cancel_presentation(&self) -> Option<MountedNodeId> {
        self.surface_publication
            .current_presentation_interaction_roots()
            .into_iter()
            .find_map(|candidate| {
                candidate
                    .presentation
                    .dismisses_on_cancel_or_back()
                    .then_some(candidate.root)
            })
    }

    pub(in crate::runtime) fn presentation_focus_membership(
        &self,
        focused: Option<&MountedNodeId>,
    ) -> Vec<MountedNodeId> {
        let Some(focused) = focused else {
            return Vec::new();
        };
        self.surface_publication
            .current_presentation_interaction_roots()
            .into_iter()
            .filter(|candidate| {
                self.presentation_lifecycle
                    .lifetimes
                    .get(&candidate.root)
                    .is_some_and(|lifetime| {
                        lifetime.published
                            && lifetime.focus_policy == PresentationFocusPolicy::EnterAndRestore
                    })
                    && self
                        .tree
                        .presentation_family_contains(&candidate.root, focused)
            })
            .map(|candidate| candidate.root)
            .collect()
    }

    pub(in crate::runtime) fn retire_presentation_lifetimes_after_reconciliation(
        &mut self,
        focused_membership: &[MountedNodeId],
    ) -> Vec<PresentationRestorationPlan> {
        let mut lifecycle = mem::take(&mut self.presentation_lifecycle);
        let removed = focused_membership
            .iter()
            .filter_map(|root| {
                let still_authored = self.tree.target_status(root) == TargetStatus::Live
                    && self
                        .tree
                        .node(root)
                        .is_some_and(|node| node.surface_presentation.is_some());
                (!still_authored)
                    .then(|| lifecycle.lifetimes.get(root).cloned())
                    .flatten()
            })
            .collect::<Vec<_>>();
        lifecycle.lifetimes.retain(|root, _| {
            self.tree.target_status(root) == TargetStatus::Live
                && self
                    .tree
                    .node(root)
                    .is_some_and(|node| node.surface_presentation.is_some())
        });

        let mut restorations = Vec::new();
        for lifetime in removed {
            if lifetime.published
                && lifetime.focus_policy == PresentationFocusPolicy::EnterAndRestore
                && let Some(plan) = self.presentation_restoration_plan(&lifetime)
            {
                restorations.push(plan);
            }
        }

        self.presentation_lifecycle = lifecycle;
        restorations
    }

    pub(in crate::runtime) fn synchronize_presentation_lifecycle_after_publication(
        &mut self,
        causal_parent: Option<crate::TraceSequence>,
    ) {
        let mut lifecycle = mem::take(&mut self.presentation_lifecycle);
        let published = self
            .surface_publication
            .current_presentation_interaction_roots();
        let published_roots = published
            .iter()
            .map(|candidate| candidate.root.clone())
            .collect::<HashSet<_>>();
        let authored = self.authored_presentations();
        let authored_roots = authored
            .iter()
            .map(|(root, _, _)| root.clone())
            .collect::<HashSet<_>>();

        lifecycle
            .lifetimes
            .retain(|root, _| authored_roots.contains(root));
        if !self.synchronize_published_presentation_lifetimes(
            &mut lifecycle,
            published,
            causal_parent,
        ) {
            self.presentation_lifecycle = lifecycle;
            return;
        }
        let _ = self.synchronize_unpublished_presentation_lifetimes(
            &mut lifecycle,
            authored,
            &published_roots,
            causal_parent,
        );
        self.presentation_lifecycle = lifecycle;
    }

    fn authored_presentations(
        &self,
    ) -> Vec<(MountedNodeId, Option<MountedNodeId>, SurfacePresentation)> {
        self.tree
            .publication_preorder_ids()
            .into_iter()
            .filter_map(|root| {
                let node = self.tree.node(&root)?;
                let presentation = node.surface_presentation.clone()?;
                Some((root, node.parent.clone(), presentation))
            })
            .collect()
    }

    fn synchronize_published_presentation_lifetimes(
        &mut self,
        lifecycle: &mut PresentationLifecycleState,
        published: Vec<crate::surface::PresentationInteractionRoot>,
        causal_parent: Option<crate::TraceSequence>,
    ) -> bool {
        let mut prospective_focus = self.focus.focused_node().cloned();
        for candidate in published.into_iter().rev() {
            let previous_policy = lifecycle
                .lifetimes
                .get(&candidate.root)
                .map(|lifetime| lifetime.focus_policy);
            let newly_active = lifecycle
                .lifetimes
                .get(&candidate.root)
                .is_none_or(|lifetime| !lifetime.published);
            let enter_policy_changed = previous_policy == Some(PresentationFocusPolicy::Preserve)
                && candidate.presentation.focus_policy()
                    == PresentationFocusPolicy::EnterAndRestore;
            let should_enter = newly_active || enter_policy_changed;

            let mut restoration_target = None;
            let mut restoration_scopes = Vec::new();
            let mut ambiguous_preferred_focus = false;
            if should_enter
                && candidate.presentation.focus_policy() == PresentationFocusPolicy::EnterAndRestore
            {
                restoration_target.clone_from(&prospective_focus);
                restoration_scopes =
                    self.presentation_scope_chain(candidate.owner.as_ref(), &candidate.root);
                match self.presentation_initial_focus_target(&candidate.root) {
                    InitialFocusSelection::Target(target) => {
                        prospective_focus = Some(target.clone());
                        if self
                            .submit_presentation_focus_request(
                                &target,
                                Some(target),
                                FocusReason::ProgrammaticRequest,
                                causal_parent,
                            )
                            .is_err()
                        {
                            return false;
                        }
                    }
                    InitialFocusSelection::AmbiguousPreferred => {
                        ambiguous_preferred_focus = true;
                    }
                    InitialFocusSelection::None => {}
                }
            }

            let lifetime = lifecycle
                .lifetimes
                .entry(candidate.root.clone())
                .or_insert_with(|| PresentationLifetime {
                    owner: candidate.owner.clone(),
                    focus_policy: candidate.presentation.focus_policy(),
                    restoration_target: None,
                    restoration_scopes: Vec::new(),
                    published: false,
                    anchor_unavailable_requested: false,
                    ambiguous_preferred_focus: false,
                });
            lifetime.owner = candidate.owner;
            if should_enter {
                lifetime.restoration_target = restoration_target;
                lifetime.restoration_scopes = restoration_scopes;
                lifetime.ambiguous_preferred_focus = ambiguous_preferred_focus;
            }
            if candidate.presentation.focus_policy() == PresentationFocusPolicy::Preserve {
                lifetime.restoration_target = None;
                lifetime.restoration_scopes.clear();
                lifetime.ambiguous_preferred_focus = false;
            }
            lifetime.focus_policy = candidate.presentation.focus_policy();
            lifetime.published = true;
        }
        true
    }

    fn synchronize_unpublished_presentation_lifetimes(
        &mut self,
        lifecycle: &mut PresentationLifecycleState,
        authored: Vec<(MountedNodeId, Option<MountedNodeId>, SurfacePresentation)>,
        published_roots: &HashSet<MountedNodeId>,
        causal_parent: Option<crate::TraceSequence>,
    ) -> bool {
        for (root, owner, presentation) in authored {
            if published_roots.contains(&root) {
                continue;
            }
            let lifetime =
                lifecycle
                    .lifetimes
                    .entry(root.clone())
                    .or_insert_with(|| PresentationLifetime {
                        owner: owner.clone(),
                        focus_policy: presentation.focus_policy(),
                        restoration_target: None,
                        restoration_scopes: Vec::new(),
                        published: false,
                        anchor_unavailable_requested: false,
                        ambiguous_preferred_focus: false,
                    });

            let was_published = lifetime.published;
            let should_restore = was_published
                && lifetime.focus_policy == PresentationFocusPolicy::EnterAndRestore
                && self
                    .focus
                    .focused_node()
                    .is_some_and(|focused| self.tree.presentation_family_contains(&root, focused));
            let restore_snapshot = should_restore.then(|| lifetime.clone());
            lifetime.owner = owner;
            lifetime.focus_policy = presentation.focus_policy();
            lifetime.published = false;
            lifetime.restoration_target = None;
            lifetime.restoration_scopes.clear();

            if let Some(snapshot) = restore_snapshot
                && let Some(plan) = self.presentation_restoration_plan(&snapshot)
                && self
                    .submit_presentation_focus_request(
                        &plan.routing_target,
                        plan.focus_target,
                        FocusReason::PresentationRestoration,
                        causal_parent,
                    )
                    .is_err()
            {
                return false;
            }

            let owner_relative = matches!(
                presentation.anchor(),
                SurfacePresentationAnchor::OwnerBounds | SurfacePresentationAnchor::OwnerRect(_)
            );
            if owner_relative && !lifetime.anchor_unavailable_requested {
                lifetime.anchor_unavailable_requested = true;
                if self
                    .submit_presentation_dismiss_request(
                        &root,
                        PresentationDismissReason::AnchorUnavailable,
                        causal_parent,
                    )
                    .is_err()
                {
                    return false;
                }
            }
        }
        true
    }

    fn presentation_initial_focus_target(&mut self, root: &MountedNodeId) -> InitialFocusSelection {
        let descendants = self
            .tree
            .publication_preorder_ids()
            .into_iter()
            .filter(|candidate| self.exact_presentation_descendant(root, candidate))
            .collect::<Vec<_>>();
        let preferred = descendants
            .iter()
            .filter(|candidate| {
                self.tree.node(candidate).is_some_and(|node| {
                    node.presentation_focus_entry == PresentationFocusEntry::Preferred
                })
            })
            .cloned()
            .collect::<Vec<_>>();
        if preferred.len() > 1 {
            return InitialFocusSelection::AmbiguousPreferred;
        }

        let eligibility = self.focus_eligibility_projection();
        if let Some(preferred) = preferred.first()
            && is_focus_eligible(&mut self.tree, preferred, &eligibility)
        {
            return InitialFocusSelection::Target(preferred.clone());
        }
        descendants
            .into_iter()
            .find(|candidate| is_focus_eligible(&mut self.tree, candidate, &eligibility))
            .map_or(InitialFocusSelection::None, InitialFocusSelection::Target)
    }

    fn exact_presentation_descendant(
        &self,
        root: &MountedNodeId,
        candidate: &MountedNodeId,
    ) -> bool {
        if candidate == root
            || self
                .tree
                .node(candidate)
                .is_some_and(|node| node.surface_presentation.is_some())
        {
            return false;
        }
        let mut current = candidate.clone();
        let mut remaining = self.tree.live_count().saturating_add(1);
        while remaining != 0 {
            remaining -= 1;
            let Some(parent) = self
                .tree
                .node(&current)
                .and_then(|node| node.parent.clone())
            else {
                return false;
            };
            if &parent == root {
                return true;
            }
            if self
                .tree
                .node(&parent)
                .is_some_and(|node| node.surface_presentation.is_some())
            {
                return false;
            }
            current = parent;
        }
        false
    }

    fn presentation_scope_chain(
        &self,
        owner: Option<&MountedNodeId>,
        root: &MountedNodeId,
    ) -> Vec<MountedNodeId> {
        let mut current = owner.cloned().unwrap_or_else(|| root.clone());
        let mut scopes = Vec::new();
        let mut remaining = self.tree.live_count().saturating_add(1);
        while remaining != 0 {
            remaining -= 1;
            let Some(node) = self.tree.node(&current) else {
                break;
            };
            if node.focus_scope.is_some() || node.parent.is_none() {
                scopes.push(current.clone());
            }
            let Some(parent) = node.parent.clone() else {
                break;
            };
            current = parent;
        }
        scopes
    }

    fn presentation_restoration_plan(
        &mut self,
        lifetime: &PresentationLifetime,
    ) -> Option<PresentationRestorationPlan> {
        let eligibility = self.focus_eligibility_projection();
        let exact = [
            lifetime.restoration_target.as_ref(),
            lifetime.owner.as_ref(),
        ]
        .into_iter()
        .flatten()
        .find(|candidate| {
            self.tree.target_status(candidate) == TargetStatus::Live
                && is_focus_eligible(&mut self.tree, candidate, &eligibility)
        })
        .cloned();

        let focus_target = exact.or_else(|| {
            let geometry = self.surface_publication.current_focus_geometry();
            for scope in &lifetime.restoration_scopes {
                if self.tree.target_status(scope) != TargetStatus::Live {
                    continue;
                }
                let selection = select_focus(
                    &mut self.tree,
                    &self.focus,
                    scope,
                    FocusNavigation::Restore,
                    &geometry,
                    &eligibility,
                )?;
                if let Some(target) = selection.target {
                    return Some(target);
                }
            }
            None
        });

        let routing_target = focus_target
            .clone()
            .or_else(|| {
                lifetime
                    .owner
                    .as_ref()
                    .filter(|owner| self.tree.target_status(owner) == TargetStatus::Live)
                    .cloned()
            })
            .or_else(|| {
                lifetime
                    .restoration_scopes
                    .iter()
                    .find(|scope| self.tree.target_status(scope) == TargetStatus::Live)
                    .cloned()
            })
            .or_else(|| self.tree.root_id().cloned())?;

        Some(PresentationRestorationPlan {
            routing_target,
            focus_target,
        })
    }
}
