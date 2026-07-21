//! Owned semantic-command submission protocol.

use core::fmt;

use runenui_core::{CommandOrigin, MountedNodeId, SemanticCommand, WorkSequence};

use crate::RuntimeTerminalReason;

/// Accepted identity of one canonical semantic-command envelope.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CommandSubmission {
    sequence: WorkSequence,
}

impl CommandSubmission {
    pub(crate) const fn new(sequence: WorkSequence) -> Self {
        Self { sequence }
    }

    /// Returns the sequence assigned to the accepted command envelope.
    #[must_use]
    pub const fn sequence(self) -> WorkSequence {
        self.sequence
    }
}

/// Exact owned semantic command that did not enter the canonical FIFO.
#[must_use]
pub struct UnacceptedCommand {
    target: MountedNodeId,
    command: SemanticCommand,
    origin: CommandOrigin,
}

impl UnacceptedCommand {
    pub(crate) const fn new(
        target: MountedNodeId,
        command: SemanticCommand,
        origin: CommandOrigin,
    ) -> Self {
        Self {
            target,
            command,
            origin,
        }
    }

    /// Borrows the exact rejected mounted target.
    #[must_use]
    pub const fn target(&self) -> &MountedNodeId {
        &self.target
    }

    /// Returns the rejected semantic command.
    #[must_use]
    pub const fn command(&self) -> SemanticCommand {
        self.command
    }

    /// Returns the rejected normalized origin.
    #[must_use]
    pub const fn origin(&self) -> CommandOrigin {
        self.origin
    }

    /// Recovers every exact owned submission input.
    #[must_use]
    pub fn into_parts(self) -> (MountedNodeId, SemanticCommand, CommandOrigin) {
        (self.target, self.command, self.origin)
    }
}

impl fmt::Debug for UnacceptedCommand {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("UnacceptedCommand")
            .field("target", &self.target)
            .field("command", &self.command)
            .field("origin", &self.origin)
            .finish()
    }
}

/// Borrowed classification of one semantic-command submission rejection.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SubmitCommandErrorKind {
    Full,
    Closed,
    Terminal(RuntimeTerminalReason),
    ForeignTarget,
    StaleTarget,
    MissingTarget,
    WorkSequenceExhausted,
    TraceSequenceExhausted,
}

/// Submission rejection retaining the exact unaccepted command inputs.
#[must_use]
pub struct SubmitCommandError {
    kind: SubmitCommandErrorKind,
    unaccepted: UnacceptedCommand,
}

impl SubmitCommandError {
    pub(crate) const fn new(kind: SubmitCommandErrorKind, unaccepted: UnacceptedCommand) -> Self {
        Self { kind, unaccepted }
    }

    /// Returns the rejection classification.
    #[must_use]
    pub const fn kind(&self) -> SubmitCommandErrorKind {
        self.kind
    }

    /// Borrows the exact unaccepted command.
    pub const fn unaccepted(&self) -> &UnacceptedCommand {
        &self.unaccepted
    }

    /// Recovers the exact unaccepted command.
    pub fn into_unaccepted(self) -> UnacceptedCommand {
        self.unaccepted
    }
}

impl fmt::Debug for SubmitCommandError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SubmitCommandError")
            .field("kind", &self.kind)
            .field("unaccepted", &self.unaccepted)
            .finish()
    }
}

impl fmt::Display for SubmitCommandError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.kind {
            SubmitCommandErrorKind::Full => formatter.write_str("runtime work queue is full"),
            SubmitCommandErrorKind::Closed => formatter.write_str("runtime is closed"),
            SubmitCommandErrorKind::Terminal(reason) => {
                write!(formatter, "runtime is terminal: {reason}")
            }
            SubmitCommandErrorKind::ForeignTarget => {
                formatter.write_str("command target belongs to another runtime")
            }
            SubmitCommandErrorKind::StaleTarget => {
                formatter.write_str("command target lifetime is stale")
            }
            SubmitCommandErrorKind::MissingTarget => {
                formatter.write_str("command target has no mounted address")
            }
            SubmitCommandErrorKind::WorkSequenceExhausted => {
                formatter.write_str("runtime work sequence is exhausted")
            }
            SubmitCommandErrorKind::TraceSequenceExhausted => {
                formatter.write_str("enabled canonical trace sequence is exhausted")
            }
        }
    }
}

impl std::error::Error for SubmitCommandError {}
