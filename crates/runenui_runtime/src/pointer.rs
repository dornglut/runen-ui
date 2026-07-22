//! Public pointer submission result and exact rejection ownership.

use core::fmt;

use runenui_core::{PointerEvent, WorkSequence};

use crate::RuntimeTerminalReason;

/// Borrowed classification of one pointer-submission failure.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SubmitPointerErrorKind {
    Full,
    Closed,
    Terminal(RuntimeTerminalReason),
    WorkSequenceExhausted,
    TraceSequenceExhausted,
}

/// Exact unaccepted pointer event and its rejection reason.
#[must_use]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SubmitPointerError {
    kind: SubmitPointerErrorKind,
    event: Box<PointerEvent>,
}

impl SubmitPointerError {
    pub(crate) fn new(kind: SubmitPointerErrorKind, event: PointerEvent) -> Self {
        Self {
            kind,
            event: Box::new(event),
        }
    }

    /// Returns the rejection classification.
    #[must_use]
    pub const fn kind(&self) -> SubmitPointerErrorKind {
        self.kind
    }

    /// Borrows the exact unaccepted event.
    #[must_use]
    pub fn event(&self) -> &PointerEvent {
        &self.event
    }

    /// Recovers the exact unaccepted event.
    #[must_use]
    pub fn into_event(self) -> PointerEvent {
        *self.event
    }
}

impl fmt::Display for SubmitPointerError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.kind {
            SubmitPointerErrorKind::Full => formatter.write_str("runtime work queue is full"),
            SubmitPointerErrorKind::Closed => formatter.write_str("runtime is closed"),
            SubmitPointerErrorKind::Terminal(reason) => {
                write!(formatter, "runtime is terminal: {reason}")
            }
            SubmitPointerErrorKind::WorkSequenceExhausted => {
                formatter.write_str("runtime work sequence is exhausted")
            }
            SubmitPointerErrorKind::TraceSequenceExhausted => {
                formatter.write_str("runtime trace sequence is exhausted")
            }
        }
    }
}

impl std::error::Error for SubmitPointerError {}

/// Receipt for one pointer event accepted into the canonical queue.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PointerSubmission {
    sequence: WorkSequence,
}

impl PointerSubmission {
    pub(crate) const fn new(sequence: WorkSequence) -> Self {
        Self { sequence }
    }

    /// Returns the canonical work sequence assigned at acceptance.
    #[must_use]
    pub const fn sequence(self) -> WorkSequence {
        self.sequence
    }
}
