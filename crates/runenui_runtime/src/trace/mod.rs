//! Bounded canonical runtime trace records.

#![allow(clippy::redundant_pub_crate)]

mod admission;
mod model;
mod store;

pub(crate) use admission::{MandatoryTracePlan, TraceReservation};
pub use model::{
    TraceConfig, TraceRecord, TraceRecordKind, TraceRoutedAdmissionRejection,
    TraceRoutedIntegrityFailure, TraceSequence, TraceSurfaceIngressKind, TraceSurfaceRejection,
    TraceSurfaceSnapshotKind, TraceTarget, TraceTargetRejection, TraceTimerTerminalOutcome,
    TraceWorkFamily, TraceWorkIdentity, TraceWorkOwner, TraceWorkStartRefusal,
};
pub use store::Trace;
