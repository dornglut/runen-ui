//! File: domain/ui/ui_runtime/src/input/outcome.rs
//! Purpose: Combined low-level dispatch response and semantic interaction results.

use crate::{UiInputDispatchResult, UiInteractionResults};
use ui_input::InputResponse;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct UiInvalidation {
    pub repaint: bool,
    pub relayout: bool,
}

impl UiInvalidation {
    pub fn from_response(response: InputResponse) -> Self {
        Self {
            repaint: response.repaint,
            relayout: response.relayout,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct UiInputOutcome {
    pub dispatch: UiInputDispatchResult,
    pub interactions: UiInteractionResults,
    pub invalidation: UiInvalidation,
}

impl UiInputOutcome {
    pub fn ignored() -> Self {
        Self {
            dispatch: UiInputDispatchResult::ignored(),
            interactions: UiInteractionResults::new(),
            invalidation: UiInvalidation::default(),
        }
    }
}
