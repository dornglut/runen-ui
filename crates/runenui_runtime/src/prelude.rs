//! Common imports for `RunenUI` runtime users.

pub use crate::{
    ActivationResult, AppRuntime, InputEvent, InputIntent, Key, KeyModifiers, KeyPhase,
    KeyboardEvent, LogicalPoint, PointerButton, PointerEvent, PointerPhase, Runtime, RuntimeEvent,
    RuntimeNodeId, RuntimeNodeRef, RuntimeTreeIndex, Trace, TraceRecord, TraceTarget, UiApp,
};

/// Handles resolved runtime input intents.
///
/// This trait intentionally consumes [`InputIntent`] values that have already
/// been resolved from raw input by hit testing, focus handling, or host logic.
/// It does not process raw [`InputEvent`] values.
pub trait InputIntentHandler {
    /// Handles one resolved input intent.
    fn handle_intent(&mut self, intent: InputIntent) -> ActivationResult;
}

impl<App> InputIntentHandler for AppRuntime<App>
where
    App: UiApp,
    App::Action: Clone,
{
    fn handle_intent(&mut self, intent: InputIntent) -> ActivationResult {
        match intent {
            InputIntent::ActivateNode(node_id) => self.activate_node(node_id),
        }
    }
}
