use runenui_core::{Focusability, MountedNodeId, SemanticKey, SemanticNodeId, WidgetActivation};

use super::{
    CachedCapability, CachedSemanticContribution, MountedTree, TargetStatus,
    node::state_is_corrupted,
    semantic::{SemanticTargetResolution, SemanticTargetStatus},
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SemanticActionAuthority {
    owner: MountedNodeId,
    key: SemanticKey,
    activation: WidgetActivation,
    focusability: Focusability,
}

impl SemanticActionAuthority {
    pub(crate) const fn owner(&self) -> &MountedNodeId {
        &self.owner
    }

    pub(crate) const fn key(&self) -> &SemanticKey {
        &self.key
    }

    pub(crate) const fn activation(&self) -> WidgetActivation {
        self.activation
    }

    pub(crate) const fn focusability(&self) -> Focusability {
        self.focusability
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SemanticActionAuthorityError {
    ForeignTarget,
    StaleTarget,
    MissingTarget,
    MissingOwner,
    StaleAuthority,
    Integrity,
}

impl<Action> MountedTree<Action> {
    /// Resolves one exact semantic lifetime to current private owner/key and
    /// already-cached action authority without invoking any widget capability.
    pub(crate) fn semantic_action_authority(
        &self,
        target: &SemanticNodeId,
    ) -> Result<SemanticActionAuthority, SemanticActionAuthorityError> {
        let resolution = self
            .semantic_store
            .resolve_target(&self.runtime, target)
            .map_err(map_target_status)?;
        self.semantic_action_authority_for_resolution(&resolution)
    }

    fn semantic_action_authority_for_resolution(
        &self,
        resolution: &SemanticTargetResolution,
    ) -> Result<SemanticActionAuthority, SemanticActionAuthorityError> {
        match self.target_status(resolution.owner()) {
            TargetStatus::Live => {}
            TargetStatus::Foreign | TargetStatus::Stale | TargetStatus::Missing => {
                return Err(SemanticActionAuthorityError::MissingOwner);
            }
        }
        let node = self
            .node(resolution.owner())
            .ok_or(SemanticActionAuthorityError::MissingOwner)?;
        if node.integrity_failed || state_is_corrupted(node) {
            return Err(SemanticActionAuthorityError::Integrity);
        }
        let activation = match node.caches.activation {
            CachedCapability::Ready(activation) => activation,
            CachedCapability::Unresolved => {
                return Err(SemanticActionAuthorityError::StaleAuthority);
            }
            CachedCapability::StatePayloadMismatch => {
                return Err(SemanticActionAuthorityError::Integrity);
            }
        };
        match &node.caches.semantics {
            CachedSemanticContribution::Ready(_) => {}
            CachedSemanticContribution::Unresolved
            | CachedSemanticContribution::Invalid(_)
            | CachedSemanticContribution::IdentityExhausted => {
                return Err(SemanticActionAuthorityError::StaleAuthority);
            }
            CachedSemanticContribution::IndexIntegrityFailure
            | CachedSemanticContribution::StatePayloadMismatch => {
                return Err(SemanticActionAuthorityError::Integrity);
            }
        }
        Ok(SemanticActionAuthority {
            owner: resolution.owner().clone(),
            key: resolution.key().clone(),
            activation,
            focusability: node.focusability,
        })
    }
}

fn map_target_status(status: SemanticTargetStatus) -> SemanticActionAuthorityError {
    match status {
        SemanticTargetStatus::Foreign => SemanticActionAuthorityError::ForeignTarget,
        SemanticTargetStatus::Stale => SemanticActionAuthorityError::StaleTarget,
        SemanticTargetStatus::Missing => SemanticActionAuthorityError::MissingTarget,
        #[cfg(test)]
        SemanticTargetStatus::Live => unreachable!("live semantic targets resolve to a record"),
    }
}
