//! Headless runtime for `RunenUI`.
//!
//! This crate owns typed action delivery, update calls, root rebuilding, and
//! trace recording. Input dispatch, accessibility extraction, and runtime
//! surface-frame publication remain future runtime slices.

#![forbid(unsafe_code)]

mod app;
mod focus;
mod input;
mod node;
pub mod prelude;
mod runtime;
mod surface;
mod trace;

pub use app::{
    ActivationResult, AppRuntime, InputEventResult, KeyboardActivationResult, KeyboardFocusResult,
    PointerActivationResult, PointerFocusResult, UiApp,
};
pub use focus::FocusState;
pub use input::{
    InputEvent, InputIntent, Key, KeyModifiers, KeyPhase, KeyboardEvent, LogicalPoint,
    PointerButton, PointerEvent, PointerPhase,
};
pub use node::{RuntimeNodeId, RuntimeNodeRef, RuntimeTreeIndex};
pub use runtime::Runtime;
pub use surface::{
    LogicalRect, LogicalSize, SurfaceFrame, SurfaceLayoutMetrics, SurfaceNode, SurfaceNodeKind,
    layout_surface, layout_surface_with_metrics,
};
pub use trace::{RuntimeEvent, Trace, TraceRecord, TraceTarget};
