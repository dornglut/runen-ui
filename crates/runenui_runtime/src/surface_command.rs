//! Checked displayed-surface semantic-command ingress vocabulary.

use core::fmt;

use runenui_core::{CommandOrigin, SemanticCommand, SurfaceInputContext};

use crate::{LogicalPoint, MountedNodeId, RuntimeTerminalReason};

/// Owned displayed-surface command request that did not enter the canonical FIFO.
#[must_use]
pub enum UnacceptedSurfaceCommand {
    /// Logical-coordinate request against one exact displayed snapshot.
    Logical {
        context: SurfaceInputContext,
        point: LogicalPoint,
        command: SemanticCommand,
        origin: CommandOrigin,
    },
    /// Checked low-level request for one exact target in one displayed snapshot.
    Resolved {
        context: SurfaceInputContext,
        target: MountedNodeId,
        command: SemanticCommand,
        origin: CommandOrigin,
    },
}

impl UnacceptedSurfaceCommand {
    /// Borrows the exact rejected surface context.
    #[must_use]
    pub const fn context(&self) -> &SurfaceInputContext {
        match self {
            Self::Logical { context, .. } | Self::Resolved { context, .. } => context,
        }
    }

    /// Returns the rejected semantic command.
    #[must_use]
    pub const fn command(&self) -> SemanticCommand {
        match self {
            Self::Logical { command, .. } | Self::Resolved { command, .. } => *command,
        }
    }

    /// Returns the rejected normalized command origin.
    #[must_use]
    pub const fn origin(&self) -> CommandOrigin {
        match self {
            Self::Logical { origin, .. } | Self::Resolved { origin, .. } => *origin,
        }
    }

    /// Returns the logical position for a coordinate request.
    #[must_use]
    pub const fn point(&self) -> Option<LogicalPoint> {
        match self {
            Self::Logical { point, .. } => Some(*point),
            Self::Resolved { .. } => None,
        }
    }

    /// Borrows the checked target for a resolved-target request.
    #[must_use]
    pub const fn target(&self) -> Option<&MountedNodeId> {
        match self {
            Self::Logical { .. } => None,
            Self::Resolved { target, .. } => Some(target),
        }
    }
}

impl fmt::Debug for UnacceptedSurfaceCommand {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Logical {
                context,
                point,
                command,
                origin,
            } => formatter
                .debug_struct("UnacceptedSurfaceCommand::Logical")
                .field("context", context)
                .field("point", point)
                .field("command", command)
                .field("origin", origin)
                .finish(),
            Self::Resolved {
                context,
                target,
                command,
                origin,
            } => formatter
                .debug_struct("UnacceptedSurfaceCommand::Resolved")
                .field("context", context)
                .field("target", target)
                .field("command", command)
                .field("origin", origin)
                .finish(),
        }
    }
}

/// Classification of one checked displayed-surface command rejection.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SubmitSurfaceCommandErrorKind {
    Full,
    Closed,
    Terminal(RuntimeTerminalReason),
    ForeignSurfaceContext,
    ForeignSurface,
    RetiredSurfaceContext,
    MissingSurfaceGeneration,
    CoordinateRevisionMismatch,
    NoTarget,
    TargetNotInSnapshot,
    ForeignTarget,
    StaleTarget,
    MissingTarget,
    WorkSequenceExhausted,
    TraceSequenceExhausted,
}

/// Checked displayed-surface command rejection retaining every owned input.
#[must_use]
pub struct SubmitSurfaceCommandError {
    kind: SubmitSurfaceCommandErrorKind,
    unaccepted: UnacceptedSurfaceCommand,
}

impl SubmitSurfaceCommandError {
    pub(crate) const fn new(
        kind: SubmitSurfaceCommandErrorKind,
        unaccepted: UnacceptedSurfaceCommand,
    ) -> Self {
        Self { kind, unaccepted }
    }

    /// Returns the rejection classification.
    #[must_use]
    pub const fn kind(&self) -> SubmitSurfaceCommandErrorKind {
        self.kind
    }

    /// Borrows the exact unaccepted request.
    pub const fn unaccepted(&self) -> &UnacceptedSurfaceCommand {
        &self.unaccepted
    }

    /// Recovers the exact unaccepted request.
    pub fn into_unaccepted(self) -> UnacceptedSurfaceCommand {
        self.unaccepted
    }
}

impl fmt::Debug for SubmitSurfaceCommandError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SubmitSurfaceCommandError")
            .field("kind", &self.kind)
            .field("unaccepted", &self.unaccepted)
            .finish()
    }
}

