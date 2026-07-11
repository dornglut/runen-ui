//! Common imports for `RunenUI` runtime users.

pub use crate::{
    ActivationResult, AppRuntime, DebugSurfaceRenderer, FocusState, InputEvent, InputEventResult,
    InputIntent, Key, KeyModifiers, KeyPhase, KeyboardActivationResult, KeyboardEvent,
    KeyboardFocusResult, LogicalPoint, LogicalRect, LogicalSize, PointerActivationResult,
    PointerButton, PointerEvent, PointerFocusResult, PointerPhase, Runtime, RuntimeEvent,
    RuntimeNodeId, RuntimeNodeRef, RuntimeTreeIndex, SurfaceFrame, SurfaceLayoutMetrics,
    SurfaceNode, SurfaceNodeKind, SurfaceStyleNode, SurfaceStyleReport, Trace, TraceRecord,
    TraceTarget, UiApp, layout_surface, layout_surface_with_metrics, render_debug_surface_frame,
    render_debug_surface_style_report, resolve_pointer_event_target,
    resolve_pointer_input_event_target, resolve_surface_style_report,
};

/// Resolves already-targeted raw input into runtime intents.
///
/// This trait intentionally handles only input events that already carry a
/// runtime node target. It does not perform hit testing, focus traversal, or
/// host input translation.
pub trait InputIntentResolver {
    /// Resolves this input event into a runtime intent, when it is actionable.
    fn resolve_intent(&self) -> Option<InputIntent>;
}

impl InputIntentResolver for InputEvent {
    fn resolve_intent(&self) -> Option<InputIntent> {
        match self {
            Self::Pointer(event) => resolve_pointer_intent(event),
            Self::Keyboard(event) => resolve_keyboard_intent(event),
        }
    }
}

fn resolve_pointer_intent(event: &PointerEvent) -> Option<InputIntent> {
    if event.phase() == PointerPhase::Pressed && event.button() == Some(PointerButton::Primary) {
        event.target().map(InputIntent::activate_node)
    } else {
        None
    }
}

fn resolve_keyboard_intent(event: &KeyboardEvent) -> Option<InputIntent> {
    if event.phase() == KeyPhase::Pressed && matches!(event.key(), Key::Enter | Key::Space) {
        event.target().map(InputIntent::activate_node)
    } else {
        None
    }
}

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
