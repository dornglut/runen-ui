use runenui_core::{
    PresentationOutsidePointerPolicy, SurfacePresentation,
};

use super::{HostProtocol, Runtime};
use crate::MountedNodeId;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(in crate::runtime) struct PresentationPointerBlock {
    pub(in crate::runtime) root: MountedNodeId,
    pub(in crate::runtime) dismiss: bool,
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
                return None;
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
                PresentationOutsidePointerPolicy::Ignore => {}
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

    pub(in crate::runtime) fn presentation_config(
        &self,
        root: &MountedNodeId,
    ) -> Option<&SurfacePresentation> {
        self.tree
            .node(root)
            .and_then(|node| node.surface_presentation.as_ref())
    }
}
