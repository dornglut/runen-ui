//! Bounded canonical runtime trace records.

#![allow(clippy::redundant_pub_crate)]

mod admission;
mod construction;
mod context;
mod model;
mod store;

pub(crate) use admission::{MandatoryTracePlan, TraceReservation};
pub(crate) use construction::TraceRecordDraft;
// Public context vocabulary is added only when its producer family is complete.
pub use context::{
    TraceContext, TraceDeliveryOutcome, TraceEventContext, TraceEventFamily, TracePointerCleanup,
    TracePointerContext, TracePointerPath, TracePointerRecordRole, TracePublicationContext,
    TraceRouteSnapshot, TraceSurfaceContext, TraceTargetTransition,
};
pub use model::{
    TraceConfig, TraceFocusBoundaryOutcome, TracePointerCaptureRequestKind,
    TracePointerCaptureRequestRejection, TracePointerRejection, TraceRecord, TraceRecordKind,
    TraceRoutedAdmissionRejection, TraceRoutedIntegrityFailure, TraceSequence,
    TraceSpaceCleanupReason, TraceSurfaceIngressKind, TraceSurfaceRejection,
    TraceSurfaceSnapshotKind, TraceTarget, TraceTargetRejection, TraceTimerTerminalOutcome,
    TraceWorkFamily, TraceWorkIdentity, TraceWorkOwner, TraceWorkStartRefusal,
};
pub use store::Trace;
