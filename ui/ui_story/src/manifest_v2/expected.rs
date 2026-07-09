use serde::{Deserialize, Serialize};

use crate::evidence::UiStoryDiagnosticExpectation;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum UiStoryExpectedOutcomeV2 {
    Pass,
    ExpectedFailure {
        expectation: UiStoryDiagnosticExpectation,
    },
}

impl UiStoryExpectedOutcomeV2 {
    pub const fn pass() -> Self {
        Self::Pass
    }

    pub fn expected_failure(expectation: UiStoryDiagnosticExpectation) -> Self {
        Self::ExpectedFailure { expectation }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum UiStoryMountPolicyV2 {
    GalleryOnly,
    EligibleWhenPassed,
    Never,
}
