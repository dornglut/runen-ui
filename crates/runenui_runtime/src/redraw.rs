//! Revision-aware redraw take and acknowledgment tokens.

use std::sync::Arc;

use crate::TraceSequence;

/// Opaque redraw request for one exact dirty publication revision.
#[derive(Clone)]
pub struct RedrawRequest {
    pub(crate) namespace: Arc<()>,
    pub(crate) revision: u64,
    pub(crate) request_trace: Option<TraceSequence>,
    pub(crate) taken_trace: Option<TraceSequence>,
}

impl RedrawRequest {
    #[must_use]
    pub const fn revision(&self) -> u64 {
        self.revision
    }

    pub(crate) const fn publication_parent(&self) -> Option<TraceSequence> {
        match self.taken_trace {
            Some(taken) => Some(taken),
            None => self.request_trace,
        }
    }

    pub(crate) fn bind_taken_trace(&mut self, taken_trace: Option<TraceSequence>) {
        self.taken_trace = taken_trace;
    }
}

impl core::fmt::Debug for RedrawRequest {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_struct("RedrawRequest")
            .field("revision", &self.revision)
            .finish_non_exhaustive()
    }
}

#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RedrawAcknowledgeError {
    ForeignRuntime,
    FutureRevision,
}
