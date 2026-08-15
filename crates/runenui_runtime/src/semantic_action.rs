//! Exact semantic-node action submission and rejection vocabulary.

use core::fmt;

use runenui_core::SemanticActionRequest;

use crate::RuntimeTerminalReason;

/// Borrowed classification of one semantic action submission rejection.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SubmitSemanticActionErrorKind {
    Full,
    Closed,
    Terminal(RuntimeTerminalReason),
    ForeignSurface,
    WrongSurface,
    ForeignTarget,
    StaleTarget,
    MissingTarget,
    TargetNotInSurface,
    UnsupportedAction,
    UnavailableAction,
    StaleAuthority,
    Integrity,
    WorkSequenceExhausted,
    TraceSequenceExhausted,
}

/// Submission rejection retaining the exact unaccepted semantic request.
#[must_use]
pub struct SubmitSemanticActionError {
    kind: SubmitSemanticActionErrorKind,
    request: SemanticActionRequest,
}

impl SubmitSemanticActionError {
    pub(crate) const fn new(
        kind: SubmitSemanticActionErrorKind,
        request: SemanticActionRequest,
    ) -> Self {
        Self { kind, request }
    }

    /// Returns the rejection classification.
    #[must_use]
    pub const fn kind(&self) -> SubmitSemanticActionErrorKind {
        self.kind
    }

    /// Borrows the exact semantic request that did not enter the canonical FIFO.
    #[must_use]
    pub const fn request(&self) -> &SemanticActionRequest {
        &self.request
    }

    /// Recovers the exact semantic request that did not enter the canonical FIFO.
    #[must_use]
    pub fn into_request(self) -> SemanticActionRequest {
        self.request
    }
}

impl fmt::Debug for SubmitSemanticActionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SubmitSemanticActionError")
            .field("kind", &self.kind)
            .field("request", &self.request)
            .finish()
    }
}

impl fmt::Display for SubmitSemanticActionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.kind {
            SubmitSemanticActionErrorKind::Full => {
                formatter.write_str("runtime work queue is full")
            }
            SubmitSemanticActionErrorKind::Closed => formatter.write_str("runtime is closed"),
            SubmitSemanticActionErrorKind::Terminal(reason) => {
                write!(formatter, "runtime is terminal: {reason}")
            }
            SubmitSemanticActionErrorKind::ForeignSurface => {
                formatter.write_str("semantic action surface belongs to another runtime")
            }
            SubmitSemanticActionErrorKind::WrongSurface => {
                formatter.write_str("semantic action targets a different logical surface")
            }
            SubmitSemanticActionErrorKind::ForeignTarget => {
                formatter.write_str("semantic target belongs to another runtime")
            }
            SubmitSemanticActionErrorKind::StaleTarget => {
                formatter.write_str("semantic target lifetime is stale")
            }
            SubmitSemanticActionErrorKind::MissingTarget => {
                formatter.write_str("semantic target has no semantic address")
            }
            SubmitSemanticActionErrorKind::TargetNotInSurface => {
                formatter.write_str("semantic target is not present in the current surface product")
            }
            SubmitSemanticActionErrorKind::UnsupportedAction => {
                formatter.write_str("semantic target does not support the requested action")
            }
            SubmitSemanticActionErrorKind::UnavailableAction => {
                formatter.write_str("semantic action is currently unavailable")
            }
            SubmitSemanticActionErrorKind::StaleAuthority => formatter
                .write_str("semantic action authority requires publication before admission"),
            SubmitSemanticActionErrorKind::Integrity => {
                formatter.write_str("semantic target or mounted-owner authority is inconsistent")
            }
            SubmitSemanticActionErrorKind::WorkSequenceExhausted => {
                formatter.write_str("runtime work sequence is exhausted")
            }
            SubmitSemanticActionErrorKind::TraceSequenceExhausted => {
                formatter.write_str("enabled canonical trace sequence is exhausted")
            }
        }
    }
}

impl std::error::Error for SubmitSemanticActionError {}
