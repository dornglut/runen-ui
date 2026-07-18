//! Bounded canonical runtime trace records.

use core::num::NonZeroU64;
use std::collections::VecDeque;

use runenui_core::{ElementId, WorkKey};

use crate::{MountedNodeId, ReconciliationGeneration, RuntimeTerminalReason, WorkSequence};

/// Checked admission requirement for one mutation boundary.
///
/// Constructors describe either the exact mandatory records for an operation or
/// the documented maximum path that must be admitted before user code runs.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MandatoryTracePlan {
    records: usize,
}

impl MandatoryTracePlan {
    const fn exact(records: usize) -> Self {
        Self { records }
    }

    pub(super) const fn action_acceptance() -> Self {
        Self::exact(1)
    }

    pub(super) const fn one_fact() -> Self {
        Self::exact(1)
    }

    pub(super) const fn work_cancellation() -> Self {
        Self::exact(2)
    }

    pub(super) const fn send_completion() -> Self {
        Self::exact(3)
    }

    pub(super) const fn host_completion() -> Self {
        Self::exact(4)
    }

    pub(super) const fn callback_with_action() -> Self {
        Self::exact(3)
    }

    pub(super) const fn typed_start_refusal_with_action() -> Self {
        Self::exact(1)
    }

    pub(super) const fn work_start(host_request: bool) -> Self {
        Self::exact(if host_request { 3 } else { 2 })
    }

    pub(super) const fn application_action_base(has_focus: bool) -> Self {
        Self::exact(if has_focus { 4 } else { 3 })
    }

    pub(super) fn lifecycle_invalidations(count: usize) -> Option<Self> {
        Self::exact(2).checked_mul(count)
    }

    pub(super) fn activation_maximum(outputs: usize) -> Option<Self> {
        Self::exact(4)
            .checked_mul(outputs)
            .and_then(|plan| plan.checked_add(Self::one_fact()))
    }

    pub(super) fn activation_commit(
        invalidated: usize,
        starts: usize,
        actions: usize,
    ) -> Option<Self> {
        Self::exact(2)
            .checked_mul(invalidated)
            .and_then(|plan| {
                Self::exact(2)
                    .checked_mul(starts)
                    .and_then(|starts| plan.checked_add(starts))
            })
            .and_then(|plan| plan.checked_add(Self::exact(actions)))
            .and_then(|plan| plan.checked_add(Self::one_fact()))
    }

    pub(super) const fn planned_scheduler_transaction(records: usize) -> Self {
        Self::exact(records)
    }

    pub(super) fn checked_add(self, other: Self) -> Option<Self> {
        self.records.checked_add(other.records).map(Self::exact)
    }

    pub(super) fn checked_mul(self, count: usize) -> Option<Self> {
        self.records.checked_mul(count).map(Self::exact)
    }
}

/// Non-wrapping identity of one canonical trace record.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct TraceSequence(NonZeroU64);

impl TraceSequence {
    /// Returns the numeric sequence value.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0.get()
    }

    const fn new(value: NonZeroU64) -> Self {
        Self(value)
    }
}

/// Configuration for canonical in-memory trace retention.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TraceConfig {
    capacity: usize,
}

impl TraceConfig {
    /// Creates trace configuration with the requested retained-record capacity.
    #[must_use]
    pub const fn new(capacity: usize) -> Self {
        Self { capacity }
    }

    /// Returns the retained-record capacity.
    #[must_use]
    pub const fn capacity(self) -> usize {
        self.capacity
    }
}

impl Default for TraceConfig {
    fn default() -> Self {
        Self::new(1024)
    }
}

/// Trace target for runtime work caused by a specific mounted node.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TraceTarget {
    mounted_node_id: MountedNodeId,
    authored_id: Option<ElementId>,
}

impl TraceTarget {
    #[must_use]
    pub(crate) const fn new(
        mounted_node_id: MountedNodeId,
        authored_id: Option<ElementId>,
    ) -> Self {
        Self {
            mounted_node_id,
            authored_id,
        }
    }

    /// Returns the mounted node identity for this target.
    #[must_use]
    pub const fn mounted_node_id(&self) -> &MountedNodeId {
        &self.mounted_node_id
    }

    /// Returns the optional authored element identity for this target.
    #[must_use]
    pub const fn authored_id(&self) -> Option<&ElementId> {
        self.authored_id.as_ref()
    }
}

