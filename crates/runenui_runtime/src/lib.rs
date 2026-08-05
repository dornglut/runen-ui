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
//! ```compile_fail
//! use runenui_runtime::SurfaceId;
//! let _ = SurfaceId { slot: 0, generation: 1 };
//! ```
//!
//! ```compile_fail
//! use runenui_runtime::SurfaceInputContext;
//! let _ = SurfaceInputContext {
//!     coordinate_revision: 1,
//!     hit_test_generation: 1,
//! };
//! ```
//!
//! Displayed-surface ingress remains logical and host-neutral:
//!
//! ```compile_fail
//! use runenui_runtime::{DpiScale, MonitorId, NativeWindowId, PhysicalPoint, PhysicalSize};
//! ```
//!
//! M4C2 owns one mounted root and one logical surface, not multi-window or
//! cross-surface focus lifecycle:
//!
//! ```compile_fail
//! use runenui_runtime::{CrossSurfaceFocus, SurfaceManager, WindowId};
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
//! Scheduler trace work identity is inspectable but cannot forge live work:
//!
//! ```compile_fail
//! use runenui_runtime::{TraceWorkFamily, TraceWorkIdentity, TraceWorkOwner};
//! let _ = TraceWorkIdentity::new(
//!     TraceWorkOwner::Application,
//!     TraceWorkFamily::LocalTask,
//!     1,
//!     None,
//! );
//! ```
//!
//! Host request tokens are opaque exact-generation capabilities:
//!
//! ```compile_fail
//! use runenui_runtime::HostRequestToken;
//! let _ = HostRequestToken {};
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
//! # use runenui_core::{IntoEffects, NoHostProtocol, UiApp, View, text};
//! # use runenui_runtime::AppRuntime;
//! # struct App;
//! # impl UiApp for App {
//! #   type State = (); type Action = (); type HostProtocol = NoHostProtocol;
//! #   fn root(_: &()) -> impl View<()> { text("x") }
//! #   fn update(_: &mut (), _: ()) -> impl IntoEffects<(), NoHostProtocol> {}
//! # }
//! let mut runtime = AppRuntime::<App>::mount(());
//! runtime.dispatch(());
//! ```
//!
//! M4C3 removes the public pointer proof helpers and unchecked target path:
//!
//! ```compile_fail
//! use runenui_runtime::{InputEvent, PointerFocusResult, resolve_pointer_event_target};
//! ```

#![forbid(unsafe_code)]

mod app;
mod clock;
mod command;
mod completion;
mod config;
mod constraints;
mod debug;
mod focus;
mod input;
mod measurement;
mod mounted;
mod pointer;
pub mod prelude;
mod pump;
mod queue;
mod redraw;
mod runtime;
mod style_debug;
mod surface;
mod surface_command;
mod surface_publication;
mod trace;
mod transaction;
mod wake;
mod work;

