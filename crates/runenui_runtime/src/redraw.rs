//! Revision-aware redraw take and acknowledgment tokens.

use std::sync::Arc;

/// Opaque redraw request for one exact dirty publication revision.
#[derive(Clone)]
pub struct RedrawRequest {
    pub(crate) namespace: Arc<()>,
    pub(crate) revision: u64,
}

impl RedrawRequest {
    #[must_use]
    pub const fn revision(&self) -> u64 {
        self.revision
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
