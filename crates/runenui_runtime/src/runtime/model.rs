//! Runtime status, diagnostics, reconciliation, and terminal result vocabulary.

use super::{
    CommandOrigin, ElementKey, MountedNodeId, SemanticCommand, TraceSequence,
    WorkCancellationCounts, fmt,
};

pub(in crate::runtime) enum ActionCommitError<Action> {
    QueueFull(Action),
    WorkSequenceExhausted(Action),
    TraceSequenceExhausted(Action),
    Integrity(Action),
}

pub(in crate::runtime) enum CollectedRoutedOutput<Action> {
    Action {
        action: Action,
        causal_parent: Option<TraceSequence>,
        current_target: MountedNodeId,
    },
    Command {
        target: MountedNodeId,
        command: SemanticCommand,
        origin: CommandOrigin,
        causal_parent: Option<TraceSequence>,
    },
}

#[derive(Clone, Copy)]
pub(in crate::runtime) enum MutationPhase {
    PreMutation,
    Mutated,
}

impl MutationPhase {
    pub(in crate::runtime) const fn terminal_reason(
        self,
        pre_mutation_reason: RuntimeTerminalReason,
    ) -> RuntimeTerminalReason {
        match self {
            Self::PreMutation => pre_mutation_reason,
            Self::Mutated => RuntimeTerminalReason::Poisoned,
        }
    }
}

#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum RuntimeTerminalReason {
    WorkSequenceExhausted,
    WorkGenerationExhausted,
    ReconciliationGenerationExhausted,
    MountedIdentityExhausted,
    TraceSequenceExhausted,
    Poisoned,
}

impl fmt::Display for RuntimeTerminalReason {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WorkSequenceExhausted => formatter.write_str("work sequence exhausted"),
            Self::WorkGenerationExhausted => formatter.write_str("work generation exhausted"),
            Self::ReconciliationGenerationExhausted => {
                formatter.write_str("reconciliation generation exhausted")
            }
            Self::MountedIdentityExhausted => {
                formatter.write_str("mounted identity capacity exhausted")
            }
            Self::TraceSequenceExhausted => formatter.write_str("trace sequence exhausted"),
            Self::Poisoned => formatter.write_str("runtime integrity poisoned after mutation"),
        }
    }
}

#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuntimeStatus {
    Running,
    Terminal(RuntimeTerminalReason),
    Closed,
}

#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TimerStartOutcome {
    Started,
    ZeroInterval,
    DeadlineOverflow,
}

#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TimerFiringOutcome {
    Completed,
    Rescheduled,
    RepeatDeadlineOverflow,
}

#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SubscriptionOwnerKind {
    Application,
    Mounted,
}

#[non_exhaustive]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SubscriptionDiagnostic {
    DuplicateKey {
        owner: SubscriptionOwnerKind,
        key: runenui_core::WorkKey,
    },
}

pub enum HostResponseError<Response> {
    ForeignRuntime(Response),
    Stale(Response),
    MismatchedKind(Response),
    Full(Response),
    Closed(Response),
    Terminal {
        response: Response,
        reason: RuntimeTerminalReason,
    },
}

impl<Response> fmt::Debug for HostResponseError<Response> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::ForeignRuntime(_) => "HostResponseError::ForeignRuntime(..)",
            Self::Stale(_) => "HostResponseError::Stale(..)",
            Self::MismatchedKind(_) => "HostResponseError::MismatchedKind(..)",
            Self::Full(_) => "HostResponseError::Full(..)",
            Self::Closed(_) => "HostResponseError::Closed(..)",
            Self::Terminal { .. } => "HostResponseError::Terminal { .. }",
        })
    }
}

#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HostRequestCancelError {
    ForeignRuntime,
    Stale,
    Full,
    Closed,
    Terminal(RuntimeTerminalReason),
}

#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RuntimeError {
    WidgetStatePayloadMismatch,
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::WidgetStatePayloadMismatch => {
                formatter.write_str("mounted widget state payload mismatch")
            }
        }
    }
}