pub use app::AppRuntime;
pub use clock::{ManualClock, MonotonicClock, MonotonicInstant, MonotonicTimeError};
pub use command::{
    CommandSubmission, SubmitCommandError, SubmitCommandErrorKind, UnacceptedCommand,
};
pub use completion::{
    HostResponseCompletion, HostResponseCompletionError, SendTaskCompletion,
    SendTaskCompletionError, SendTaskExecutor, SendTaskJob, SendTaskStartError,
    SendTaskStartOutcome,
};
pub use config::{RuntimeConfig, RuntimeLimits};
pub use constraints::{AxisConstraints, AxisLimit, LayoutConstraints};
pub use debug::{DebugSurfaceRenderer, render_debug_surface_frame};
pub use focus::FocusState;
pub use input::{
    AutomationSubmission, CompositionStartRequest, CompositionStartSubmission,
    CompositionSubmission, KeyboardSubmission, SubmitAutomationError, SubmitAutomationErrorKind,
    SubmitCompositionError, SubmitCompositionErrorKind, SubmitCompositionStartError,
    SubmitKeyboardError, SubmitKeyboardErrorKind, SubmitTextError, SubmitTextErrorKind,
    TextSubmission,
};
pub use measurement::{
    BaselineError, DeterministicMeasurementProvider, MeasurementProvider, TextMeasurement,
    TextMeasurementKind, TextMeasurementRequest,
};
pub use mounted::{
    AutomationMatchDiagnostic, DuplicateIdentityKind, IdentityDiagnostic, InteractionStateRef,
    MountedNodeId, MountedNodeRef, MountedTreeIndex, SemanticNodeId,
};
pub use pointer::{PointerSubmission, SubmitPointerError, SubmitPointerErrorKind};
pub use pump::{PumpBudget, PumpBudgetExhaustion, PumpOutcome, PumpReport};
pub use queue::{SubmitActionError, SubmitActionErrorKind, SubmitActionResult, WorkSequence};
pub use redraw::{RedrawAcknowledgeError, RedrawRequest};
pub use runenui_core::{
    CommittedTextError, CommittedTextEvent, CompositionCancel, CompositionCancelReason,
    CompositionEnd, CompositionEvent, CompositionGeneration, CompositionRange,
    CompositionRangeError, CompositionStart, CompositionUpdate, FocusBoundaryPolicy,
    FocusDirection, FocusEvent, FocusEventKind, FocusReason, FocusScope, FocusScopePolicy,
    Focusability, InputDeviceId, InputModality, KeyLocation, KeyModifiers,
    KeyboardCompositionState, KeyboardEvent, KeyboardPhase, LogicalDelta, LogicalDeltaError,
    LogicalKey, LogicalPoint, LogicalPointError, LogicalScrollCommand, PhysicalKey,
    PointerBoundaryEvent, PointerBoundaryKind, PointerButton, PointerButtons, PointerCaptureEvent,
    PointerCaptureKind, PointerDeviceKind, PointerEvent, PointerId, PointerPhase, SurfaceId,
    SurfaceInputContext,
};
pub use runtime::{
    HostRequestCancelError, HostResponseError, ReconciliationDiagnostic, ReconciliationGeneration,
    ReconciliationReport, RuntimeError, RuntimeStatus, RuntimeTerminalReason, ShutdownReport,
    SubscriptionDiagnostic, SubscriptionOwnerKind, TimerFiringOutcome, TimerStartOutcome,
};
pub use style_debug::{SurfaceStyleNode, SurfaceStyleReport, render_debug_surface_style_report};
pub use surface::{
    LayoutOverflow, LogicalRect, LogicalSize, SurfaceBuildContext, SurfaceFrame, SurfaceLayoutNode,
    SurfaceLayoutReport, SurfaceNode, SurfacePhase, SurfacePhaseReport,
};
pub use surface_command::{
    SubmitSurfaceCommandError, SubmitSurfaceCommandErrorKind, UnacceptedSurfaceCommand,
};
pub use surface_publication::SurfacePublication;
pub use trace::{
    Trace, TraceActionCategory, TraceActionIdentity, TraceCompositionContext,
    TraceCompositionRange, TraceConfig, TraceContext, TraceDeliveryOutcome, TraceEventContext,
    TraceEventFamily, TraceFocusBoundaryOutcome, TraceModalityTransition,
    TracePointerCaptureRequestKind, TracePointerCaptureRequestRejection, TracePointerCleanup,
    TracePointerContext, TracePointerPath, TracePointerRejection, TracePublicationContext,
    TraceRecord, TraceRecordKind, TraceRouteSnapshot, TraceRoutedAdmissionRejection,
    TraceRoutedIntegrityFailure, TraceSequence, TraceSpaceCleanupReason, TraceSurfaceContext,
    TraceSurfaceIngressKind, TraceSurfaceRejection, TraceSurfaceSnapshotKind, TraceTarget,
    TraceTargetRejection, TraceTargetTransition, TraceTextMetrics, TraceTimerTerminalOutcome,
    TraceWorkFamily, TraceWorkIdentity, TraceWorkOwner, TraceWorkStartRefusal,
};
pub use wake::{WakeRequestOutcome, WakeTransport};
pub use work::host_request::{HostRequestRef, HostRequestToken};
