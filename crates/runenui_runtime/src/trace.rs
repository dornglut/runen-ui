//! Bounded canonical runtime trace records.

use core::num::NonZeroU64;
use std::collections::VecDeque;

use runenui_core::ElementId;

use crate::{MountedNodeId, ReconciliationGeneration, RuntimeTerminalReason, WorkSequence};

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
    ActivationRejectedFull,
    ApplicationActionTransactionStarted,
    ApplicationStateUpdated,
    TreeReconciled,
    FocusRetained,
    FocusCleared,
    PumpBudgetExhausted,
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

    pub(crate) fn can_record_mandatory(&self, count: usize) -> bool {
        if self.capacity == 0 || count == 0 {
            return true;
        }
        self.next_sequence.is_some_and(|next| {
            u64::try_from(count - 1)
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
        });
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

    #[cfg(any(test, feature = "internal-test-seams"))]
    pub(crate) const fn seed_next_sequence_for_test(&mut self, next: u64) {
        self.next_sequence = NonZeroU64::new(next);
    }
}
