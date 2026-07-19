//! Bounded canonical runtime trace records.

use core::num::NonZeroU64;
use std::collections::VecDeque;

use runenui_core::{
    CommandOrigin, ElementId, EventPhase, MonotonicInstant, SemanticCommand, WidgetInvalidation,
    WorkKey,
};

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

    pub(super) fn routed_event(route_invocations: usize, max_outputs: usize) -> Option<Self> {
        Self::exact(6)
            .checked_add(Self::exact(6).checked_mul(route_invocations)?)
            .and_then(|plan| plan.checked_add(Self::exact(6).checked_mul(max_outputs)?))
            .and_then(|plan| {
                // Every collected output may be a delegated command. Its accepted
                // envelope must retain one future processing-outcome sequence.
                plan.checked_add(Self::exact(max_outputs))
            })
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

/// Private authority retained by one accepted command for exactly one future
/// processing outcome. Disabled tracing carries a no-op reservation so command
/// behavior remains identical when canonical retention is disabled.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TraceReservation {
    active: bool,
}

impl TraceReservation {
    const DISABLED: Self = Self { active: false };
    const ACTIVE: Self = Self { active: true };

    pub(crate) const fn is_active(self) -> bool {
        self.active
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
    CommandSubmissionAccepted,
    CommandProcessingRejected {
        outcome: TraceTargetRejection,
    },
    RoutedEventStarted,
    RouteSnapshotCreated {
        invocations: usize,
    },
    EventPhaseInvoked {
        phase: EventPhase,
    },
    RoutedActionCollected,
    DelegatedCommandCollected {
        command: SemanticCommand,
    },
    PropagationStopped,
    DefaultPrevented,
    WidgetStateMutated,
    WidgetInvalidated {
        invalidation: WidgetInvalidation,
    },
    MountedSubscriptionInvalidated,
    SemanticDefaultApplied {
        command: SemanticCommand,
    },
    SemanticDefaultSuppressed {
        command: SemanticCommand,
    },
    RoutedEventCommitted,
    RoutedIntegrityFailed {
        failure: TraceRoutedIntegrityFailure,
    },
    RoutedEventAdmissionRejected {
        capacity: TraceRoutedAdmissionRejection,
    },
    ActionSubmissionRejectedFull,
    ActionSubmissionRejectedClosed,
    ActionSubmissionRejectedTerminal,
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

/// Exact routed integrity boundary that failed after command acceptance.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TraceRoutedIntegrityFailure {
    BrokenTopology,
    EventBridgeMismatch,
    CallbackBridgeFailure,
    OutputAllowanceExceeded,
    SemanticDefaultFailure,
    CommitInvariantFailure,
}

/// Exact target-lifetime rejection while processing an accepted command.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TraceTargetRejection {
    Foreign,
    Stale,
    Missing,
}

/// Bounded authority that refused an accepted routed transaction preflight.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TraceRoutedAdmissionRejection {
    TransactionOutputs,
    WaitingEnvelopes,
    LocalTasks,
    SendTasks,
    Timers,
    WorkSequenceExhausted,
    WorkGenerationExhausted,
    ReconciliationGenerationExhausted,
    TraceSequenceExhausted,
    CheckedArithmeticOverflow,
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
    instant: Option<MonotonicInstant>,
    original_target: Option<MountedNodeId>,
    current_target: Option<MountedNodeId>,
    command_origin: Option<CommandOrigin>,
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

    /// Returns the accepted routed transaction time, when applicable.
    #[must_use]
    pub const fn instant(&self) -> Option<MonotonicInstant> {
        self.instant
    }

    /// Returns the immutable original routed target, when applicable.
    #[must_use]
    pub const fn original_target(&self) -> Option<&MountedNodeId> {
        self.original_target.as_ref()
    }

    /// Returns the callback's current routed target, when applicable.
    #[must_use]
    pub const fn current_target(&self) -> Option<&MountedNodeId> {
        self.current_target.as_ref()
    }

    /// Returns the normalized command origin, when applicable.
    #[must_use]
    pub const fn command_origin(&self) -> Option<CommandOrigin> {
        self.command_origin
    }
}

