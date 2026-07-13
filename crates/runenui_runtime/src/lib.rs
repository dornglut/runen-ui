//! Headless runtime for `RunenUI`.
//!
//! This crate owns typed action delivery, update calls, root rebuilding, input
//! policy, trace recording, and renderer-facing surface-frame publication.
//! Accessibility extraction remains a future runtime slice.
//!
//! Runtime-generated identities and products have no public forgery constructors:
//!
//! ```compile_fail
//! use runenui_runtime::MountedNodeId;
//! let _ = MountedNodeId { slot: 7, generation: 1 };
//! ```
//!
//! ```compile_fail
//! use runenui_runtime::SemanticNodeId;
//! let _ = SemanticNodeId { slot: 7, generation: 1 };
//! ```
//!
//! Mounted storage and transient publication are not public escape hatches:
//!
//! ```compile_fail
//! use runenui_runtime::mounted::MountedArena;
//! let _ = MountedArena::<()>::new();
//! ```
//!
//! ```compile_fail
//! use runenui_runtime::publish_surface;
//! ```
//!
//! ```compile_fail
//! use runenui_runtime::{LogicalSize, SurfaceFrame};
//! let _ = SurfaceFrame::new(LogicalSize::try_new(10.0, 10.0).unwrap(), Vec::new());
//! ```

#![forbid(unsafe_code)]

mod app;
mod constraints;
mod debug;
mod focus;
mod input;
mod measurement;
mod mounted;
mod policy;
pub mod prelude;
mod runtime;
mod style_debug;
mod surface;
mod trace;

pub use app::{ActivationResult, AppRuntime, UiApp};
pub use constraints::{AxisConstraints, AxisLimit, LayoutConstraints};
pub use debug::{DebugSurfaceRenderer, render_debug_surface_frame};
pub use focus::{FocusState, FocusTargetResult};
pub use input::{
    InputEvent, InputIntent, Key, KeyModifiers, KeyPhase, KeyboardEvent, LogicalPoint,
    LogicalPointError, PointerButton, PointerEvent, PointerPhase, resolve_pointer_event_target,
    resolve_pointer_input_event_target,
};
pub use measurement::{
    BaselineError, DeterministicMeasurementProvider, MeasurementProvider, TextMeasurement,
    TextMeasurementKind, TextMeasurementRequest,
};
pub use mounted::{
    DuplicateIdentityKind, IdentityDiagnostic, InteractionStateRef, MountedNodeId, MountedNodeRef,
    MountedTreeIndex, SemanticNodeId,
};
pub use policy::{
    InputEventResult, KeyboardActivationResult, KeyboardFocusResult, PointerActivationResult,
    PointerFocusResult,
};
pub use runtime::{
    ReconciliationDiagnostic, ReconciliationGeneration, ReconciliationReport, RuntimeError,
};
pub use style_debug::{SurfaceStyleNode, SurfaceStyleReport, render_debug_surface_style_report};
pub use surface::{
    LayoutOverflow, LogicalRect, LogicalSize, SurfaceBuildContext, SurfaceFrame, SurfaceLayoutNode,
    SurfaceLayoutReport, SurfaceNode, SurfacePhase, SurfacePhaseReport, SurfacePublication,
};
pub use trace::{RuntimeEvent, Trace, TraceRecord, TraceTarget};
