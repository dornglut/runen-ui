//! Bounded canonical runtime trace records.

#![allow(clippy::redundant_pub_crate)]

mod admission;
mod context;
mod model;
mod store;

pub(crate) use admission::{MandatoryTracePlan, TraceReservation};
pub use context::{
    TraceActionCategory, TraceActionIdentity, TraceCompositionContext, TraceCompositionRange,
    TraceContext, TraceDeliveryOutcome, TraceEventContext, TraceEventFamily, TracePointerContext,
    TracePointerPath, TracePublicationContext, TraceRouteSnapshot, TraceSurfaceContext,
    TraceTargetTransition, TraceTextMetrics,
};
pub use model::{
    TraceConfig, TraceFocusBoundaryOutcome, TracePointerCaptureRequestRejection,
    TracePointerRejection, TraceRecord, TraceRecordKind, TraceRoutedAdmissionRejection,
    TraceRoutedIntegrityFailure, TraceSequence, TraceSpaceCleanupReason, TraceSurfaceIngressKind,
    TraceSurfaceRejection, TraceSurfaceSnapshotKind, TraceTarget, TraceTargetRejection,
    TraceTimerTerminalOutcome, TraceWorkFamily, TraceWorkIdentity, TraceWorkOwner,
    TraceWorkStartRefusal,
};
pub use store::Trace;