/// Structured kind of one canonical trace record.
#[non_exhaustive]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TraceRecordKind {
    RuntimeMounted,
    ActionSubmissionAccepted,
    ActionSubmissionRejectedFull,
    ActionSubmissionRejectedClosed,
    ActionSubmissionRejectedTerminal,
    ActivationCommitted,
    ActivationRejectedSaturated {
        capacity: crate::ActivationCapacity,
    },
    ApplicationActionTransactionStarted,
    ApplicationStateUpdated,
    TreeReconciled,
    FocusRetained,
    FocusCleared,
    PumpBudgetExhausted,
    InitialEffectsCommitted {
        count: usize,
    },
    InitialApplicationTransactionStarted,
    UpdateEffectsCommitted {
        count: usize,
    },
    WorkRequested,
    WorkGenerationCommitted,
    WorkStartAttempted,
    WorkStartAccepted,
    WorkStartRefused {
        outcome: TraceWorkStartRefusal,
    },
    WorkLogicallyInvalidated,
    WorkCancellationBound,
    WorkCleanupProcessed,
    WorkCompletionImported,
    WorkCompletionRejectedStale,
    WorkCompletionMapped,
    LocalWorkPolled,
    LocalWorkReady,
    TimerPromoted,
    ReadinessCheckpoint {
        imported_completions: usize,
        polled_local_work: usize,
        promoted_timers: usize,
    },
    SubscriptionDeclared,
    SubscriptionDiffCommitted {
        started: usize,
        cancelled: usize,
        duplicate_keys: usize,
    },
    MountedSubscriptionReconciliationSuppressedStale,
    TimerFired,
    TimerTerminated {
        outcome: TraceTimerTerminalOutcome,
    },
    HostRequestExposed,
    HostResponseAccepted,
    HostResponseRejected,
    WakeRequested,
    WakeAcknowledged,
    RedrawRequested {
        revision: u64,
    },
    RedrawTaken {
        revision: u64,
    },
    RedrawAcknowledged {
        revision: u64,
    },
    QueuedWorkCancelled {
        count: usize,
    },
    RuntimeTerminal {
        reason: RuntimeTerminalReason,
    },
    RuntimeShutdown {
        cancelled_queued: usize,
        unmounted_lifetimes: usize,
    },
}

#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TraceWorkFamily {
    LocalTask,
    SendTask,
    Timer,
    Subscription,
    HostRequest,
}

/// Public owner classification for opaque scheduler trace identity.
#[non_exhaustive]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TraceWorkOwner {
    Application,
    Mounted(MountedNodeId),
}

/// Opaque, read-only identity of one exact scheduler work generation.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TraceWorkIdentity {
    owner: TraceWorkOwner,
    family: TraceWorkFamily,
    generation: u64,
    key: Option<WorkKey>,
}

impl TraceWorkIdentity {
    pub(crate) const fn new(
        owner: TraceWorkOwner,
        family: TraceWorkFamily,
        generation: u64,
        key: Option<WorkKey>,
    ) -> Self {
        Self {
            owner,
            family,
            generation,
            key,
        }
    }

    /// Returns the application or exact mounted owner classification.
    #[must_use]
    pub const fn owner(&self) -> &TraceWorkOwner {
        &self.owner
    }

    /// Returns the scheduler family.
    #[must_use]
    pub const fn family(&self) -> TraceWorkFamily {
        self.family
    }

    /// Returns the exact private generation as a read-only diagnostic value.
    #[must_use]
    pub const fn generation(&self) -> u64 {
        self.generation
    }

    /// Returns the optional authored key.
    #[must_use]
    pub const fn key(&self) -> Option<&WorkKey> {
        self.key.as_ref()
    }
}

/// Structured executor or timer start refusal.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TraceWorkStartRefusal {
    ExecutorUnavailable,
    ExecutorFull,
    ExecutorClosed,
    ExecutorRejected,
    SubscriptionUnavailable,
    SubscriptionFull,
    SubscriptionClosed,
    SubscriptionRejected,
    TimerZeroInterval,
    TimerDeadlineOverflow,
}

/// Structured terminal outcome of one timer generation.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TraceTimerTerminalOutcome {
    Completed,
    RepeatDeadlineOverflow,
}

/// One immutable canonical trace record.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TraceRecord {
    sequence: TraceSequence,
    kind: TraceRecordKind,
    work_sequence: Option<WorkSequence>,
    causal_parent: Option<TraceSequence>,
    reconciliation_before: Option<ReconciliationGeneration>,
    reconciliation_after: Option<ReconciliationGeneration>,
    target: Option<TraceTarget>,
    work: Option<TraceWorkIdentity>,
}

impl TraceRecord {
    /// Returns this record's trace sequence.
    #[must_use]
    pub const fn sequence(&self) -> TraceSequence {
        self.sequence
    }

    /// Returns this record's structured kind.
    #[must_use]
    pub const fn kind(&self) -> &TraceRecordKind {
        &self.kind
    }

    /// Returns the associated global work sequence, when applicable.
    #[must_use]
    pub const fn work_sequence(&self) -> Option<WorkSequence> {
        self.work_sequence
    }