impl std::error::Error for RuntimeError {}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct ReconciliationGeneration(pub(in crate::runtime) u64);

impl ReconciliationGeneration {
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReconciliationReport {
    pub(in crate::runtime) generation: ReconciliationGeneration,
    pub(in crate::runtime) live_node_count: usize,
    pub(in crate::runtime) mounted_count: usize,
    pub(in crate::runtime) updated_count: usize,
    pub(in crate::runtime) unmounted_count: usize,
    pub(in crate::runtime) moved_count: usize,
    pub(in crate::runtime) retained_focus: bool,
    pub(in crate::runtime) diagnostics: Vec<ReconciliationDiagnostic>,
}

#[non_exhaustive]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ReconciliationDiagnostic {
    DuplicateSiblingKey {
        key: ElementKey,
        parent_path: String,
        old_occurrence_paths: Vec<String>,
        new_occurrence_paths: Vec<String>,
    },
    StatePayloadMismatch {
        path: String,
    },
}

impl fmt::Display for ReconciliationDiagnostic {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateSiblingKey {
                key,
                parent_path,
                old_occurrence_paths,
                new_occurrence_paths,
            } => write!(
                formatter,
                "duplicate sibling key {:?} under {parent_path}; old=[{}], new=[{}]",
                key.as_str(),
                old_occurrence_paths.join(", "),
                new_occurrence_paths.join(", ")
            ),
            Self::StatePayloadMismatch { path } => {
                write!(formatter, "mounted widget state payload mismatch at {path}")
            }
        }
    }
}

impl ReconciliationReport {
    #[must_use]
    pub const fn generation(&self) -> ReconciliationGeneration {
        self.generation
    }
    #[must_use]
    pub const fn live_node_count(&self) -> usize {
        self.live_node_count
    }
    #[must_use]
    pub const fn mounted_count(&self) -> usize {
        self.mounted_count
    }
    #[must_use]
    pub const fn updated_count(&self) -> usize {
        self.updated_count
    }
    #[must_use]
    pub const fn unmounted_count(&self) -> usize {
        self.unmounted_count
    }
    #[must_use]
    pub const fn moved_count(&self) -> usize {
        self.moved_count
    }
    #[must_use]
    pub const fn retained_focus(&self) -> bool {
        self.retained_focus
    }
    #[must_use]
    pub fn diagnostics(&self) -> &[ReconciliationDiagnostic] {
        &self.diagnostics
    }
}

/// Result of one explicit, idempotent runtime shutdown.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ShutdownReport {
    pub(in crate::runtime) already_complete: bool,
    pub(in crate::runtime) cancelled_queued_envelopes: usize,
    pub(in crate::runtime) unmounted_lifetimes: usize,
    pub(in crate::runtime) cancelled_live_work: WorkCancellationCounts,
}

impl ShutdownReport {
    #[must_use]
    pub const fn already_complete(self) -> bool {
        self.already_complete
    }
    #[must_use]
    pub const fn cancelled_queued_envelopes(self) -> usize {
        self.cancelled_queued_envelopes
    }
    #[must_use]
    pub const fn unmounted_lifetimes(self) -> usize {
        self.unmounted_lifetimes
    }
    #[must_use]
    pub const fn cancelled_local_tasks(self) -> usize {
        self.cancelled_live_work.local_tasks
    }
    #[must_use]
    pub const fn cancelled_send_tasks(self) -> usize {
        self.cancelled_live_work.send_tasks
    }
    #[must_use]
    pub const fn cancelled_timers(self) -> usize {
        self.cancelled_live_work.timers
    }
    #[must_use]
    pub const fn cancelled_subscriptions(self) -> usize {
        self.cancelled_live_work.subscriptions
    }
    #[must_use]
    pub const fn cancelled_host_requests(self) -> usize {
        self.cancelled_live_work.host_requests
    }
}