impl fmt::Display for SubmitSurfaceCommandError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.kind {
            SubmitSurfaceCommandErrorKind::Full => {
                formatter.write_str("runtime work queue is full")
            }
            SubmitSurfaceCommandErrorKind::Closed => formatter.write_str("runtime is closed"),
            SubmitSurfaceCommandErrorKind::Terminal(reason) => {
                write!(formatter, "runtime is terminal: {reason}")
            }
            SubmitSurfaceCommandErrorKind::ForeignSurfaceContext => {
                formatter.write_str("surface context belongs to another runtime")
            }
            SubmitSurfaceCommandErrorKind::ForeignSurface => {
                formatter.write_str("surface context names another logical surface")
            }
            SubmitSurfaceCommandErrorKind::RetiredSurfaceContext => {
                formatter.write_str("surface context generation has retired")
            }
            SubmitSurfaceCommandErrorKind::MissingSurfaceGeneration => {
                formatter.write_str("surface context generation was never retained")
            }
            SubmitSurfaceCommandErrorKind::CoordinateRevisionMismatch => {
                formatter.write_str("surface coordinate revision does not match its generation")
            }
            SubmitSurfaceCommandErrorKind::NoTarget => {
                formatter.write_str("logical position has no target in the named snapshot")
            }
            SubmitSurfaceCommandErrorKind::TargetNotInSnapshot => {
                formatter.write_str("resolved target is absent from the named snapshot")
            }
            SubmitSurfaceCommandErrorKind::ForeignTarget => {
                formatter.write_str("surface command target belongs to another runtime")
            }
            SubmitSurfaceCommandErrorKind::StaleTarget => {
                formatter.write_str("surface command target lifetime is stale")
            }
            SubmitSurfaceCommandErrorKind::MissingTarget => {
                formatter.write_str("surface command target has no mounted address")
            }
            SubmitSurfaceCommandErrorKind::WorkSequenceExhausted => {
                formatter.write_str("runtime work sequence is exhausted")
            }
            SubmitSurfaceCommandErrorKind::TraceSequenceExhausted => {
                formatter.write_str("enabled canonical trace sequence is exhausted")
            }
        }
    }
}

impl std::error::Error for SubmitSurfaceCommandError {}

#[cfg(test)]
mod tests {
    use runenui_core::{
        __runtime::RuntimeNamespace, CommandOrigin, SemanticCommand, SurfaceInputContext,
    };

    use super::{
        LogicalPoint, SubmitSurfaceCommandError, SubmitSurfaceCommandErrorKind,
        UnacceptedSurfaceCommand,
    };

    fn context(namespace: &RuntimeNamespace) -> SurfaceInputContext {
        namespace
            .__runtime_surface_context(namespace.__runtime_surface_id(0, 1), 2, 3)
            .unwrap_or_else(|| unreachable!("test context shares its namespace"))
    }

    #[test]
    fn logical_rejection_recovers_every_owned_input() {
        let namespace = RuntimeNamespace::__runtime_new();
        let context = context(&namespace);
        let point = LogicalPoint::new(4.0, 5.0).unwrap_or_else(|_| unreachable!());
        let command = SemanticCommand::Activate;
        let origin = CommandOrigin::programmatic();
        let error = SubmitSurfaceCommandError::new(
            SubmitSurfaceCommandErrorKind::NoTarget,
            UnacceptedSurfaceCommand::Logical {
                context: context.clone(),
                point,
                command,
                origin,
            },
        );

        match error.into_unaccepted() {
            UnacceptedSurfaceCommand::Logical {
                context: recovered_context,
                point: recovered_point,
                command: recovered_command,
                origin: recovered_origin,
            } => {
                assert_eq!(recovered_context, context);
                assert_eq!(recovered_point, point);
                assert_eq!(recovered_command, command);
                assert_eq!(recovered_origin, origin);
            }
            UnacceptedSurfaceCommand::Resolved { .. } => {
                unreachable!("logical rejection retains the logical request")
            }
        }
    }

    #[test]
    fn resolved_rejection_recovers_every_owned_input() {
        let namespace = RuntimeNamespace::__runtime_new();
        let context = context(&namespace);
        let target = namespace.__runtime_mounted_id(7, 9);
        let command = SemanticCommand::OpenContextMenu;
        let origin = CommandOrigin::automation();
        let error = SubmitSurfaceCommandError::new(
            SubmitSurfaceCommandErrorKind::MissingTarget,
            UnacceptedSurfaceCommand::Resolved {
                context: context.clone(),
                target: target.clone(),
                command,
                origin,
            },
        );

        match error.into_unaccepted() {
            UnacceptedSurfaceCommand::Resolved {
                context: recovered_context,
                target: recovered_target,
                command: recovered_command,
                origin: recovered_origin,
            } => {
                assert_eq!(recovered_context, context);
                assert_eq!(recovered_target, target);
                assert_eq!(recovered_command, command);
                assert_eq!(recovered_origin, origin);
            }
            UnacceptedSurfaceCommand::Logical { .. } => {
                unreachable!("resolved rejection retains the resolved request")
            }
        }
    }
}
