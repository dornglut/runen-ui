//! File: domain/ui/ui_runtime/src/input/dispatch.rs
//! Purpose: Result of dispatching one UI input event.

use crate::WidgetId;
use ui_input::InputResponse;

#[derive(Debug, Clone, PartialEq)]
pub struct UiInputDispatchResult {
    pub target: Option<WidgetId>,
    pub response: InputResponse,
}

impl UiInputDispatchResult {
    pub fn ignored() -> Self {
        Self {
            target: None,
            response: InputResponse::ignored(),
        }
    }
}