    /// Returns the causal parent trace sequence, when applicable.
    #[must_use]
    pub const fn causal_parent(&self) -> Option<TraceSequence> {
        self.causal_parent
    }

    /// Returns the reconciliation generation before this record's transaction.
    #[must_use]
    pub const fn reconciliation_before(&self) -> Option<ReconciliationGeneration> {
        self.reconciliation_before
    }

    /// Returns the reconciliation generation after this record's transaction.
    #[must_use]
    pub const fn reconciliation_after(&self) -> Option<ReconciliationGeneration> {
        self.reconciliation_after
    }

    /// Returns the mounted trace target, when applicable.
    #[must_use]
    pub const fn target(&self) -> Option<&TraceTarget> {
        self.target.as_ref()
    }

    /// Returns the exact scheduler work identity for work-specific facts.
    #[must_use]
    pub const fn work(&self) -> Option<&TraceWorkIdentity> {
        self.work.as_ref()
    }
}

/// One bounded canonical trace store.
#[derive(Debug, Eq, PartialEq)]
pub struct Trace {
    capacity: usize,
    records: VecDeque<TraceRecord>,
    next_sequence: Option<NonZeroU64>,
    dropped_before_sequence: Option<TraceSequence>,
}

impl Trace {
    #[must_use]
    pub(crate) const fn new(config: TraceConfig) -> Self {
        let capacity = config.capacity();
        Self {
            capacity,
            records: VecDeque::new(),
            next_sequence: NonZeroU64::new(1),
            dropped_before_sequence: None,
        }
    }

    pub(crate) const fn is_enabled(&self) -> bool {
        self.capacity != 0
    }

    pub(crate) fn can_admit(&self, plan: MandatoryTracePlan) -> bool {
        if !self.is_enabled() || plan.records == 0 {
            return true;
        }
        self.next_sequence.is_some_and(|next| {
            u64::try_from(plan.records - 1)
                .ok()
                .and_then(|additional| next.get().checked_add(additional))
                .is_some()
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn record(
        &mut self,
        kind: TraceRecordKind,
        work_sequence: Option<WorkSequence>,
        causal_parent: Option<TraceSequence>,
        reconciliation_before: Option<ReconciliationGeneration>,
        reconciliation_after: Option<ReconciliationGeneration>,
        target: Option<TraceTarget>,
    ) -> Option<TraceSequence> {
        if self.capacity == 0 {
            return None;
        }
        let sequence = TraceSequence::new(self.next_sequence?);
        self.next_sequence = sequence.get().checked_add(1).and_then(NonZeroU64::new);
        if self.records.len() == self.capacity
            && let Some(evicted) = self.records.pop_front()
            && let Some(next) = evicted
                .sequence
                .get()
                .checked_add(1)
                .and_then(NonZeroU64::new)
        {
            self.dropped_before_sequence = Some(TraceSequence::new(next));
        }
        self.records.push_back(TraceRecord {
            sequence,
            kind,
            work_sequence,
            causal_parent,
            reconciliation_before,
            reconciliation_after,
            target,
            work: None,
        });
        Some(sequence)
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn record_work(
        &mut self,
        kind: TraceRecordKind,
        work_sequence: Option<WorkSequence>,
        causal_parent: Option<TraceSequence>,
        reconciliation_before: Option<ReconciliationGeneration>,
        reconciliation_after: Option<ReconciliationGeneration>,
        target: Option<TraceTarget>,
        work: TraceWorkIdentity,
    ) -> Option<TraceSequence> {
        let sequence = self.record(
            kind,
            work_sequence,
            causal_parent,
            reconciliation_before,
            reconciliation_after,
            target,
        )?;
        if let Some(record) = self.records.back_mut() {
            record.work = Some(work);
        }
        Some(sequence)
    }

    /// Borrows canonical records from oldest retained to newest.
    #[must_use]
    pub fn records(&self) -> impl ExactSizeIterator<Item = &TraceRecord> {
        self.records.iter()
    }

    /// Borrows structured record kinds from oldest retained to newest.
    #[must_use]
    pub fn kinds(&self) -> impl ExactSizeIterator<Item = &TraceRecordKind> {
        self.records.iter().map(TraceRecord::kind)
    }

    /// Returns the number of retained records.
    #[must_use]
    pub fn len(&self) -> usize {
        self.records.len()
    }

    /// Returns whether no records are retained.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    /// Returns the exclusive watermark for records evicted from retention.
    #[must_use]
    pub const fn dropped_before_sequence(&self) -> Option<TraceSequence> {
        self.dropped_before_sequence
    }

    #[cfg(feature = "internal-test-seams")]
    pub(crate) const fn seed_next_sequence_for_test(&mut self, next: u64) {
        self.next_sequence = NonZeroU64::new(next);
    }
}
