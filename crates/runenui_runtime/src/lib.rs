//! Headless runtime for `RunenUI`.
//!
//! This crate owns typed action delivery, update calls, root rebuilding, input
//! policy, trace recording, and renderer-facing surface-frame publication.
//! Accessibility extraction remains a future runtime slice.

#![forbid(unsafe_code)]

mod app;
mod constraints;
mod debug;
mod focus;
mod input;
mod measurement;
mod node;
mod policy;
pub mod prelude;
mod runtime;
mod style_debug;
mod surface;
mod trace;

pub use app::{ActivationResult, AppRuntime, UiApp};
pub use constraints::{AxisConstraints, AxisLimit, LayoutConstraints};
pub use debug::{DebugSurfaceRenderer, render_debug_surface_frame};
pub use focus::FocusState;
pub use input::{
    InputEvent, InputIntent, Key, KeyModifiers, KeyPhase, KeyboardEvent, LogicalPoint,
    PointerButton, PointerEvent, PointerPhase, resolve_pointer_event_target,
    resolve_pointer_input_event_target,
};
pub use measurement::{
    DeterministicMeasurementProvider, MeasurementProvider, TextMeasurement, TextMeasurementKind,
    TextMeasurementRequest,
};
pub use node::{RuntimeNodeId, RuntimeNodeRef, RuntimeTreeIndex};
pub use policy::{
    InputEventResult, KeyboardActivationResult, KeyboardFocusResult, PointerActivationResult,
    PointerFocusResult,
};
pub use runtime::Runtime;
pub use style_debug::{SurfaceStyleNode, SurfaceStyleReport, render_debug_surface_style_report};
pub use surface::{
    LogicalRect, LogicalSize, SurfaceBuildContext, SurfaceFrame, SurfaceNode, SurfaceNodeKind,
    SurfacePublication, publish_surface,
};
pub use trace::{RuntimeEvent, Trace, TraceRecord, TraceTarget};
