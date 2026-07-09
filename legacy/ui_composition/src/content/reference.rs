use serde::{Deserialize, Serialize};

use crate::{ContentInstanceRef, ContentOwnerId, ContentProfileId};

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct MountedContentRef {
    owner: ContentOwnerId,
    profile: ContentProfileId,
    instance: ContentInstanceRef,
}

impl MountedContentRef {
    pub fn new(
        owner: ContentOwnerId,
        profile: ContentProfileId,
        instance: ContentInstanceRef,
    ) -> Self {
        Self {
            owner,
            profile,
            instance,
        }
    }

    pub fn owner(&self) -> &ContentOwnerId {
        &self.owner
    }
    pub fn profile(&self) -> &ContentProfileId {
        &self.profile
    }
    pub fn instance(&self) -> &ContentInstanceRef {
        &self.instance
    }
}

#[derive(
    Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize,
)]
pub enum UnavailableContentPolicy {
    #[default]
    ShowFallback,
    AllowHide,
}

impl UnavailableContentPolicy {
    pub const fn permits_hide(self) -> bool {
        matches!(self, Self::AllowHide)
    }
}
