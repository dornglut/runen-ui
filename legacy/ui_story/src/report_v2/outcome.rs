use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum UiStoryOutcomeV2 {
    Passed,
    ExpectedFailureMatched,
    Failed,
    Blocked,
    InvalidManifest,
    InvalidWorkflow,
}

impl UiStoryOutcomeV2 {
    pub const fn is_green(self) -> bool {
        matches!(self, Self::Passed | Self::ExpectedFailureMatched)
    }

    pub const fn is_mount_candidate(self) -> bool {
        matches!(self, Self::Passed)
    }

    pub const fn is_failure(self) -> bool {
        matches!(
            self,
            Self::Failed | Self::Blocked | Self::InvalidManifest | Self::InvalidWorkflow
        )
    }
}
