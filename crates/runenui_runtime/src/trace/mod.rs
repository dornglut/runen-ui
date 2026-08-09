//! Bounded canonical runtime trace records.

#![allow(clippy::redundant_pub_crate)]

mod action_context;
mod admission;
mod automation_context;
mod construction;
mod context;
mod export;
mod input_context;
mod model;
mod sink;
mod store;

pub use action_context::{TraceActionCategory, TraceActionIdentity};
pub(crate) use admission::{MandatoryTracePlan, TraceReservation};
pub use automation_context::{TraceAutomationContext, TraceAutomationRecordRole};
pub(crate) use construction::TraceRecordDraft;
pub use context::{
    TraceContext, TraceDeliveryOutcome, TraceEventContext, TraceEventFamily, TraceFocusRecordRole,
    TraceModalityTransition, TracePointerCleanup, TracePointerContext, TracePointerPath,
    TracePointerRecordRole, TracePublicationContext, TraceRouteSnapshot, TraceSurfaceContext,
    TraceTargetTransition,
};
pub(crate) use export::encode_record_json;
pub use input_context::{
    TraceCompositionContext, TraceCompositionRange, TraceInputContext, TraceInputRecordRole,
    TraceTextMetrics,
};
pub use model::{
    TraceConfig, TraceFocusBoundaryOutcome, TracePayloadCapture, TracePointerCaptureRequestKind,
    TracePointerCaptureRequestRejection, TracePointerRejection, TraceRecord, TraceRecordKind,
    TraceRoutedAdmissionRejection, TraceRoutedIntegrityFailure, TraceSequence,
    TraceSinkDeliveryOutcome, TraceSpaceCleanupReason, TraceSurfaceIngressKind,
    TraceSurfaceRejection, TraceSurfaceSnapshotKind, TraceTarget, TraceTargetRejection,
    TraceTimerTerminalOutcome, TraceWorkFamily, TraceWorkIdentity, TraceWorkOwner,
    TraceWorkStartRefusal,
};
use sink::TraceSink;
pub use sink::{TraceJsonlLine, TraceSinkReceiveError, TraceSinkReceiver};
pub use store::Trace;
