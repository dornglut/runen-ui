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
//!
//! Runtime sequences and records are runtime-issued and cannot be forged:
//!
//! ```compile_fail
//! use runenui_runtime::WorkSequence;
//! let _ = WorkSequence(1);
//! ```
//!
//! ```compile_fail
//! use runenui_runtime::TraceSequence;
//! let _ = TraceSequence(1);
//! ```
//!
//! ```compile_fail
//! use runenui_runtime::{TraceRecord, TraceRecordKind};
//! let _ = TraceRecord { kind: TraceRecordKind::RuntimeMounted };
//! ```
//!
//! Queue storage and envelopes are private runtime implementation details:
//!
//! ```compile_fail
//! use runenui_runtime::WorkEnvelope;
//! ```
//!
//! ```compile_fail
//! use runenui_runtime::queue::WorkQueue;
//! ```
//!
//! Work and trace sequences are separate, non-interchangeable types:
//!
//! ```compile_fail
//! use runenui_runtime::{TraceSequence, WorkSequence};
//! fn accept_work(_: WorkSequence) {}
//! let trace: Option<TraceSequence> = None;
//! if let Some(sequence) = trace { accept_work(sequence); }
//! ```
//!
//! Runtime configuration remains accessor-based and extensible:
//!
//! ```compile_fail
//! use runenui_runtime::{RuntimeConfig, TraceConfig};
//! let _ = RuntimeConfig { queue_capacity: 1, trace_config: TraceConfig::new(1) };
//! ```
//!
//! Direct dispatch was removed; actions enter through submission and pumping:
//!
//! ```compile_fail
//! # use runenui_core::{Element, View, text};
//! # use runenui_runtime::{AppRuntime, UiApp};
//! # struct App;
//! # impl UiApp for App {
//! #   type State = (); type Action = ();
//! #   fn root(_: &()) -> Element<()> { text("x").into_element() }
//! #   fn update(_: &mut (), _: ()) {}
//! # }
//! let mut runtime = AppRuntime::<App>::mount(());
//! runtime.dispatch(());
//! ```

#![forbid(unsafe_code)]

mod app;
mod config;
mod constraints;
mod debug;
mod focus;
mod input;
mod measurement;
mod mounted;
mod policy;
pub mod prelude;
mod pump;
mod queue;
mod runtime;
mod style_debug;
mod surface;
mod trace;

pub use app::{ActivationResult, AppRuntime, UiApp};
pub use config::RuntimeConfig;
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
pub use pump::{PumpBudget, PumpOutcome, PumpReport};
pub use queue::{SubmitActionError, SubmitActionErrorKind, SubmitActionResult, WorkSequence};
pub use runtime::{
    ReconciliationDiagnostic, ReconciliationGeneration, ReconciliationReport, RuntimeError,
    RuntimeStatus, RuntimeTerminalReason, ShutdownReport,
};
pub use style_debug::{SurfaceStyleNode, SurfaceStyleReport, render_debug_surface_style_report};
pub use surface::{
    LayoutOverflow, LogicalRect, LogicalSize, SurfaceBuildContext, SurfaceFrame, SurfaceLayoutNode,
    SurfaceLayoutReport, SurfaceNode, SurfacePhase, SurfacePhaseReport, SurfacePublication,
};
pub use trace::{Trace, TraceConfig, TraceRecord, TraceRecordKind, TraceSequence, TraceTarget};
