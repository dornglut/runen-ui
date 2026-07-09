//! File: domain/ui/ui_runtime/src/runtime/ui_runtime/helpers.rs
//! Purpose: Runtime input outcome helpers.

use ui_input::InputResponse;

use crate::{
    UiInputDispatchResult, UiInputOutcome, UiInteractionResults, UiInvalidation, WidgetId,
};

pub(super) fn outcome(
    target: Option<WidgetId>,
    response: InputResponse,
    interactions: UiInteractionResults,
) -> UiInputOutcome {
    UiInputOutcome {
        dispatch: UiInputDispatchResult { target, response },
        interactions,
        invalidation: UiInvalidation::from_response(response),
    }
}