/// One bounded canonical trace store.
#[derive(Debug, Eq, PartialEq)]
pub struct Trace {
    capacity: usize,
    records: VecDeque<TraceRecord>,
    next_sequence: Option<NonZeroU64>,
    reserved_records: usize,
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
            reserved_records: 0,
            dropped_before_sequence: None,
        }
    }

    pub(crate) const fn is_enabled(&self) -> bool {
        self.capacity != 0
    }

    pub(crate) fn can_admit(&self, plan: MandatoryTracePlan) -> bool {
        self.can_admit_with_reserved(plan, self.reserved_records)
    }

    fn can_admit_with_reserved(&self, plan: MandatoryTracePlan, reserved: usize) -> bool {
        if !self.is_enabled() {
            return true;
        }
        let Some(required) = reserved.checked_add(plan.records) else {
            return false;
        };
        if required == 0 {
            return true;
        }
        self.next_sequence.is_some_and(|next| {
            u64::try_from(required - 1)
                .ok()
                .and_then(|additional| next.get().checked_add(additional))
                .is_some()
        })
    }

    pub(crate) fn reserve_command_outcome(&mut self) -> Option<TraceReservation> {
        if !self.is_enabled() {
            return Some(TraceReservation::DISABLED);
        }
        let reserved = self.reserved_records.checked_add(1)?;
        if !self.can_admit_with_reserved(MandatoryTracePlan::exact(0), reserved) {
            return None;
        }
        self.reserved_records = reserved;
        Some(TraceReservation::ACTIVE)
    }

    pub(crate) fn can_replace_reservation(
        &self,
        reservation: TraceReservation,
        plan: MandatoryTracePlan,
    ) -> bool {
        if !reservation.is_active() {
            return self.can_admit(plan);
        }
        let Some(reserved) = self.reserved_records.checked_sub(1) else {
            return false;
        };
        self.can_admit_with_reserved(plan, reserved)
    }

    pub(crate) fn release_reservation(&mut self, reservation: TraceReservation) {
        if reservation.is_active() {
            self.reserved_records = self
                .reserved_records
                .checked_sub(1)
                .unwrap_or_else(|| unreachable!("accepted command retains one trace reservation"));
        }
    }

    pub(crate) fn release_reservations(&mut self, count: usize) {
        self.reserved_records = self
            .reserved_records
            .checked_sub(count)
            .unwrap_or_else(|| unreachable!("queued commands retain exact trace reservations"));
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn record_reserved_event(
        &mut self,
        reservation: TraceReservation,
        kind: TraceRecordKind,
        work_sequence: WorkSequence,
        causal_parent: Option<TraceSequence>,
        target: Option<TraceTarget>,
        instant: MonotonicInstant,
        original_target: &MountedNodeId,
        current_target: Option<&MountedNodeId>,
        origin: CommandOrigin,
    ) -> Option<TraceSequence> {
        self.release_reservation(reservation);
        self.record_event(
            kind,
            work_sequence,
            causal_parent,
            target,
            instant,
            original_target,
            current_target,
            origin,
        )
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
        if !self.can_admit(MandatoryTracePlan::one_fact()) {
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
            instant: None,
            original_target: None,
            current_target: None,
            command_origin: None,
        });
        Some(sequence)
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn record_event(
        &mut self,
        kind: TraceRecordKind,
        work_sequence: WorkSequence,
        causal_parent: Option<TraceSequence>,
        target: Option<TraceTarget>,
        instant: MonotonicInstant,
        original_target: &MountedNodeId,
        current_target: Option<&MountedNodeId>,
        origin: CommandOrigin,
    ) -> Option<TraceSequence> {
        let sequence = self.record(kind, Some(work_sequence), causal_parent, None, None, target)?;
        if let Some(record) = self.records.back_mut() {
            record.instant = Some(instant);
            record.original_target = Some(original_target.clone());
            record.current_target = current_target.cloned();
            record.command_origin = Some(origin);
        }
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

    #[cfg(feature = "internal-test-seams")]
    pub(crate) const fn reserved_records_for_test(&self) -> usize {
        self.reserved_records
    }

    #[cfg(feature = "internal-test-seams")]
    pub(crate) const fn next_sequence_for_test(&self) -> Option<u64> {
        match self.next_sequence {
            Some(sequence) => Some(sequence.get()),
            None => None,
        }
    }
}
