//! Canonical application-work queue and work sequencing.

#![allow(clippy::redundant_pub_crate)]

mod input_continuations;
pub(crate) use input_continuations::InputContinuationKey;

use core::{fmt, num::NonZeroU64};
use std::{
    collections::{HashMap, VecDeque},
    sync::Arc,
};

use runenui_core::{
    ApplicationCommandId, CommandOrigin, CommittedTextEvent, CompositionEvent, FocusReason,
    FrameworkServiceResponse, KeyboardEvent, PointerEvent, ProducerBudget, SemanticActionTarget,
    SemanticCommand, SurfaceInputContext,
};

use crate::trace::TraceReservation;
use crate::{MonotonicInstant, MountedNodeId, focus::FocusSession, work::WorkGeneration};
use crate::{RuntimeTerminalReason, TraceSequence, TraceTarget, TraceWorkIdentity};
pub use runenui_core::WorkSequence;

/// Borrowed classification of an action-submission failure.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SubmitActionErrorKind {
    Full,
    WorkCapacity(crate::WorkCapacityFamily),
    InvalidProducerProfile,
    Closed,
    Terminal(RuntimeTerminalReason),
}

/// An unaccepted application action and the exact rejection reason.
#[must_use]
#[non_exhaustive]
pub enum SubmitActionError<Action> {
    Full(Action),
    WorkCapacity {
        action: Action,
        family: crate::WorkCapacityFamily,
    },
    InvalidProducerProfile(Action),
    Closed(Action),
    Terminal {
        action: Action,
        reason: RuntimeTerminalReason,
    },
}

impl<Action> SubmitActionError<Action> {
    /// Returns the rejection classification without borrowing the action.
    #[must_use]
    pub const fn kind(&self) -> SubmitActionErrorKind {
        match self {
            Self::Full(_) => SubmitActionErrorKind::Full,
            Self::WorkCapacity { family, .. } => SubmitActionErrorKind::WorkCapacity(*family),
            Self::InvalidProducerProfile(_) => SubmitActionErrorKind::InvalidProducerProfile,
            Self::Closed(_) => SubmitActionErrorKind::Closed,
            Self::Terminal { reason, .. } => SubmitActionErrorKind::Terminal(*reason),
        }
    }

    /// Recovers the exact unaccepted action.
    #[must_use]
    pub fn into_action(self) -> Action {
        match self {
            Self::Full(action)
            | Self::InvalidProducerProfile(action)
            | Self::Closed(action)
            | Self::WorkCapacity { action, .. }
            | Self::Terminal { action, .. } => action,
        }
    }
}

impl<Action> fmt::Debug for SubmitActionError<Action> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("SubmitActionError")
            .field("kind", &self.kind())
            .finish_non_exhaustive()
    }
}

impl<Action> fmt::Display for SubmitActionError<Action> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.kind() {
            SubmitActionErrorKind::Full => formatter.write_str("runtime work queue is full"),
            SubmitActionErrorKind::WorkCapacity(family) => {
                write!(
                    formatter,
                    "runtime {family:?} capacity is reserved or occupied"
                )
            }
            SubmitActionErrorKind::InvalidProducerProfile => {
                formatter.write_str("application producer profile cannot fit runtime limits")
            }
            SubmitActionErrorKind::Closed => formatter.write_str("runtime is closed"),
            SubmitActionErrorKind::Terminal(reason) => {
                write!(formatter, "runtime is terminal: {reason}")
            }
        }
    }
}

/// Result of submitting one owned application action.
pub type SubmitActionResult<Action> = Result<WorkSequence, SubmitActionError<Action>>;

/// Exact private target carried by one canonical semantic-command queue append.
///
/// Semantic-origin metadata augments the existing mounted command target; it
/// does not create a second queue or alternate routing address.
pub(crate) struct SemanticCommandQueueTarget {
    target: MountedNodeId,
    semantic_target: Option<SemanticActionTarget>,
}

impl SemanticCommandQueueTarget {
    pub(crate) const fn mounted(target: MountedNodeId) -> Self {
        Self {
            target,
            semantic_target: None,
        }
    }

    pub(crate) const fn semantic(
        target: MountedNodeId,
        semantic_target: SemanticActionTarget,
    ) -> Self {
        Self {
            target,
            semantic_target: Some(semantic_target),
        }
    }
}

pub(crate) struct ApplicationCommandEnvelope {
    pub(crate) sequence: WorkSequence,
    pub(crate) target: MountedNodeId,
    pub(crate) command: ApplicationCommandId,
    pub(crate) origin: CommandOrigin,
    pub(crate) instant: MonotonicInstant,
    pub(crate) causal_parent: Option<TraceSequence>,
    pub(crate) trace_reservation: TraceReservation,
    pub(crate) root_quota: RootQuotaId,
}

pub(crate) struct ApplicationCommandQueuePayload {
    pub(crate) target: MountedNodeId,
    pub(crate) command: ApplicationCommandId,
    pub(crate) origin: CommandOrigin,
    pub(crate) instant: MonotonicInstant,
    pub(crate) causal_parent: Option<TraceSequence>,
    pub(crate) trace_reservation: TraceReservation,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct FocusRequestOverride {
    pub(crate) target: Option<MountedNodeId>,
    pub(crate) reason: FocusReason,
    pub(crate) presentation_entry: Option<PresentationFocusEntry>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PresentationFocusEntry {
    pub(crate) root: MountedNodeId,
    pub(crate) activation_generation: u64,
    pub(crate) focused_session_at_request: Option<FocusSession>,
}

pub(crate) struct SemanticCommandEnvelope {
    pub(crate) sequence: WorkSequence,
    pub(crate) target: MountedNodeId,
    pub(crate) command: SemanticCommand,
    pub(crate) origin: CommandOrigin,
    pub(crate) semantic_target: Option<SemanticActionTarget>,
    pub(crate) focus_request_override: Option<FocusRequestOverride>,
    pub(crate) presentation_cancel_origin:
        Option<crate::runtime::presentation::PresentationCancelOrigin>,
    pub(crate) instant: MonotonicInstant,
    pub(crate) causal_parent: Option<TraceSequence>,
    pub(crate) trace_reservation: TraceReservation,
    pub(crate) root_quota: RootQuotaId,
}

pub(crate) struct SemanticCommandQueuePayload {
    pub(crate) queued_target: SemanticCommandQueueTarget,
    pub(crate) command: SemanticCommand,
    pub(crate) origin: CommandOrigin,
    pub(crate) focus_request_override: Option<FocusRequestOverride>,
    pub(crate) presentation_cancel_origin:
        Option<crate::runtime::presentation::PresentationCancelOrigin>,
    pub(crate) instant: MonotonicInstant,
    pub(crate) causal_parent: Option<TraceSequence>,
    pub(crate) trace_reservation: TraceReservation,
    pub(crate) root_quota: RootQuotaId,
}

pub(crate) enum PointerEnvelopePayload {
    Event(PointerEvent),
    StationaryRehit(SurfaceInputContext),
}

pub(crate) struct PointerEnvelope {
    pub(crate) sequence: WorkSequence,
    pub(crate) root_quota: Option<RootQuotaId>,
    pub(crate) payload: PointerEnvelopePayload,
    pub(crate) instant: MonotonicInstant,
    pub(crate) causal_parent: Option<TraceSequence>,
    pub(crate) trace_reservation: TraceReservation,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum FocusInputBinding {
    Committed(FocusSession),
    PendingEntry {
        request: WorkSequence,
        presentation: PresentationFocusEntry,
    },
    Uncertified,
}

pub(crate) enum InputEnvelopePayload {
    Keyboard(KeyboardEvent),
    CommittedText(CommittedTextEvent),
    Composition(CompositionEvent),
}

pub(crate) struct InputEnvelope {
    pub(crate) sequence: WorkSequence,
    pub(crate) target: MountedNodeId,
    pub(crate) focus_binding: Option<FocusInputBinding>,
    pub(crate) text_revoked: bool,
    pub(crate) root_quota: Option<RootQuotaId>,
    pub(crate) payload: InputEnvelopePayload,
    pub(crate) instant: MonotonicInstant,
    pub(crate) causal_parent: Option<TraceSequence>,
    pub(crate) trace_reservation: TraceReservation,
}

pub(crate) struct ApplicationActionEnvelope<Action> {
    pub(crate) sequence: WorkSequence,
    pub(crate) action: Action,
    pub(crate) producer_budget: ProducerBudget,
    pub(crate) root_quota: RootQuotaId,
    pub(crate) causal_parent: Option<TraceSequence>,
    pub(crate) target: Option<TraceTarget>,
    pub(crate) origin: ApplicationActionOrigin,
}

#[derive(Clone)]
pub(crate) enum ApplicationActionOrigin {
    Ordinary,
    Edit(crate::editing::EditActionOrigin),
}

pub(crate) enum WorkEnvelope<Action> {
    ApplicationAction(ApplicationActionEnvelope<Action>),
    SemanticCommand(SemanticCommandEnvelope),
    ApplicationCommand(ApplicationCommandEnvelope),
    Pointer(PointerEnvelope),
    Input(InputEnvelope),
    EffectStart(SequencedWork),
    WorkCancellation(CancellationEnvelope),
    FrameworkServiceResponse(FrameworkServiceResponseEnvelope),
    TimerFiring(SequencedWork),
    MountedSubscriptionReconcile {
        sequence: WorkSequence,
        owner: MountedNodeId,
        causal_parent: Option<TraceSequence>,
        root_quota: Option<RootQuotaId>,
    },
}

impl<Action> WorkEnvelope<Action> {
    /// Correlates the work already sequenced by the only canonical FIFO.
    #[must_use]
    pub(crate) const fn sequence(&self) -> WorkSequence {
        match self {
            Self::ApplicationAction(work) => work.sequence,
            Self::SemanticCommand(work) => work.sequence,
            Self::ApplicationCommand(work) => work.sequence,
            Self::Pointer(work) => work.sequence,
            Self::Input(work) => work.sequence,
            Self::EffectStart(work) | Self::TimerFiring(work) => work.sequence,
            Self::WorkCancellation(work) => work.sequence,
            Self::FrameworkServiceResponse(work) => work.sequence,
            Self::MountedSubscriptionReconcile { sequence, .. } => *sequence,
        }
    }

    pub(crate) const fn root_quota(&self) -> Option<RootQuotaId> {
        match self {
            Self::ApplicationAction(work) => Some(work.root_quota),
            Self::EffectStart(work) | Self::TimerFiring(work) => work.root_quota,
            Self::WorkCancellation(work) => work.root_quota,
            Self::FrameworkServiceResponse(work) => work.root_quota,
            Self::MountedSubscriptionReconcile { root_quota, .. } => *root_quota,
            Self::SemanticCommand(work) => Some(work.root_quota),
            Self::ApplicationCommand(work) => Some(work.root_quota),
            Self::Input(work) => work.root_quota,
            Self::Pointer(work) => work.root_quota,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct SequencedWork {
    pub(crate) sequence: WorkSequence,
    pub(crate) generation: WorkGeneration,
    pub(crate) root_quota: Option<RootQuotaId>,
}

pub(crate) struct CancellationEnvelope {
    pub(crate) sequence: WorkSequence,
    pub(crate) generation: WorkGeneration,
    pub(crate) identity: TraceWorkIdentity,
    pub(crate) causal_parent: Option<TraceSequence>,
    pub(crate) root_quota: Option<RootQuotaId>,
}

pub(crate) struct FrameworkServiceResponseEnvelope {
    pub(crate) sequence: WorkSequence,
    pub(crate) generation: WorkGeneration,
    pub(crate) response: FrameworkServiceResponse,
    pub(crate) trace_identity: TraceWorkIdentity,
    pub(crate) causal_parent: Option<TraceSequence>,
    pub(crate) root_quota: Option<RootQuotaId>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum QueueCommitError {
    Full,
    SequenceExhausted,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum QueueReservationError {
    Capacity,
    WorkCapacity(crate::work::WorkFamily),
    Closed,
    Foreign,
    Stale,
    Exhausted,
    SequenceExhausted,
}

#[derive(Clone, Copy)]
pub(crate) struct RootProducerWorkStarts {
    pub(crate) host_requests: usize,
    pub(crate) local_tasks: usize,
    pub(crate) send_tasks: usize,
    pub(crate) timers: usize,
    pub(crate) repeating_timers: usize,
    pub(crate) framework_services: usize,
}

#[derive(Clone, Copy)]
pub(crate) struct RootProducerCancellations {
    pub(crate) host_requests: usize,
    pub(crate) framework_services: usize,
    pub(crate) local_tasks: usize,
    pub(crate) send_tasks: usize,
    pub(crate) timers: usize,
}

#[derive(Clone, Copy)]
pub(crate) struct RootProducerResponseReleases {
    pub(crate) host_requests: usize,
    pub(crate) framework_services: usize,
}

/// Exact counts needed to preflight one producer's rooted queue commit.
#[derive(Clone, Copy)]
pub(crate) struct RootProducerCommit {
    pub(crate) root: RootQuotaId,
    pub(crate) queued_envelopes: usize,
    pub(crate) starts: RootProducerWorkStarts,
    pub(crate) cancellations: RootProducerCancellations,
    pub(crate) response_releases: RootProducerResponseReleases,
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub(crate) struct RootQuotaId(u64);

/// Unsequenced capacity reserved in the canonical FIFO for later work.
///
/// Reservations hold capacity only. A `WorkSequence` is allocated when an
/// envelope is actually committed through `WorkQueue::push_reserved`.
pub(crate) struct QueueReservation {
    quota: RootQuotaId,
    owner: Arc<()>,
    epoch: Arc<()>,
}

impl QueueReservation {
    pub(crate) const fn quota(&self) -> RootQuotaId {
        self.quota
    }
}

struct RootQuotaState {
    limit: usize,
    remaining: usize,
    input_continuations: usize,
    pending_envelopes: usize,
    active_envelopes: usize,
    live_work_leases: usize,
    host_requests: HashMap<WorkGeneration, HostRequestCredits>,
    framework_services: HashMap<WorkGeneration, FrameworkServiceCredits>,
    local_tasks: HashMap<WorkGeneration, LocalTaskCredits>,
    send_tasks: HashMap<WorkGeneration, LocalTaskCredits>,
    timers: HashMap<WorkGeneration, TimerCredits>,
    timer_callback_actions: HashMap<WorkSequence, WorkGeneration>,
    profile: ProducerBudget,
}

#[derive(Clone, Copy)]
struct HostRequestCredits {
    start_pending: bool,
    response: bool,
    cancellation: bool,
}

#[derive(Clone, Copy)]
struct FrameworkServiceCredits {
    response: bool,
    cancellation: bool,
}

#[derive(Clone, Copy)]
struct LocalTaskCredits {
    start_pending: bool,
    completion: LocalTaskCompletionCredits,
    invalidated: bool,
}

#[derive(Clone, Copy)]
struct LocalTaskCompletionCredits {
    outcome: bool,
    cancellation: bool,
}

#[derive(Clone, Copy)]
struct TimerCredits {
    start_pending: bool,
    activity: TimerActivityCredits,
    repeating: bool,
    invalidated: bool,
}

#[derive(Clone, Copy)]
struct TimerActivityCredits {
    firing: bool,
    callback: bool,
    cancellation: bool,
}

#[derive(Clone, Copy, Default)]
struct PoppedRootCredits {
    host_start: Option<WorkGeneration>,
    local_start: Option<WorkGeneration>,
    send_start: Option<WorkGeneration>,
    timer_start: Option<WorkGeneration>,
    timer_firing: Option<WorkGeneration>,
    timer_callback: Option<WorkGeneration>,
}

#[derive(Clone, Copy)]
enum TimerCreditKind {
    Firing,
    Callback,
    Cancellation,
}

pub(crate) struct CancelledQueue {
    pub(crate) envelopes: usize,
    pub(crate) command_trace_reservations: usize,
    pub(crate) pointer_trace_reservations: usize,
    pub(crate) input_trace_reservations: usize,
}

pub(crate) struct WorkQueue<Action> {
    capacity: usize,
    waiting: VecDeque<WorkEnvelope<Action>>,
    reserved: usize,
    root_quotas: HashMap<RootQuotaId, RootQuotaState>,
    input_continuations: HashMap<InputContinuationKey, RootQuotaId>,
    claimed_input_continuations: HashMap<WorkSequence, (InputContinuationKey, RootQuotaId)>,
    next_root_quota: Option<NonZeroU64>,
    identity: Arc<()>,
    reservation_epoch: Arc<()>,
    closed: bool,
    next_sequence: Option<NonZeroU64>,
}

impl<Action> WorkQueue<Action> {
    pub(crate) fn new(capacity: usize) -> Self {
        Self {
            capacity,
            waiting: VecDeque::new(),
            reserved: 0,
            root_quotas: HashMap::new(),
            input_continuations: HashMap::new(),
            claimed_input_continuations: HashMap::new(),
            next_root_quota: NonZeroU64::new(1),
            identity: Arc::new(()),
            reservation_epoch: Arc::new(()),
            closed: false,
            next_sequence: NonZeroU64::new(1),
        }
    }

    pub(crate) fn is_full(&self) -> bool {
        self.closed
            || self
                .waiting
                .len()
                .checked_add(self.reserved)
                .is_none_or(|occupied| occupied >= self.capacity)
    }

    pub(crate) fn preflight_commit(&self, count: usize) -> Result<(), QueueCommitError> {
        if self.closed {
            return Err(QueueCommitError::Full);
        }
        if self
            .waiting
            .len()
            .checked_add(self.reserved)
            .and_then(|occupied| occupied.checked_add(count))
            .is_none_or(|required| required > self.capacity)
        {
            return Err(QueueCommitError::Full);
        }
        let sequenced = self
            .reserved
            .checked_add(count)
            .ok_or(QueueCommitError::SequenceExhausted)?;
        if sequenced == 0 {
            return Ok(());
        }
        let next = self
            .next_sequence
            .ok_or(QueueCommitError::SequenceExhausted)?;
        let additional =
            u64::try_from(sequenced - 1).map_err(|_| QueueCommitError::SequenceExhausted)?;
        next.get()
            .checked_add(additional)
            .ok_or(QueueCommitError::SequenceExhausted)?;
        Ok(())
    }

    /// Checks a new root without allocating its identity or assigning a sequence.
    pub(crate) fn preflight_reservation(&self, count: usize) -> Result<(), QueueReservationError> {
        if self.closed {
            return Err(QueueReservationError::Closed);
        }
        self.preflight_commit(count).map_err(|error| match error {
            QueueCommitError::Full => QueueReservationError::Capacity,
            QueueCommitError::SequenceExhausted => QueueReservationError::SequenceExhausted,
        })?;
        self.next_root_quota
            .ok_or(QueueReservationError::Exhausted)?;
        Ok(())
    }

    /// Reserves FIFO and sequence capacity without assigning a sequence.
    pub(crate) fn reserve(
        &mut self,
        count: usize,
    ) -> Result<QueueReservation, QueueReservationError> {
        self.preflight_reservation(count)?;
        let quota = RootQuotaId(
            self.next_root_quota
                .ok_or(QueueReservationError::Exhausted)?
                .get(),
        );
        self.next_root_quota = quota.0.checked_add(1).and_then(NonZeroU64::new);
        self.reserved += count;
        self.root_quotas.insert(
            quota,
            RootQuotaState {
                limit: count,
                remaining: count,
                input_continuations: 0,
                pending_envelopes: 0,
                active_envelopes: 0,
                live_work_leases: 0,
                host_requests: HashMap::new(),
                framework_services: HashMap::new(),
                local_tasks: HashMap::new(),
                send_tasks: HashMap::new(),
                timers: HashMap::new(),
                timer_callback_actions: HashMap::new(),
                profile: ProducerBudget::new(count, 0),
            },
        );
        Ok(QueueReservation {
            quota,
            owner: Arc::clone(&self.identity),
            epoch: Arc::clone(&self.reservation_epoch),
        })
    }

    pub(crate) fn reserve_for_producer(
        &mut self,
        profile: ProducerBudget,
    ) -> Result<QueueReservation, QueueReservationError> {
        let reservation = self.reserve(profile.queue_credits())?;
        let quota = self
            .root_quotas
            .get_mut(&reservation.quota)
            .ok_or(QueueReservationError::Stale)?;
        quota.limit = profile.queue_credits();
        quota.remaining = profile.queue_credits();
        quota.profile = profile;
        Ok(reservation)
    }

    /// Resident claims stay with their canonical roots through queued and live work.
    pub(crate) fn work_residency_claims(&self, family: crate::work::WorkFamily) -> Option<usize> {
        self.root_quotas.values().try_fold(0usize, |total, root| {
            total.checked_add(family.residency(root.profile.work_residency()))
        })
    }

    pub(crate) fn root_work_residency(
        &self,
        root: RootQuotaId,
        family: crate::work::WorkFamily,
    ) -> Option<usize> {
        self.root_quotas
            .get(&root)
            .map(|quota| family.residency(quota.profile.work_residency()))
    }

    /// Commits one envelope against a prior reservation.
    pub(crate) fn push_reserved(
        &mut self,
        reservation: &QueueReservation,
        envelope: impl FnOnce(WorkSequence) -> WorkEnvelope<Action>,
    ) -> Result<WorkSequence, QueueReservationError> {
        self.validate_reservation(reservation)?;
        if self.root_remaining(reservation.quota)? == 0 || self.reserved == 0 {
            return Err(QueueReservationError::Exhausted);
        }
        self.preflight_root_sequence(reservation.quota, 1)?;
        let next = self
            .next_sequence
            .ok_or(QueueReservationError::SequenceExhausted)?;
        let sequence = WorkSequence::__runtime_new(next);
        let root = reservation.quota;
        let envelope = envelope(sequence);
        self.note_rooted_enqueued(root, &envelope)?;
        self.next_sequence = sequence.get().checked_add(1).and_then(NonZeroU64::new);
        self.reserved -= 1;
        self.root_quotas
            .get_mut(&root)
            .ok_or(QueueReservationError::Stale)?
            .remaining -= 1;
        self.waiting.push_back(envelope);
        Ok(sequence)
    }

    /// Commits one FIFO envelope as a member of the shared causal-root quota.
    /// The sequence consumes one pre-reserved frontier credit without assigning
    /// a sequence to any unmaterialized credit.
    pub(crate) fn push_reserved_with_remainder(
        &mut self,
        reservation: &QueueReservation,
        envelope: impl FnOnce(WorkSequence, RootQuotaId) -> WorkEnvelope<Action>,
    ) -> Result<WorkSequence, QueueReservationError> {
        self.validate_reservation(reservation)?;
        self.push_rooted(reservation.quota, envelope)
    }

    pub(crate) fn push_rooted(
        &mut self,
        root: RootQuotaId,
        envelope: impl FnOnce(WorkSequence, RootQuotaId) -> WorkEnvelope<Action>,
    ) -> Result<WorkSequence, QueueReservationError> {
        if self.closed || self.root_remaining(root)? == 0 || self.reserved == 0 {
            return Err(QueueReservationError::Exhausted);
        }
        self.preflight_root_sequence(root, 1)?;
        let next = self
            .next_sequence
            .ok_or(QueueReservationError::SequenceExhausted)?;
        let sequence = WorkSequence::__runtime_new(next);
        let mut envelope = envelope(sequence, root);
        Self::bind_root(&mut envelope, root)?;
        self.note_rooted_enqueued(root, &envelope)?;
        self.next_sequence = sequence.get().checked_add(1).and_then(NonZeroU64::new);
        self.reserved -= 1;
        self.root_quotas
            .get_mut(&root)
            .ok_or(QueueReservationError::Stale)?
            .remaining -= 1;
        self.waiting.push_back(envelope);
        Ok(sequence)
    }

    pub(crate) fn preflight_root_commit(
        &self,
        root: RootQuotaId,
        count: usize,
    ) -> Result<(), QueueReservationError> {
        if self.closed {
            return Err(QueueReservationError::Closed);
        }
        if self.root_remaining(root)? < count {
            return Err(QueueReservationError::Exhausted);
        }
        if count != 0 {
            self.preflight_root_sequence(root, count)?;
        }
        Ok(())
    }

    /// A root may spend its own reusable frontier, but must leave every other
    /// root's unsequenced credits available. No sequence is assigned here.
    pub(crate) fn preflight_root_sequence(
        &self,
        root: RootQuotaId,
        count: usize,
    ) -> Result<(), QueueReservationError> {
        if self.closed {
            return Err(QueueReservationError::Closed);
        }
        let quota = self
            .root_quotas
            .get(&root)
            .ok_or(QueueReservationError::Stale)?;
        if count == 0 {
            return Ok(());
        }
        let owned = future_credit_count(quota)
            .and_then(|future| quota.remaining.checked_add(future))
            .and_then(|owned| owned.checked_sub(quota.input_continuations))
            .ok_or(QueueReservationError::Stale)?;
        let required = self
            .reserved
            .checked_sub(owned)
            .and_then(|other_roots| other_roots.checked_add(count))
            .ok_or(QueueReservationError::Stale)?;
        self.preflight_sequence_capacity(required)
    }

    pub(crate) fn preflight_root_producer_commit(
        &self,
        commit: RootProducerCommit,
    ) -> Result<(), QueueReservationError> {
        let RootProducerCommit {
            root,
            queued_envelopes,
            starts,
            cancellations,
            response_releases,
        } = commit;
        // Current outputs use this root's reusable frontier while independent
        // roots retain their previously reserved sequence capacity.
        if queued_envelopes != 0 {
            self.preflight_root_sequence(root, queued_envelopes)?;
        }
        let cancellation_count = cancellations
            .host_requests
            .checked_add(cancellations.framework_services)
            .and_then(|count| count.checked_add(cancellations.local_tasks))
            .and_then(|count| count.checked_add(cancellations.send_tasks))
            .and_then(|count| count.checked_add(cancellations.timers))
            .ok_or(QueueReservationError::Exhausted)?;
        let current_root_outputs = queued_envelopes
            .checked_sub(cancellation_count)
            .ok_or(QueueReservationError::Stale)?;
        let required = current_root_outputs
            .checked_add(starts.host_requests)
            .and_then(|count| count.checked_add(starts.local_tasks))
            .and_then(|count| count.checked_add(starts.send_tasks))
            .and_then(|count| count.checked_add(starts.timers))
            .and_then(|count| count.checked_add(starts.repeating_timers))
            .and_then(|count| count.checked_add(starts.framework_services.checked_mul(2)?))
            .ok_or(QueueReservationError::Exhausted)?;
        let available = self
            .root_remaining(root)?
            .checked_add(response_releases.host_requests)
            .and_then(|count| count.checked_add(response_releases.framework_services))
            .ok_or(QueueReservationError::Exhausted)?;
        if self.closed {
            return Err(QueueReservationError::Closed);
        }
        if available < required {
            return Err(QueueReservationError::Exhausted);
        }
        Ok(())
    }

    fn preflight_sequence_capacity(&self, count: usize) -> Result<(), QueueReservationError> {
        if count == 0 {
            return Ok(());
        }
        let next = self
            .next_sequence
            .ok_or(QueueReservationError::SequenceExhausted)?;
        let additional =
            u64::try_from(count - 1).map_err(|_| QueueReservationError::SequenceExhausted)?;
        next.get()
            .checked_add(additional)
            .ok_or(QueueReservationError::SequenceExhausted)?;
        Ok(())
    }

    pub(crate) fn preflight_host_request_cancellations(
        &self,
        generations: &[WorkGeneration],
        root: RootQuotaId,
    ) -> Result<(usize, usize), QueueReservationError> {
        generations
            .iter()
            .try_fold((0usize, 0usize), |(count, released), generation| {
                let matching = self.root_quotas.iter().find_map(|(owner, quota)| {
                    quota
                        .host_requests
                        .get(generation)
                        .map(|credits| (*owner, credits))
                });
                match matching {
                    Some((owner, credits)) if credits.cancellation => Ok((
                        count
                            .checked_add(1)
                            .ok_or(QueueReservationError::Exhausted)?,
                        released
                            .checked_add(usize::from(owner == root && credits.response))
                            .ok_or(QueueReservationError::Exhausted)?,
                    )),
                    Some(_) => Err(QueueReservationError::Stale),
                    None => Ok((count, released)),
                }
            })
    }

    pub(crate) fn preflight_local_task_cancellations(
        &self,
        generations: &[WorkGeneration],
    ) -> Result<usize, QueueReservationError> {
        generations.iter().try_fold(0usize, |count, generation| {
            let matching = self
                .root_quotas
                .values()
                .find_map(|quota| quota.local_tasks.get(generation));
            match matching {
                Some(credits)
                    if !credits.invalidated
                        && (credits.completion.outcome || credits.completion.cancellation) =>
                {
                    count.checked_add(1).ok_or(QueueReservationError::Exhausted)
                }
                Some(_) => Err(QueueReservationError::Stale),
                None => Ok(count),
            }
        })
    }

    pub(crate) fn preflight_framework_service_cancellations(
        &self,
        generations: &[WorkGeneration],
        root: RootQuotaId,
    ) -> Result<(usize, usize), QueueReservationError> {
        generations
            .iter()
            .try_fold((0usize, 0usize), |(count, released), generation| {
                let matching = self.root_quotas.iter().find_map(|(owner, quota)| {
                    quota
                        .framework_services
                        .get(generation)
                        .map(|credits| (*owner, credits))
                });
                match matching {
                    Some((owner, credits)) if credits.cancellation => Ok((
                        count
                            .checked_add(1)
                            .ok_or(QueueReservationError::Exhausted)?,
                        released
                            .checked_add(usize::from(owner == root && credits.response))
                            .ok_or(QueueReservationError::Exhausted)?,
                    )),
                    Some(_) => Err(QueueReservationError::Stale),
                    None => Ok((count, released)),
                }
            })
    }

    pub(crate) fn preflight_send_task_cancellations(
        &self,
        generations: &[WorkGeneration],
    ) -> Result<usize, QueueReservationError> {
        generations.iter().try_fold(0usize, |count, generation| {
            let matching = self
                .root_quotas
                .values()
                .find_map(|quota| quota.send_tasks.get(generation));
            match matching {
                Some(credits)
                    if !credits.invalidated
                        && (credits.completion.outcome || credits.completion.cancellation) =>
                {
                    count.checked_add(1).ok_or(QueueReservationError::Exhausted)
                }
                Some(_) => Err(QueueReservationError::Stale),
                None => Ok(count),
            }
        })
    }

    pub(crate) fn preflight_timer_cancellations(
        &self,
        generations: &[WorkGeneration],
    ) -> Result<usize, QueueReservationError> {
        generations.iter().try_fold(0usize, |count, generation| {
            let matching = self
                .root_quotas
                .values()
                .find_map(|quota| quota.timers.get(generation));
            match matching {
                Some(credits) if !credits.invalidated && credits.activity.cancellation => {
                    count.checked_add(1).ok_or(QueueReservationError::Exhausted)
                }
                Some(_) => Err(QueueReservationError::Stale),
                None => Ok(count),
            }
        })
    }

    pub(crate) fn preflight_root_host_response(
        &self,
        root: RootQuotaId,
        generation: WorkGeneration,
    ) -> Result<(), QueueReservationError> {
        let credits = self
            .root_quotas
            .get(&root)
            .ok_or(QueueReservationError::Stale)?
            .host_requests
            .get(&generation)
            .ok_or(QueueReservationError::Stale)?;
        if credits.response {
            Ok(())
        } else {
            Err(QueueReservationError::Exhausted)
        }
    }

    pub(crate) fn preflight_root_host_cancellation(
        &self,
        root: RootQuotaId,
        generation: WorkGeneration,
    ) -> Result<(), QueueReservationError> {
        let credits = self
            .root_quotas
            .get(&root)
            .ok_or(QueueReservationError::Stale)?
            .host_requests
            .get(&generation)
            .ok_or(QueueReservationError::Stale)?;
        if credits.cancellation {
            Ok(())
        } else {
            Err(QueueReservationError::Exhausted)
        }
    }

    pub(crate) fn preflight_root_framework_service_response(
        &self,
        root: RootQuotaId,
        generation: WorkGeneration,
    ) -> Result<(), QueueReservationError> {
        let credits = self
            .root_quotas
            .get(&root)
            .ok_or(QueueReservationError::Stale)?
            .framework_services
            .get(&generation)
            .ok_or(QueueReservationError::Stale)?;
        if credits.response {
            Ok(())
        } else {
            Err(QueueReservationError::Exhausted)
        }
    }

    pub(crate) fn preflight_root_framework_service_cancellation(
        &self,
        root: RootQuotaId,
        generation: WorkGeneration,
    ) -> Result<(), QueueReservationError> {
        let credits = self
            .root_quotas
            .get(&root)
            .ok_or(QueueReservationError::Stale)?
            .framework_services
            .get(&generation)
            .ok_or(QueueReservationError::Stale)?;
        if credits.cancellation {
            Ok(())
        } else {
            Err(QueueReservationError::Exhausted)
        }
    }

    pub(crate) fn preflight_root_local_task_outcome(
        &self,
        root: RootQuotaId,
        generation: WorkGeneration,
    ) -> Result<(), QueueReservationError> {
        let credits = self
            .root_quotas
            .get(&root)
            .ok_or(QueueReservationError::Stale)?
            .local_tasks
            .get(&generation)
            .ok_or(QueueReservationError::Stale)?;
        if credits.completion.outcome && !credits.invalidated {
            Ok(())
        } else {
            Err(QueueReservationError::Exhausted)
        }
    }

    pub(crate) fn preflight_root_send_task_outcome(
        &self,
        root: RootQuotaId,
        generation: WorkGeneration,
    ) -> Result<(), QueueReservationError> {
        let credits = self
            .root_quotas
            .get(&root)
            .ok_or(QueueReservationError::Stale)?
            .send_tasks
            .get(&generation)
            .ok_or(QueueReservationError::Stale)?;
        if credits.completion.outcome && !credits.invalidated {
            Ok(())
        } else {
            Err(QueueReservationError::Exhausted)
        }
    }

    pub(crate) fn preflight_root_timer_firing(
        &self,
        root: RootQuotaId,
        generation: WorkGeneration,
    ) -> Result<(), QueueReservationError> {
        let credits = self
            .root_quotas
            .get(&root)
            .ok_or(QueueReservationError::Stale)?
            .timers
            .get(&generation)
            .ok_or(QueueReservationError::Stale)?;
        if credits.activity.firing && !credits.invalidated {
            Ok(())
        } else {
            Err(QueueReservationError::Exhausted)
        }
    }

    pub(crate) fn preflight_root_timer_callback(
        &self,
        root: RootQuotaId,
        generation: WorkGeneration,
    ) -> Result<(), QueueReservationError> {
        let credits = self
            .root_quotas
            .get(&root)
            .ok_or(QueueReservationError::Stale)?
            .timers
            .get(&generation)
            .ok_or(QueueReservationError::Stale)?;
        if credits.activity.callback && !credits.invalidated {
            Ok(())
        } else {
            Err(QueueReservationError::Exhausted)
        }
    }

    pub(crate) fn root_covers_action_budget(
        &self,
        root: RootQuotaId,
        budget: ProducerBudget,
    ) -> bool {
        !self.closed
            && budget.queue_credits() > 0
            && self.root_quotas.get(&root).is_some_and(|quota| {
                budget.queue_credits() <= quota.limit && quota.profile.covers(budget)
            })
    }

    pub(crate) fn root_profile(&self, root: RootQuotaId) -> Option<ProducerBudget> {
        self.root_quotas.get(&root).map(|quota| quota.profile)
    }

    pub(crate) fn begin_root_action(
        &mut self,
        reservation: &QueueReservation,
    ) -> Result<RootQuotaId, QueueReservationError> {
        self.validate_reservation(reservation)?;
        self.begin_root_activity(reservation.quota)?;
        Ok(reservation.quota)
    }

    pub(crate) fn begin_root_activity(
        &mut self,
        root: RootQuotaId,
    ) -> Result<(), QueueReservationError> {
        let quota = self
            .root_quotas
            .get_mut(&root)
            .ok_or(QueueReservationError::Stale)?;
        quota.active_envelopes = quota
            .active_envelopes
            .checked_add(1)
            .ok_or(QueueReservationError::Exhausted)?;
        Ok(())
    }

    /// Releases an unused root quota. A root with queued or active application
    /// actions is released only by `finish_root_action`.
    pub(crate) fn release_reservation(
        &mut self,
        reservation: &QueueReservation,
    ) -> Result<(), QueueReservationError> {
        self.validate_reservation(reservation)?;
        let quota = self
            .root_quotas
            .get(&reservation.quota)
            .ok_or(QueueReservationError::Stale)?;
        if quota.pending_envelopes != 0
            || quota.active_envelopes != 0
            || quota.live_work_leases != 0
            || !quota.host_requests.is_empty()
            || !quota.framework_services.is_empty()
            || !quota.local_tasks.is_empty()
            || !quota.send_tasks.is_empty()
        {
            return Err(QueueReservationError::Stale);
        }
        self.close_root(reservation.quota)
    }

    /// Ends direct producer admission. Queued descendants and live work keep
    /// the same root funded; a producer that emitted nothing releases it now.
    pub(crate) fn finish_reservation(
        &mut self,
        reservation: &QueueReservation,
    ) -> Result<(), QueueReservationError> {
        self.validate_reservation(reservation)?;
        self.close_idle_root(reservation.quota)
    }

    fn validate_reservation(
        &self,
        reservation: &QueueReservation,
    ) -> Result<(), QueueReservationError> {
        if !Arc::ptr_eq(&reservation.owner, &self.identity) {
            return Err(QueueReservationError::Foreign);
        }
        if !Arc::ptr_eq(&reservation.epoch, &self.reservation_epoch) || self.closed {
            return Err(QueueReservationError::Stale);
        }
        if !self.root_quotas.contains_key(&reservation.quota) {
            return Err(QueueReservationError::Exhausted);
        }
        Ok(())
    }

    fn root_remaining(&self, root: RootQuotaId) -> Result<usize, QueueReservationError> {
        self.root_quotas
            .get(&root)
            .and_then(|quota| quota.remaining.checked_sub(quota.input_continuations))
            .ok_or(QueueReservationError::Stale)
    }

    fn bind_root(
        envelope: &mut WorkEnvelope<Action>,
        root: RootQuotaId,
    ) -> Result<(), QueueReservationError> {
        match envelope {
            WorkEnvelope::Input(work) => {
                if work.root_quota.replace(root).is_some() {
                    return Err(QueueReservationError::Foreign);
                }
                Ok(())
            }
            WorkEnvelope::ApplicationAction(work) if work.root_quota == root => Ok(()),
            WorkEnvelope::SemanticCommand(work) if work.root_quota == root => Ok(()),
            WorkEnvelope::ApplicationCommand(work) if work.root_quota == root => Ok(()),
            WorkEnvelope::Pointer(work) if work.root_quota == Some(root) => Ok(()),
            WorkEnvelope::EffectStart(work) | WorkEnvelope::TimerFiring(work) => {
                if work.root_quota.replace(root).is_some() {
                    return Err(QueueReservationError::Foreign);
                }
                Ok(())
            }
            WorkEnvelope::WorkCancellation(work) => {
                if work.root_quota.replace(root).is_some() {
                    return Err(QueueReservationError::Foreign);
                }
                Ok(())
            }
            WorkEnvelope::FrameworkServiceResponse(work) => {
                if work.root_quota.replace(root).is_some() {
                    return Err(QueueReservationError::Foreign);
                }
                Ok(())
            }
            WorkEnvelope::MountedSubscriptionReconcile { root_quota, .. } => {
                if root_quota.replace(root).is_some() {
                    return Err(QueueReservationError::Foreign);
                }
                Ok(())
            }
            WorkEnvelope::ApplicationAction(_)
            | WorkEnvelope::SemanticCommand(_)
            | WorkEnvelope::ApplicationCommand(_)
            | WorkEnvelope::Pointer(_) => Err(QueueReservationError::Foreign),
        }
    }

    fn note_rooted_enqueued(
        &mut self,
        root: RootQuotaId,
        envelope: &WorkEnvelope<Action>,
    ) -> Result<(), QueueReservationError> {
        match envelope.root_quota() {
            Some(envelope_root) if envelope_root == root => {
                let quota = self
                    .root_quotas
                    .get_mut(&root)
                    .ok_or(QueueReservationError::Stale)?;
                quota.pending_envelopes = quota
                    .pending_envelopes
                    .checked_add(1)
                    .ok_or(QueueReservationError::Exhausted)?;
            }
            Some(_) => return Err(QueueReservationError::Foreign),
            None => {}
        }
        Ok(())
    }

    pub(crate) fn finish_root_envelope(
        &mut self,
        root: RootQuotaId,
    ) -> Result<(), QueueReservationError> {
        let quota = self
            .root_quotas
            .get_mut(&root)
            .ok_or(QueueReservationError::Stale)?;
        if quota.active_envelopes == 0 {
            return Err(QueueReservationError::Stale);
        }
        quota.active_envelopes -= 1;
        self.close_idle_root(root)
    }

    pub(crate) fn retain_root_work_lease(
        &mut self,
        root: RootQuotaId,
    ) -> Result<(), QueueReservationError> {
        let quota = self
            .root_quotas
            .get_mut(&root)
            .ok_or(QueueReservationError::Stale)?;
        quota.live_work_leases = quota
            .live_work_leases
            .checked_add(1)
            .ok_or(QueueReservationError::Exhausted)?;
        Ok(())
    }

    pub(crate) fn retain_root_host_request_lease(
        &mut self,
        root: RootQuotaId,
        generation: WorkGeneration,
    ) -> Result<(), QueueReservationError> {
        let quota = self
            .root_quotas
            .get_mut(&root)
            .ok_or(QueueReservationError::Stale)?;
        if quota.remaining == 0 || quota.host_requests.contains_key(&generation) {
            return Err(QueueReservationError::Exhausted);
        }
        quota.live_work_leases = quota
            .live_work_leases
            .checked_add(1)
            .ok_or(QueueReservationError::Exhausted)?;
        quota.remaining -= 1;
        quota.host_requests.insert(
            generation,
            HostRequestCredits {
                start_pending: true,
                response: false,
                cancellation: true,
            },
        );
        Ok(())
    }

    pub(crate) fn retain_root_framework_service_lease(
        &mut self,
        root: RootQuotaId,
        generation: WorkGeneration,
    ) -> Result<(), QueueReservationError> {
        let quota = self
            .root_quotas
            .get_mut(&root)
            .ok_or(QueueReservationError::Stale)?;
        if quota.remaining < 2 || quota.framework_services.contains_key(&generation) {
            return Err(QueueReservationError::Exhausted);
        }
        quota.live_work_leases = quota
            .live_work_leases
            .checked_add(1)
            .ok_or(QueueReservationError::Exhausted)?;
        quota.remaining -= 2;
        quota.framework_services.insert(
            generation,
            FrameworkServiceCredits {
                response: true,
                cancellation: true,
            },
        );
        Ok(())
    }

    pub(crate) fn retain_root_local_task_lease(
        &mut self,
        root: RootQuotaId,
        generation: WorkGeneration,
    ) -> Result<(), QueueReservationError> {
        let quota = self
            .root_quotas
            .get_mut(&root)
            .ok_or(QueueReservationError::Stale)?;
        if quota.remaining == 0 || quota.local_tasks.contains_key(&generation) {
            return Err(QueueReservationError::Exhausted);
        }
        quota.live_work_leases = quota
            .live_work_leases
            .checked_add(1)
            .ok_or(QueueReservationError::Exhausted)?;
        quota.remaining -= 1;
        quota.local_tasks.insert(
            generation,
            LocalTaskCredits {
                start_pending: true,
                completion: LocalTaskCompletionCredits {
                    outcome: false,
                    cancellation: true,
                },
                invalidated: false,
            },
        );
        Ok(())
    }

    pub(crate) fn retain_root_send_task_lease(
        &mut self,
        root: RootQuotaId,
        generation: WorkGeneration,
    ) -> Result<(), QueueReservationError> {
        let quota = self
            .root_quotas
            .get_mut(&root)
            .ok_or(QueueReservationError::Stale)?;
        if quota.remaining == 0 || quota.send_tasks.contains_key(&generation) {
            return Err(QueueReservationError::Exhausted);
        }
        quota.live_work_leases = quota
            .live_work_leases
            .checked_add(1)
            .ok_or(QueueReservationError::Exhausted)?;
        quota.remaining -= 1;
        quota.send_tasks.insert(
            generation,
            LocalTaskCredits {
                start_pending: true,
                completion: LocalTaskCompletionCredits {
                    outcome: false,
                    cancellation: true,
                },
                invalidated: false,
            },
        );
        Ok(())
    }

    pub(crate) fn retain_root_timer_lease(
        &mut self,
        root: RootQuotaId,
        generation: WorkGeneration,
        repeating: bool,
    ) -> Result<(), QueueReservationError> {
        let quota = self
            .root_quotas
            .get_mut(&root)
            .ok_or(QueueReservationError::Stale)?;
        let future = 1usize + usize::from(repeating); // cancel, plus repeating callback
        if quota.remaining < future || quota.timers.contains_key(&generation) {
            return Err(QueueReservationError::Exhausted);
        }
        quota.live_work_leases = quota
            .live_work_leases
            .checked_add(1)
            .ok_or(QueueReservationError::Exhausted)?;
        quota.remaining -= future;
        quota.timers.insert(
            generation,
            TimerCredits {
                start_pending: true,
                activity: TimerActivityCredits {
                    firing: false,
                    callback: repeating,
                    cancellation: true,
                },
                repeating,
                invalidated: false,
            },
        );
        Ok(())
    }

    pub(crate) fn release_root_work_lease(
        &mut self,
        root: RootQuotaId,
        generation: WorkGeneration,
    ) -> Result<(), QueueReservationError> {
        let quota = self
            .root_quotas
            .get_mut(&root)
            .ok_or(QueueReservationError::Stale)?;
        if quota.live_work_leases == 0 {
            return Err(QueueReservationError::Stale);
        }

        // A keyed lease transfers its unused future terminal credits back to
        // the same root. Prove the complete transfer before removing the work
        // generation; an invalid ledger must not be partially repaired.
        let released = quota
            .host_requests
            .get(&generation)
            .map(|credits| usize::from(credits.response) + usize::from(credits.cancellation))
            .or_else(|| {
                quota.framework_services.get(&generation).map(|credits| {
                    usize::from(credits.response) + usize::from(credits.cancellation)
                })
            })
            .or_else(|| {
                quota.local_tasks.get(&generation).map(|credits| {
                    usize::from(credits.completion.outcome)
                        + usize::from(credits.completion.cancellation)
                })
            })
            .or_else(|| {
                quota.send_tasks.get(&generation).map(|credits| {
                    usize::from(credits.completion.outcome)
                        + usize::from(credits.completion.cancellation)
                })
            })
            .or_else(|| {
                quota.timers.get(&generation).map(|credits| {
                    usize::from(credits.activity.firing)
                        + usize::from(credits.activity.callback)
                        + usize::from(credits.activity.cancellation)
                })
            });

        if let Some(released) = released {
            let future_after = future_credit_count(quota)
                .and_then(|future| future.checked_sub(released))
                .ok_or(QueueReservationError::Stale)?;
            let remaining = quota
                .remaining
                .checked_add(released)
                .ok_or(QueueReservationError::Exhausted)?;
            if quota
                .pending_envelopes
                .checked_add(remaining)
                .and_then(|occupied| occupied.checked_add(future_after))
                .is_none_or(|occupied| occupied > quota.limit)
            {
                return Err(QueueReservationError::Stale);
            }
            if quota.host_requests.contains_key(&generation) {
                quota.host_requests.remove(&generation);
            } else if quota.framework_services.contains_key(&generation) {
                quota.framework_services.remove(&generation);
            } else if quota.local_tasks.contains_key(&generation) {
                quota.local_tasks.remove(&generation);
            } else if quota.send_tasks.contains_key(&generation) {
                quota.send_tasks.remove(&generation);
            } else {
                quota.timers.remove(&generation);
            }
            quota.remaining = remaining;
        }
        // A non-keyed subscription lease has no per-generation terminal map;
        // its WorkRegistry record remains the authority for revocation.
        quota.live_work_leases -= 1;
        self.close_idle_root(root)
    }

    pub(crate) fn release_root_work_lease_for_cancellation(
        &mut self,
        root: RootQuotaId,
        generation: WorkGeneration,
    ) -> Result<(), QueueReservationError> {
        let Some(quota) = self.root_quotas.get_mut(&root) else {
            return Err(QueueReservationError::Stale);
        };
        if let Some(credits) = quota.host_requests.get_mut(&generation) {
            if quota.live_work_leases == 0 || !credits.cancellation {
                return Err(QueueReservationError::Stale);
            }
            quota.remaining = quota
                .remaining
                .checked_add(usize::from(credits.response))
                .ok_or(QueueReservationError::Exhausted)?;
            credits.response = false;
            quota.live_work_leases -= 1;
            return Ok(());
        }
        if let Some(credits) = quota.framework_services.get_mut(&generation) {
            if quota.live_work_leases == 0 || !credits.cancellation {
                return Err(QueueReservationError::Stale);
            }
            quota.remaining = quota
                .remaining
                .checked_add(usize::from(credits.response))
                .ok_or(QueueReservationError::Exhausted)?;
            credits.response = false;
            quota.live_work_leases -= 1;
            return Ok(());
        }
        if let Some(credits) = quota.local_tasks.get_mut(&generation) {
            if quota.live_work_leases == 0
                || credits.invalidated
                || !(credits.completion.outcome || credits.completion.cancellation)
            {
                return Err(QueueReservationError::Stale);
            }
            credits.invalidated = true;
            quota.live_work_leases -= 1;
            return Ok(());
        }
        if let Some(credits) = quota.send_tasks.get_mut(&generation) {
            if quota.live_work_leases == 0
                || credits.invalidated
                || !(credits.completion.outcome || credits.completion.cancellation)
            {
                return Err(QueueReservationError::Stale);
            }
            credits.invalidated = true;
            quota.live_work_leases -= 1;
            return Ok(());
        }
        if let Some(credits) = quota.timers.get_mut(&generation) {
            if quota.live_work_leases == 0 || credits.invalidated || !credits.activity.cancellation
            {
                return Err(QueueReservationError::Stale);
            }
            let released =
                usize::from(credits.activity.firing) + usize::from(credits.activity.callback);
            quota.remaining = quota
                .remaining
                .checked_add(released)
                .ok_or(QueueReservationError::Exhausted)?;
            credits.activity.firing = false;
            credits.activity.callback = false;
            credits.invalidated = true;
            quota.live_work_leases -= 1;
            return Ok(());
        }
        self.release_root_work_lease(root, generation)
    }

    pub(crate) fn push_host_response_action_rooted(
        &mut self,
        root: RootQuotaId,
        generation: WorkGeneration,
        envelope: impl FnOnce(WorkSequence, RootQuotaId) -> WorkEnvelope<Action>,
    ) -> Result<WorkSequence, QueueReservationError> {
        self.push_rooted_from_host_credit(root, generation, true, envelope)
    }

    pub(crate) fn push_local_task_outcome_action_rooted(
        &mut self,
        root: RootQuotaId,
        generation: WorkGeneration,
        envelope: impl FnOnce(WorkSequence, RootQuotaId) -> WorkEnvelope<Action>,
    ) -> Result<WorkSequence, QueueReservationError> {
        self.push_rooted_from_local_credit(root, generation, false, envelope)
    }

    pub(crate) fn push_send_task_outcome_action_rooted(
        &mut self,
        root: RootQuotaId,
        generation: WorkGeneration,
        envelope: impl FnOnce(WorkSequence, RootQuotaId) -> WorkEnvelope<Action>,
    ) -> Result<WorkSequence, QueueReservationError> {
        self.push_rooted_from_send_task_credit(root, generation, false, envelope)
    }

    pub(crate) fn push_local_task_cancellation(
        &mut self,
        generation: WorkGeneration,
        identity: TraceWorkIdentity,
        causal_parent: Option<TraceSequence>,
    ) -> Result<Option<WorkSequence>, QueueReservationError> {
        let root = self
            .root_quotas
            .iter()
            .find_map(|(root, quota)| quota.local_tasks.contains_key(&generation).then_some(*root));
        let Some(root) = root else {
            return Ok(None);
        };
        self.push_rooted_from_local_credit(root, generation, true, |sequence, _| {
            WorkEnvelope::WorkCancellation(CancellationEnvelope {
                sequence,
                generation,
                identity,
                causal_parent,
                root_quota: None,
            })
        })
        .map(Some)
    }

    pub(crate) fn push_send_task_cancellation(
        &mut self,
        generation: WorkGeneration,
        identity: TraceWorkIdentity,
        causal_parent: Option<TraceSequence>,
    ) -> Result<Option<WorkSequence>, QueueReservationError> {
        let root = self
            .root_quotas
            .iter()
            .find_map(|(root, quota)| quota.send_tasks.contains_key(&generation).then_some(*root));
        let Some(root) = root else {
            return Ok(None);
        };
        self.push_rooted_from_send_task_credit(root, generation, true, |sequence, _| {
            WorkEnvelope::WorkCancellation(CancellationEnvelope {
                sequence,
                generation,
                identity,
                causal_parent,
                root_quota: None,
            })
        })
        .map(Some)
    }

    pub(crate) fn push_timer_cancellation(
        &mut self,
        generation: WorkGeneration,
        identity: TraceWorkIdentity,
        causal_parent: Option<TraceSequence>,
    ) -> Result<Option<WorkSequence>, QueueReservationError> {
        let root = self
            .root_quotas
            .iter()
            .find_map(|(root, quota)| quota.timers.contains_key(&generation).then_some(*root));
        let Some(root) = root else {
            return Ok(None);
        };
        self.push_rooted_from_timer_credit(
            root,
            generation,
            TimerCreditKind::Cancellation,
            |sequence, _| {
                WorkEnvelope::WorkCancellation(CancellationEnvelope {
                    sequence,
                    generation,
                    identity,
                    causal_parent,
                    root_quota: None,
                })
            },
        )
        .map(Some)
    }

    fn push_rooted_from_local_credit(
        &mut self,
        root: RootQuotaId,
        generation: WorkGeneration,
        cancellation: bool,
        envelope: impl FnOnce(WorkSequence, RootQuotaId) -> WorkEnvelope<Action>,
    ) -> Result<WorkSequence, QueueReservationError> {
        if self.closed {
            return Err(QueueReservationError::Closed);
        }
        let credits = self
            .root_quotas
            .get(&root)
            .ok_or(QueueReservationError::Stale)?
            .local_tasks
            .get(&generation)
            .copied()
            .ok_or(QueueReservationError::Stale)?;
        if if cancellation {
            !credits.invalidated || !(credits.completion.outcome || credits.completion.cancellation)
        } else {
            credits.invalidated || !credits.completion.outcome
        } {
            return Err(QueueReservationError::Exhausted);
        }
        if self.reserved == 0 {
            return Err(QueueReservationError::Stale);
        }
        self.preflight_root_sequence(root, 1)?;
        let next = self
            .next_sequence
            .ok_or(QueueReservationError::SequenceExhausted)?;
        let sequence = WorkSequence::__runtime_new(next);
        let mut envelope = envelope(sequence, root);
        Self::bind_root(&mut envelope, root)?;
        self.note_rooted_enqueued(root, &envelope)?;
        let quota = self
            .root_quotas
            .get_mut(&root)
            .ok_or(QueueReservationError::Stale)?;
        let credits = quota
            .local_tasks
            .get_mut(&generation)
            .ok_or(QueueReservationError::Stale)?;
        credits.completion.outcome = false;
        if cancellation {
            credits.completion.cancellation = false;
            if !credits.start_pending {
                quota.local_tasks.remove(&generation);
            }
        }
        self.next_sequence = sequence.get().checked_add(1).and_then(NonZeroU64::new);
        self.reserved -= 1;
        self.waiting.push_back(envelope);
        Ok(sequence)
    }

    fn push_rooted_from_send_task_credit(
        &mut self,
        root: RootQuotaId,
        generation: WorkGeneration,
        cancellation: bool,
        envelope: impl FnOnce(WorkSequence, RootQuotaId) -> WorkEnvelope<Action>,
    ) -> Result<WorkSequence, QueueReservationError> {
        if self.closed {
            return Err(QueueReservationError::Closed);
        }
        let credits = self
            .root_quotas
            .get(&root)
            .ok_or(QueueReservationError::Stale)?
            .send_tasks
            .get(&generation)
            .copied()
            .ok_or(QueueReservationError::Stale)?;
        if if cancellation {
            !credits.invalidated || !(credits.completion.outcome || credits.completion.cancellation)
        } else {
            credits.invalidated || !credits.completion.outcome
        } {
            return Err(QueueReservationError::Exhausted);
        }
        if self.reserved == 0 {
            return Err(QueueReservationError::Stale);
        }
        self.preflight_root_sequence(root, 1)?;
        let next = self
            .next_sequence
            .ok_or(QueueReservationError::SequenceExhausted)?;
        let sequence = WorkSequence::__runtime_new(next);
        let mut envelope = envelope(sequence, root);
        Self::bind_root(&mut envelope, root)?;
        self.note_rooted_enqueued(root, &envelope)?;
        let quota = self
            .root_quotas
            .get_mut(&root)
            .ok_or(QueueReservationError::Stale)?;
        let credits = quota
            .send_tasks
            .get_mut(&generation)
            .ok_or(QueueReservationError::Stale)?;
        credits.completion.outcome = false;
        if cancellation {
            credits.completion.cancellation = false;
            if !credits.start_pending {
                quota.send_tasks.remove(&generation);
            }
        }
        self.next_sequence = sequence.get().checked_add(1).and_then(NonZeroU64::new);
        self.reserved -= 1;
        self.waiting.push_back(envelope);
        Ok(sequence)
    }

    pub(crate) fn push_host_cancellation_rooted(
        &mut self,
        root: RootQuotaId,
        generation: WorkGeneration,
        identity: TraceWorkIdentity,
        causal_parent: Option<TraceSequence>,
    ) -> Result<WorkSequence, QueueReservationError> {
        self.push_rooted_from_host_credit(root, generation, false, |sequence, _| {
            WorkEnvelope::WorkCancellation(CancellationEnvelope {
                sequence,
                generation,
                identity,
                causal_parent,
                root_quota: None,
            })
        })
    }

    pub(crate) fn push_host_request_cancellation(
        &mut self,
        generation: WorkGeneration,
        identity: TraceWorkIdentity,
        causal_parent: Option<TraceSequence>,
    ) -> Result<Option<WorkSequence>, QueueReservationError> {
        let root = self.root_quotas.iter().find_map(|(root, quota)| {
            quota
                .host_requests
                .contains_key(&generation)
                .then_some(*root)
        });
        let Some(root) = root else {
            return Ok(None);
        };
        let sequence =
            self.push_host_cancellation_rooted(root, generation, identity, causal_parent)?;
        self.root_quotas
            .get_mut(&root)
            .ok_or(QueueReservationError::Stale)?
            .host_requests
            .remove(&generation);
        Ok(Some(sequence))
    }

    fn push_rooted_from_host_credit(
        &mut self,
        root: RootQuotaId,
        generation: WorkGeneration,
        response: bool,
        envelope: impl FnOnce(WorkSequence, RootQuotaId) -> WorkEnvelope<Action>,
    ) -> Result<WorkSequence, QueueReservationError> {
        if self.closed {
            return Err(QueueReservationError::Closed);
        }
        let credits = self
            .root_quotas
            .get(&root)
            .ok_or(QueueReservationError::Stale)?
            .host_requests
            .get(&generation)
            .copied()
            .ok_or(QueueReservationError::Stale)?;
        if if response {
            !credits.response
        } else {
            !credits.cancellation
        } {
            return Err(QueueReservationError::Exhausted);
        }
        if self.reserved == 0 {
            return Err(QueueReservationError::Stale);
        }
        self.preflight_root_sequence(root, 1)?;
        let next = self
            .next_sequence
            .ok_or(QueueReservationError::SequenceExhausted)?;
        let sequence = WorkSequence::__runtime_new(next);
        let mut envelope = envelope(sequence, root);
        Self::bind_root(&mut envelope, root)?;
        self.note_rooted_enqueued(root, &envelope)?;
        let quota = self
            .root_quotas
            .get_mut(&root)
            .ok_or(QueueReservationError::Stale)?;
        let credits = quota
            .host_requests
            .get_mut(&generation)
            .ok_or(QueueReservationError::Stale)?;
        if response {
            credits.response = false;
        } else {
            credits.cancellation = false;
        }
        self.next_sequence = sequence.get().checked_add(1).and_then(NonZeroU64::new);
        self.reserved -= 1;
        self.waiting.push_back(envelope);
        Ok(sequence)
    }

    fn push_rooted_from_timer_credit(
        &mut self,
        root: RootQuotaId,
        generation: WorkGeneration,
        kind: TimerCreditKind,
        envelope: impl FnOnce(WorkSequence, RootQuotaId) -> WorkEnvelope<Action>,
    ) -> Result<WorkSequence, QueueReservationError> {
        if self.closed {
            return Err(QueueReservationError::Closed);
        }
        let credits = self
            .root_quotas
            .get(&root)
            .ok_or(QueueReservationError::Stale)?
            .timers
            .get(&generation)
            .copied()
            .ok_or(QueueReservationError::Stale)?;
        let available = match kind {
            TimerCreditKind::Firing => credits.activity.firing && !credits.invalidated,
            TimerCreditKind::Callback => credits.activity.callback && !credits.invalidated,
            TimerCreditKind::Cancellation => credits.activity.cancellation && credits.invalidated,
        };
        if !available {
            return Err(QueueReservationError::Exhausted);
        }
        if self.reserved == 0 {
            return Err(QueueReservationError::Stale);
        }
        self.preflight_root_sequence(root, 1)?;
        let next = self
            .next_sequence
            .ok_or(QueueReservationError::SequenceExhausted)?;
        let sequence = WorkSequence::__runtime_new(next);
        let mut envelope = envelope(sequence, root);
        Self::bind_root(&mut envelope, root)?;
        if matches!(kind, TimerCreditKind::Callback)
            && !matches!(&envelope, WorkEnvelope::ApplicationAction(_))
        {
            return Err(QueueReservationError::Stale);
        }
        self.note_rooted_enqueued(root, &envelope)?;
        {
            let quota = self
                .root_quotas
                .get_mut(&root)
                .ok_or(QueueReservationError::Stale)?;
            let credits = quota
                .timers
                .get_mut(&generation)
                .ok_or(QueueReservationError::Stale)?;
            match kind {
                TimerCreditKind::Firing => credits.activity.firing = false,
                TimerCreditKind::Callback => credits.activity.callback = false,
                TimerCreditKind::Cancellation => {
                    credits.activity.cancellation = false;
                    quota.timers.remove(&generation);
                }
            }
        }
        self.next_sequence = sequence.get().checked_add(1).and_then(NonZeroU64::new);
        self.reserved -= 1;
        if matches!(kind, TimerCreditKind::Callback) {
            self.root_quotas
                .get_mut(&root)
                .ok_or(QueueReservationError::Stale)?
                .timer_callback_actions
                .insert(sequence, generation);
        }
        self.waiting.push_back(envelope);
        Ok(sequence)
    }

    pub(crate) fn preflight_root_work_leases(
        &self,
        root: RootQuotaId,
        count: usize,
    ) -> Result<(), QueueReservationError> {
        self.root_quotas
            .get(&root)
            .ok_or(QueueReservationError::Stale)?
            .live_work_leases
            .checked_add(count)
            .ok_or(QueueReservationError::Exhausted)?;
        Ok(())
    }

    pub(crate) fn finish_root_action(
        &mut self,
        root: RootQuotaId,
    ) -> Result<(), QueueReservationError> {
        self.finish_root_envelope(root)
    }

    fn close_idle_root(&mut self, root: RootQuotaId) -> Result<(), QueueReservationError> {
        let quota = self
            .root_quotas
            .get(&root)
            .ok_or(QueueReservationError::Stale)?;
        if quota.pending_envelopes == 0
            && quota.active_envelopes == 0
            && quota.input_continuations == 0
            && quota.live_work_leases == 0
            && quota.host_requests.is_empty()
            && quota.framework_services.is_empty()
            && quota.local_tasks.is_empty()
            && quota.send_tasks.is_empty()
            && quota.timers.is_empty()
            && quota.timer_callback_actions.is_empty()
        {
            self.close_root(root)
        } else {
            Ok(())
        }
    }

    fn close_root(&mut self, root: RootQuotaId) -> Result<(), QueueReservationError> {
        let quota = self
            .root_quotas
            .get(&root)
            .ok_or(QueueReservationError::Stale)?;
        if quota.pending_envelopes != 0
            || quota.active_envelopes != 0
            || quota.input_continuations != 0
            || quota.live_work_leases != 0
            || !quota.host_requests.is_empty()
            || !quota.framework_services.is_empty()
            || !quota.local_tasks.is_empty()
            || !quota.send_tasks.is_empty()
            || !quota.timers.is_empty()
            || !quota.timer_callback_actions.is_empty()
        {
            return Err(QueueReservationError::Stale);
        }
        if self.reserved < quota.remaining {
            return Err(QueueReservationError::Stale);
        }
        self.reserved -= quota.remaining;
        self.root_quotas.remove(&root);
        Ok(())
    }

    pub(crate) const fn has_sequence(&self) -> bool {
        self.next_sequence.is_some()
    }

    pub(crate) fn next_sequence(&self) -> Option<WorkSequence> {
        self.next_sequence.map(WorkSequence::__runtime_new)
    }

    #[cfg(test)]
    pub(crate) fn push_preflighted(
        &mut self,
        action: Action,
        causal_parent: Option<TraceSequence>,
        target: Option<TraceTarget>,
        origin: ApplicationActionOrigin,
    ) -> Result<WorkSequence, Action> {
        let Ok(reservation) = self.reserve(1) else {
            return Err(action);
        };
        self.push_preflighted_with_reservation(
            action,
            causal_parent,
            target,
            origin,
            ProducerBudget::new(1, 0),
            &reservation,
        )
    }

    pub(crate) fn push_preflighted_with_reservation(
        &mut self,
        action: Action,
        causal_parent: Option<TraceSequence>,
        target: Option<TraceTarget>,
        origin: ApplicationActionOrigin,
        producer_budget: ProducerBudget,
        reservation: &QueueReservation,
    ) -> Result<WorkSequence, Action> {
        let mut action = Some(action);
        self.push_reserved_with_remainder(reservation, |sequence, root_quota| {
            WorkEnvelope::ApplicationAction(ApplicationActionEnvelope {
                sequence,
                action: action.take().unwrap_or_else(|| unreachable!()),
                producer_budget,
                root_quota,
                causal_parent,
                target,
                origin,
            })
        })
        .map_err(|_| action.take().unwrap_or_else(|| unreachable!()))
    }

    pub(crate) fn push_effect_start(
        &mut self,
        generation: WorkGeneration,
    ) -> Result<WorkSequence, QueueCommitError> {
        self.push_control(|sequence| {
            WorkEnvelope::EffectStart(SequencedWork {
                sequence,
                generation,
                root_quota: None,
            })
        })
    }

    pub(crate) fn push_effect_start_rooted(
        &mut self,
        generation: WorkGeneration,
        root: RootQuotaId,
    ) -> Result<WorkSequence, QueueReservationError> {
        self.push_rooted(root, |sequence, _| {
            WorkEnvelope::EffectStart(SequencedWork {
                sequence,
                generation,
                root_quota: None,
            })
        })
    }

    pub(crate) fn push_command_preflighted(
        &mut self,
        payload: SemanticCommandQueuePayload,
        reservation: &QueueReservation,
    ) -> Result<WorkSequence, QueueReservationError> {
        let SemanticCommandQueuePayload {
            queued_target,
            command,
            origin,
            focus_request_override,
            presentation_cancel_origin,
            instant,
            causal_parent,
            trace_reservation,
            root_quota: _,
        } = payload;
        let SemanticCommandQueueTarget {
            target,
            semantic_target,
        } = queued_target;
        self.push_reserved_with_remainder(reservation, |sequence, root_quota| {
            WorkEnvelope::SemanticCommand(SemanticCommandEnvelope {
                sequence,
                target,
                command,
                origin,
                semantic_target,
                focus_request_override,
                presentation_cancel_origin,
                instant,
                causal_parent,
                trace_reservation,
                root_quota,
            })
        })
    }

    pub(crate) fn push_command_rooted(
        &mut self,
        payload: SemanticCommandQueuePayload,
    ) -> Result<WorkSequence, QueueReservationError> {
        let SemanticCommandQueuePayload {
            queued_target,
            command,
            origin,
            focus_request_override,
            presentation_cancel_origin,
            instant,
            causal_parent,
            trace_reservation,
            root_quota,
        } = payload;
        let SemanticCommandQueueTarget {
            target,
            semantic_target,
        } = queued_target;
        self.push_rooted(root_quota, |sequence, root_quota| {
            WorkEnvelope::SemanticCommand(SemanticCommandEnvelope {
                sequence,
                target,
                command,
                origin,
                semantic_target,
                focus_request_override,
                presentation_cancel_origin,
                instant,
                causal_parent,
                trace_reservation,
                root_quota,
            })
        })
    }

    pub(crate) fn push_application_command_preflighted(
        &mut self,
        reservation: &QueueReservation,
        payload: ApplicationCommandQueuePayload,
    ) -> Result<WorkSequence, QueueReservationError> {
        let ApplicationCommandQueuePayload {
            target,
            command,
            origin,
            instant,
            causal_parent,
            trace_reservation,
        } = payload;
        self.push_reserved_with_remainder(reservation, |sequence, root_quota| {
            WorkEnvelope::ApplicationCommand(ApplicationCommandEnvelope {
                sequence,
                target,
                command,
                origin,
                instant,
                causal_parent,
                trace_reservation,
                root_quota,
            })
        })
    }

    pub(crate) fn push_application_command_rooted(
        &mut self,
        root_quota: RootQuotaId,
        payload: ApplicationCommandQueuePayload,
    ) -> Result<WorkSequence, QueueReservationError> {
        let ApplicationCommandQueuePayload {
            target,
            command,
            origin,
            instant,
            causal_parent,
            trace_reservation,
        } = payload;
        self.push_rooted(root_quota, |sequence, root_quota| {
            WorkEnvelope::ApplicationCommand(ApplicationCommandEnvelope {
                sequence,
                target,
                command,
                origin,
                instant,
                causal_parent,
                trace_reservation,
                root_quota,
            })
        })
    }

    pub(crate) fn push_pointer_preflighted(
        &mut self,
        reservation: &QueueReservation,
        event: PointerEvent,
        instant: MonotonicInstant,
        causal_parent: Option<TraceSequence>,
        trace_reservation: TraceReservation,
    ) -> Result<WorkSequence, QueueReservationError> {
        self.push_reserved_with_remainder(reservation, |sequence, root| {
            WorkEnvelope::Pointer(PointerEnvelope {
                sequence,
                root_quota: Some(root),
                payload: PointerEnvelopePayload::Event(event),
                instant,
                causal_parent,
                trace_reservation,
            })
        })
    }

    pub(crate) fn push_pointer_rehit_rooted(
        &mut self,
        root: RootQuotaId,
        context: SurfaceInputContext,
        instant: MonotonicInstant,
        causal_parent: Option<TraceSequence>,
        trace_reservation: TraceReservation,
    ) -> Result<WorkSequence, QueueReservationError> {
        self.push_rooted(root, |sequence, root| {
            WorkEnvelope::Pointer(PointerEnvelope {
                sequence,
                root_quota: Some(root),
                payload: PointerEnvelopePayload::StationaryRehit(context),
                instant,
                causal_parent,
                trace_reservation,
            })
        })
    }

    pub(crate) fn push_composition_preflighted(
        &mut self,
        reservation: &QueueReservation,
        target: MountedNodeId,
        event: CompositionEvent,
        instant: MonotonicInstant,
        causal_parent: Option<TraceSequence>,
        trace_reservation: TraceReservation,
    ) -> Result<WorkSequence, QueueReservationError> {
        self.push_reserved_with_remainder(reservation, |sequence, _| {
            WorkEnvelope::Input(InputEnvelope {
                sequence,
                target,
                focus_binding: None,
                text_revoked: false,
                root_quota: None,
                payload: InputEnvelopePayload::Composition(event),
                instant,
                causal_parent,
                trace_reservation,
            })
        })
    }

    pub(crate) fn push_cancellation(
        &mut self,
        generation: WorkGeneration,
        identity: TraceWorkIdentity,
        causal_parent: Option<TraceSequence>,
    ) -> Result<WorkSequence, QueueCommitError> {
        self.push_control(|sequence| {
            WorkEnvelope::WorkCancellation(CancellationEnvelope {
                sequence,
                generation,
                identity,
                causal_parent,
                root_quota: None,
            })
        })
    }

    pub(crate) fn push_cancellation_rooted(
        &mut self,
        generation: WorkGeneration,
        identity: TraceWorkIdentity,
        causal_parent: Option<TraceSequence>,
        root: RootQuotaId,
    ) -> Result<WorkSequence, QueueReservationError> {
        self.push_rooted(root, |sequence, _| {
            WorkEnvelope::WorkCancellation(CancellationEnvelope {
                sequence,
                generation,
                identity,
                causal_parent,
                root_quota: None,
            })
        })
    }

    pub(crate) fn push_timer_firing(
        &mut self,
        generation: WorkGeneration,
    ) -> Result<WorkSequence, QueueCommitError> {
        self.push_control(|sequence| {
            WorkEnvelope::TimerFiring(SequencedWork {
                sequence,
                generation,
                root_quota: None,
            })
        })
    }

    pub(crate) fn push_timer_firing_rooted(
        &mut self,
        generation: WorkGeneration,
        root: RootQuotaId,
    ) -> Result<WorkSequence, QueueReservationError> {
        self.push_rooted_from_timer_credit(
            root,
            generation,
            TimerCreditKind::Firing,
            |sequence, _| {
                WorkEnvelope::TimerFiring(SequencedWork {
                    sequence,
                    generation,
                    root_quota: None,
                })
            },
        )
    }

    pub(crate) fn push_timer_callback_action_rooted(
        &mut self,
        root: RootQuotaId,
        generation: WorkGeneration,
        envelope: impl FnOnce(WorkSequence, RootQuotaId) -> WorkEnvelope<Action>,
    ) -> Result<WorkSequence, QueueReservationError> {
        self.push_rooted_from_timer_credit(root, generation, TimerCreditKind::Callback, envelope)
    }

    pub(crate) fn push_framework_service_response(
        &mut self,
        generation: WorkGeneration,
        response: FrameworkServiceResponse,
        trace_identity: TraceWorkIdentity,
        causal_parent: Option<TraceSequence>,
    ) -> Result<WorkSequence, QueueCommitError> {
        self.push_control(|sequence| {
            WorkEnvelope::FrameworkServiceResponse(FrameworkServiceResponseEnvelope {
                sequence,
                generation,
                response,
                trace_identity,
                causal_parent,
                root_quota: None,
            })
        })
    }

    pub(crate) fn push_framework_service_response_rooted(
        &mut self,
        generation: WorkGeneration,
        response: FrameworkServiceResponse,
        trace_identity: TraceWorkIdentity,
        causal_parent: Option<TraceSequence>,
        root: RootQuotaId,
    ) -> Result<WorkSequence, QueueReservationError> {
        self.push_rooted_from_framework_service_credit(root, generation, true, |sequence, _| {
            WorkEnvelope::FrameworkServiceResponse(FrameworkServiceResponseEnvelope {
                sequence,
                generation,
                response,
                trace_identity,
                causal_parent,
                root_quota: None,
            })
        })
    }

    pub(crate) fn push_framework_service_cancellation_rooted(
        &mut self,
        root: RootQuotaId,
        generation: WorkGeneration,
        identity: TraceWorkIdentity,
        causal_parent: Option<TraceSequence>,
    ) -> Result<WorkSequence, QueueReservationError> {
        self.push_rooted_from_framework_service_credit(root, generation, false, |sequence, _| {
            WorkEnvelope::WorkCancellation(CancellationEnvelope {
                sequence,
                generation,
                identity,
                causal_parent,
                root_quota: None,
            })
        })
    }

    pub(crate) fn push_framework_service_request_cancellation(
        &mut self,
        generation: WorkGeneration,
        identity: TraceWorkIdentity,
        causal_parent: Option<TraceSequence>,
    ) -> Result<Option<WorkSequence>, QueueReservationError> {
        let root = self.root_quotas.iter().find_map(|(root, quota)| {
            quota
                .framework_services
                .contains_key(&generation)
                .then_some(*root)
        });
        let Some(root) = root else {
            return Ok(None);
        };
        let sequence = self.push_framework_service_cancellation_rooted(
            root,
            generation,
            identity,
            causal_parent,
        )?;
        self.root_quotas
            .get_mut(&root)
            .ok_or(QueueReservationError::Stale)?
            .framework_services
            .remove(&generation);
        Ok(Some(sequence))
    }

    fn push_rooted_from_framework_service_credit(
        &mut self,
        root: RootQuotaId,
        generation: WorkGeneration,
        response: bool,
        envelope: impl FnOnce(WorkSequence, RootQuotaId) -> WorkEnvelope<Action>,
    ) -> Result<WorkSequence, QueueReservationError> {
        if self.closed {
            return Err(QueueReservationError::Closed);
        }
        let credits = self
            .root_quotas
            .get(&root)
            .ok_or(QueueReservationError::Stale)?
            .framework_services
            .get(&generation)
            .copied()
            .ok_or(QueueReservationError::Stale)?;
        if if response {
            !credits.response
        } else {
            !credits.cancellation
        } {
            return Err(QueueReservationError::Exhausted);
        }
        if self.reserved == 0 {
            return Err(QueueReservationError::Stale);
        }
        self.preflight_root_sequence(root, 1)?;
        let next = self
            .next_sequence
            .ok_or(QueueReservationError::SequenceExhausted)?;
        let mut envelope = envelope(WorkSequence::__runtime_new(next), root);
        Self::bind_root(&mut envelope, root)?;
        self.note_rooted_enqueued(root, &envelope)?;
        let quota = self
            .root_quotas
            .get_mut(&root)
            .ok_or(QueueReservationError::Stale)?;
        let credits = quota
            .framework_services
            .get_mut(&generation)
            .ok_or(QueueReservationError::Stale)?;
        if response {
            credits.response = false;
        } else {
            credits.cancellation = false;
        }
        self.next_sequence = next.get().checked_add(1).and_then(NonZeroU64::new);
        self.reserved -= 1;
        self.waiting.push_back(envelope);
        Ok(WorkSequence::__runtime_new(next))
    }

    pub(crate) fn push_mounted_subscription_reconcile(
        &mut self,
        owner: MountedNodeId,
        causal_parent: Option<TraceSequence>,
    ) -> Result<WorkSequence, QueueCommitError> {
        self.push_control(|sequence| WorkEnvelope::MountedSubscriptionReconcile {
            sequence,
            owner,
            causal_parent,
            root_quota: None,
        })
    }

    pub(crate) fn push_mounted_subscription_reconcile_rooted(
        &mut self,
        owner: MountedNodeId,
        causal_parent: Option<TraceSequence>,
        root: RootQuotaId,
    ) -> Result<WorkSequence, QueueReservationError> {
        self.push_rooted(root, |sequence, _| {
            WorkEnvelope::MountedSubscriptionReconcile {
                sequence,
                owner,
                causal_parent,
                root_quota: None,
            }
        })
    }

    fn push_control(
        &mut self,
        envelope: impl FnOnce(WorkSequence) -> WorkEnvelope<Action>,
    ) -> Result<WorkSequence, QueueCommitError> {
        let reservation = self.reserve(1).map_err(|error| match error {
            QueueReservationError::Capacity | QueueReservationError::Closed => {
                QueueCommitError::Full
            }
            QueueReservationError::SequenceExhausted => QueueCommitError::SequenceExhausted,
            QueueReservationError::Foreign
            | QueueReservationError::Stale
            | QueueReservationError::WorkCapacity(_)
            | QueueReservationError::Exhausted => {
                unreachable!("fresh queue reservation has valid ownership")
            }
        })?;
        let result = self
            .push_reserved(&reservation, envelope)
            .map_err(|error| match error {
                QueueReservationError::Capacity | QueueReservationError::Closed => {
                    QueueCommitError::Full
                }
                QueueReservationError::SequenceExhausted => QueueCommitError::SequenceExhausted,
                QueueReservationError::Foreign
                | QueueReservationError::Stale
                | QueueReservationError::WorkCapacity(_)
                | QueueReservationError::Exhausted => {
                    unreachable!("fresh queue reservation remains valid")
                }
            });
        if result.is_ok() {
            self.release_reservation(&reservation)
                .unwrap_or_else(|_| unreachable!("control append root remains valid"));
        }
        result
    }

    fn classify_popped_root_credits(
        quota: &RootQuotaState,
        envelope: &WorkEnvelope<Action>,
    ) -> PoppedRootCredits {
        let WorkEnvelope::EffectStart(start) = envelope else {
            let timer_firing = match envelope {
                WorkEnvelope::TimerFiring(firing)
                    if quota.timers.get(&firing.generation).is_some_and(|credits| {
                        !credits.invalidated && !credits.activity.firing
                    }) =>
                {
                    Some(firing.generation)
                }
                _ => None,
            };
            return PoppedRootCredits {
                timer_firing,
                timer_callback: quota
                    .timer_callback_actions
                    .get(&envelope.sequence())
                    .copied()
                    .filter(|generation| {
                        quota.timers.get(generation).is_some_and(|credits| {
                            credits.repeating && !credits.invalidated && !credits.activity.callback
                        })
                    }),
                ..PoppedRootCredits::default()
            };
        };
        PoppedRootCredits {
            host_start: quota
                .host_requests
                .get(&start.generation)
                .is_some_and(|credits| credits.start_pending)
                .then_some(start.generation),
            local_start: quota
                .local_tasks
                .get(&start.generation)
                .is_some_and(|credits| credits.start_pending)
                .then_some(start.generation),
            send_start: quota
                .send_tasks
                .get(&start.generation)
                .is_some_and(|credits| credits.start_pending)
                .then_some(start.generation),
            timer_start: quota
                .timers
                .get(&start.generation)
                .is_some_and(|credits| credits.start_pending)
                .then_some(start.generation),
            ..PoppedRootCredits::default()
        }
    }

    fn commit_popped_root_credits(
        &mut self,
        root: RootQuotaId,
        envelope: &WorkEnvelope<Action>,
        credits: PoppedRootCredits,
    ) -> bool {
        let Some(quota) = self.root_quotas.get(&root) else {
            return false;
        };
        let host_start = credits.host_start.is_some();
        let timer_start = credits.timer_start.is_some_and(|generation| {
            quota
                .timers
                .get(&generation)
                .is_some_and(|credits| !credits.invalidated)
        });
        let timer_firing = credits.timer_firing.is_some();
        let timer_callback = credits.timer_callback.is_some();
        let protect_future_credit = host_start || timer_start || timer_firing || timer_callback;
        let remaining = if protect_future_credit {
            Some(quota.remaining)
        } else {
            quota.remaining.checked_add(1)
        };
        let pending = quota.pending_envelopes.checked_sub(1);
        let future = future_credit_count(quota)
            .and_then(|count| count.checked_add(usize::from(host_start)))
            .and_then(|count| count.checked_add(usize::from(timer_start)))
            .and_then(|count| count.checked_add(usize::from(timer_firing)))
            .and_then(|count| count.checked_add(usize::from(timer_callback)));
        let Some((reserved, remaining, active, pending, future)) = self
            .reserved
            .checked_add(1)
            .zip(remaining)
            .zip(quota.active_envelopes.checked_add(1))
            .zip(pending)
            .zip(future)
            .map(|((((reserved, remaining), active), pending), future)| {
                (reserved, remaining, active, pending, future)
            })
        else {
            return false;
        };
        if pending
            .checked_add(remaining)
            .and_then(|occupied| occupied.checked_add(future))
            .is_none_or(|occupied| occupied > quota.limit)
        {
            return false;
        }

        self.reserved = reserved;
        let quota = self
            .root_quotas
            .get_mut(&root)
            .unwrap_or_else(|| unreachable!("popped root quota was preflighted"));
        quota.remaining = remaining;
        quota.active_envelopes = active;
        quota.pending_envelopes = pending;
        Self::activate_popped_root_credits(quota, credits);
        quota.timer_callback_actions.remove(&envelope.sequence());
        true
    }

    fn activate_popped_root_credits(quota: &mut RootQuotaState, credits: PoppedRootCredits) {
        if let Some(generation) = credits.host_start {
            let work = quota
                .host_requests
                .get_mut(&generation)
                .unwrap_or_else(|| unreachable!("host start credit was preflighted"));
            work.start_pending = false;
            work.response = true;
        }
        Self::activate_popped_local_task(quota, credits.local_start, false);
        Self::activate_popped_local_task(quota, credits.send_start, true);
        if let Some(generation) = credits.timer_start {
            let timer = quota
                .timers
                .get_mut(&generation)
                .unwrap_or_else(|| unreachable!("timer start credit was preflighted"));
            timer.start_pending = false;
            if !timer.invalidated {
                timer.activity.firing = true;
            }
        }
        if let Some(generation) = credits.timer_firing {
            let timer = quota
                .timers
                .get_mut(&generation)
                .unwrap_or_else(|| unreachable!("timer firing credit was preflighted"));
            if timer.repeating {
                timer.activity.firing = true;
            } else {
                timer.activity.callback = true;
            }
        }
        if let Some(generation) = credits.timer_callback {
            quota
                .timers
                .get_mut(&generation)
                .unwrap_or_else(|| unreachable!("timer callback credit was preflighted"))
                .activity
                .callback = true;
        }
    }

    fn activate_popped_local_task(
        quota: &mut RootQuotaState,
        generation: Option<WorkGeneration>,
        send: bool,
    ) {
        let Some(generation) = generation else {
            return;
        };
        let tasks = if send {
            &mut quota.send_tasks
        } else {
            &mut quota.local_tasks
        };
        let work = tasks
            .get_mut(&generation)
            .unwrap_or_else(|| unreachable!("task start credit was preflighted"));
        work.start_pending = false;
        if work.invalidated {
            tasks.remove(&generation);
        } else {
            work.completion.outcome = true;
            work.completion.cancellation = false;
        }
    }

    pub(crate) fn pop(&mut self) -> Option<WorkEnvelope<Action>> {
        let envelope = self.waiting.pop_front()?;
        if let Some(root) = envelope.root_quota() {
            let valid = if self.closed {
                false
            } else if let Some(quota) = self.root_quotas.get(&root) {
                let credits = Self::classify_popped_root_credits(quota, &envelope);
                self.commit_popped_root_credits(root, &envelope, credits)
                    && self
                        .rearm_reached_input_continuation(envelope.sequence())
                        .is_ok()
            } else {
                false
            };
            self.closed |= !valid;
        }
        Some(envelope)
    }

    pub(crate) fn len(&self) -> usize {
        self.waiting.len()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.waiting.is_empty()
    }

    /// Existing envelopes and retained causal roots can still run arbitrary
    /// producer code before a newly submitted interaction reaches its route.
    pub(crate) fn has_unfinished_producers(&self) -> bool {
        !self.waiting.is_empty() || !self.root_quotas.is_empty()
    }

    pub(crate) fn has_keyboard_input_before(&self, deadline: MonotonicInstant) -> bool {
        self.waiting.iter().any(|envelope| {
            matches!(
                envelope,
                WorkEnvelope::Input(InputEnvelope {
                    payload: InputEnvelopePayload::Keyboard(_),
                    instant,
                    ..
                }) if *instant < deadline
            )
        })
    }

    pub(crate) fn has_pointer_envelopes(&self) -> bool {
        self.waiting
            .iter()
            .any(|envelope| matches!(envelope, WorkEnvelope::Pointer(_)))
    }

    pub(crate) fn queued_pointer_ids(&self) -> impl Iterator<Item = runenui_core::PointerId> + '_ {
        self.waiting.iter().filter_map(|envelope| match envelope {
            WorkEnvelope::Pointer(PointerEnvelope {
                payload: PointerEnvelopePayload::Event(event),
                ..
            }) => Some(event.pointer_id()),
            _ => None,
        })
    }

    pub(crate) fn invalidate_queued_text_for(&mut self, owner: &MountedNodeId) {
        for item in &mut self.waiting {
            if let WorkEnvelope::Input(envelope) = item
                && envelope.target == *owner
                && matches!(envelope.payload, InputEnvelopePayload::CommittedText(_))
            {
                envelope.text_revoked = true;
            }
        }
    }

    pub(crate) fn pending_presentation_entry(
        &self,
    ) -> (
        usize,
        Option<(WorkSequence, MountedNodeId, PresentationFocusEntry)>,
    ) {
        let mut count = 0;
        let mut candidate = None;
        for envelope in &self.waiting {
            if let WorkEnvelope::SemanticCommand(command) = envelope
                && let Some(override_focus) = command.focus_request_override.as_ref()
                && let (Some(target), Some(entry)) =
                    (&override_focus.target, &override_focus.presentation_entry)
            {
                count += 1;
                candidate = Some((command.sequence, target.clone(), entry.clone()));
            }
        }
        (count, candidate)
    }

    pub(crate) fn pending_committed_text_for(
        &self,
        owner: &MountedNodeId,
        session: Option<FocusSession>,
    ) -> usize {
        self.waiting
            .iter()
            .filter(|envelope| {
                matches!(
                    envelope,
                    WorkEnvelope::Input(InputEnvelope {
                        target,
                        focus_binding: Some(FocusInputBinding::Committed(bound)),
                        text_revoked: false,
                        payload: InputEnvelopePayload::CommittedText(_),
                        ..
                    }) if target == owner && Some(*bound) == session
                )
            })
            .count()
    }

    pub(crate) fn cancel_all(&mut self) -> CancelledQueue {
        let command_trace_reservations = self
            .waiting
            .iter()
            .filter(|envelope| {
                matches!(
                    envelope,
                    WorkEnvelope::SemanticCommand(command)
                        if command.trace_reservation.is_active()
                ) || matches!(
                    envelope,
                    WorkEnvelope::ApplicationCommand(command)
                        if command.trace_reservation.is_active()
                )
            })
            .count();
        let pointer_trace_reservations = self
            .waiting
            .iter()
            .filter(|envelope| {
                matches!(
                    envelope,
                    WorkEnvelope::Pointer(pointer) if pointer.trace_reservation.is_active()
                )
            })
            .count();
        let input_trace_reservations = self.waiting.iter().filter(|envelope| {
            matches!(envelope, WorkEnvelope::Input(input) if input.trace_reservation.is_active())
        }).count();
        let envelopes = self.waiting.len();
        self.waiting.clear();
        self.reserved = 0;
        self.root_quotas.clear();
        self.input_continuations.clear();
        self.claimed_input_continuations.clear();
        self.reservation_epoch = Arc::new(());
        self.closed = true;
        CancelledQueue {
            envelopes,
            command_trace_reservations,
            pointer_trace_reservations,
            input_trace_reservations,
        }
    }

    #[cfg(feature = "internal-test-seams")]
    pub(crate) const fn seed_next_sequence_for_test(&mut self, next: u64) {
        self.next_sequence = NonZeroU64::new(next);
    }
}

fn future_credit_count(quota: &RootQuotaState) -> Option<usize> {
    let host = quota
        .host_requests
        .values()
        .try_fold(0usize, |count, credits| {
            count
                .checked_add(usize::from(credits.response))?
                .checked_add(usize::from(credits.cancellation))
        })?;
    let framework = quota
        .framework_services
        .values()
        .try_fold(host, |count, credits| {
            count
                .checked_add(usize::from(credits.response))?
                .checked_add(usize::from(credits.cancellation))
        })?;
    let local = quota
        .local_tasks
        .values()
        .try_fold(framework, |count, credits| {
            count
                .checked_add(usize::from(credits.completion.outcome))?
                .checked_add(usize::from(credits.completion.cancellation))
        })?;
    let send = quota
        .send_tasks
        .values()
        .try_fold(local, |count, credits| {
            count
                .checked_add(usize::from(credits.completion.outcome))?
                .checked_add(usize::from(credits.completion.cancellation))
        })?;
    quota.timers.values().try_fold(send, |count, credits| {
        count
            .checked_add(usize::from(credits.activity.firing))?
            .checked_add(usize::from(credits.activity.callback))?
            .checked_add(usize::from(credits.activity.cancellation))
    })
}

#[cfg(test)]
mod reservation_tests {
    #![allow(
        clippy::expect_used,
        clippy::panic,
        reason = "Queue unit tests assert exact invariant failure messages at their fixtures"
    )]

    use super::{
        ApplicationActionEnvelope, ApplicationActionOrigin, ProducerBudget, QueueCommitError,
        QueueReservationError, RootProducerCancellations, RootProducerCommit,
        RootProducerResponseReleases, RootProducerWorkStarts, RootQuotaId, WorkEnvelope, WorkQueue,
        WorkSequence,
    };

    fn action_envelope(
        sequence: WorkSequence,
        action: u8,
        root_quota: RootQuotaId,
    ) -> WorkEnvelope<u8> {
        WorkEnvelope::ApplicationAction(ApplicationActionEnvelope {
            sequence,
            action,
            producer_budget: ProducerBudget::new(1, 0),
            root_quota,
            causal_parent: None,
            target: None,
            origin: ApplicationActionOrigin::Ordinary,
        })
    }

    #[cfg(feature = "internal-test-seams")]
    fn root_commit(
        root: RootQuotaId,
        queued_envelopes: usize,
        local_cancellations: usize,
    ) -> RootProducerCommit {
        RootProducerCommit {
            root,
            queued_envelopes,
            starts: RootProducerWorkStarts {
                host_requests: 0,
                local_tasks: 0,
                send_tasks: 0,
                timers: 0,
                repeating_timers: 0,
                framework_services: 0,
            },
            cancellations: RootProducerCancellations {
                host_requests: 0,
                framework_services: 0,
                local_tasks: local_cancellations,
                send_tasks: 0,
                timers: 0,
            },
            response_releases: RootProducerResponseReleases {
                host_requests: 0,
                framework_services: 0,
            },
        }
    }

    #[cfg(feature = "internal-test-seams")]
    #[test]
    fn quota_violation_during_lease_release_keeps_all_credits_and_generation_intact() {
        use crate::{RuntimeLimits, work::WorkRegistry};

        let mut queue = WorkQueue::<u8>::new(1);
        let reservation = queue.reserve(1).expect("one credit");
        let generation =
            WorkRegistry::<u8, runenui_core::NoHostProtocol>::new(RuntimeLimits::default())
                .preview_generations(1)
                .expect("one generation")
                .0[0];
        queue
            .retain_root_local_task_lease(reservation.quota(), generation)
            .expect("lease owns its terminal credit");
        queue
            .root_quotas
            .get_mut(&reservation.quota())
            .expect("root remains live")
            .pending_envelopes = 1;

        assert_eq!(
            queue.release_root_work_lease(reservation.quota(), generation),
            Err(QueueReservationError::Stale),
            "a malformed occupied frontier cannot consume work ownership",
        );
        let quota = queue
            .root_quotas
            .get(&reservation.quota())
            .expect("failed release must retain root");
        assert_eq!(quota.remaining, 0);
        assert_eq!(quota.pending_envelopes, 1);
        assert_eq!(quota.live_work_leases, 1);
        assert!(quota.local_tasks.contains_key(&generation));
        assert_eq!(queue.reserved, 1);
        assert_eq!(queue.next_sequence().map(WorkSequence::get), Some(1));
    }

    #[cfg(feature = "internal-test-seams")]
    #[test]
    fn stale_work_lease_release_preserves_generation_credits_and_root_capacity() {
        use crate::{RuntimeLimits, work::WorkRegistry};

        let mut queue = WorkQueue::<u8>::new(1);
        let reservation = queue.reserve(1).expect("one owned terminal credit fits");
        let generation =
            WorkRegistry::<u8, runenui_core::NoHostProtocol>::new(RuntimeLimits::default())
                .preview_generations(1)
                .expect("one work generation fits")
                .0[0];
        queue
            .retain_root_local_task_lease(reservation.quota(), generation)
            .expect("the live local task owns its terminal credit");

        // Deliberately corrupt only the lease count to test an internal stale
        // release. A refused operation must not corrupt the independent credits.
        queue
            .root_quotas
            .get_mut(&reservation.quota())
            .expect("root exists")
            .live_work_leases = 0;
        assert_eq!(
            queue.release_root_work_lease(reservation.quota(), generation),
            Err(QueueReservationError::Stale),
        );
        let quota = queue
            .root_quotas
            .get(&reservation.quota())
            .expect("stale release leaves the root unchanged");
        assert_eq!(quota.remaining, 0);
        assert!(quota.local_tasks.contains_key(&generation));
        assert_eq!(quota.live_work_leases, 0);
        assert_eq!(queue.reserved, 1);
        assert_eq!(queue.next_sequence().map(WorkSequence::get), Some(1));
    }

    #[test]
    fn foreign_reserved_envelope_refusal_preserves_sequence_and_both_root_credits() {
        let mut queue = WorkQueue::<u8>::new(2);
        let first = queue.reserve(1).expect("first root fits");
        let second = queue.reserve(1).expect("second root fits");

        assert_eq!(
            queue.push_reserved(&first, |sequence| {
                action_envelope(sequence, 7, second.quota())
            }),
            Err(QueueReservationError::Foreign),
        );
        assert_eq!(queue.next_sequence().map(WorkSequence::get), Some(1));
        assert_eq!(queue.root_remaining(first.quota()), Ok(1));
        assert_eq!(queue.root_remaining(second.quota()), Ok(1));
        assert_eq!(queue.len(), 0);

        assert_eq!(
            queue
                .push_reserved_with_remainder(&first, |sequence, root| {
                    action_envelope(sequence, 8, root)
                })
                .map(WorkSequence::get),
            Ok(1),
        );
        assert_eq!(
            queue
                .push_reserved_with_remainder(&second, |sequence, root| {
                    action_envelope(sequence, 9, root)
                })
                .map(WorkSequence::get),
            Ok(2),
        );
    }

    #[test]
    fn reservations_and_waiting_envelopes_share_one_capacity_bound() {
        let mut queue = WorkQueue::<u8>::new(3);
        let reserved = queue.reserve(2).expect("two slots fit");

        assert_eq!(queue.len(), 0);
        assert!(!queue.is_full());
        assert_eq!(queue.preflight_commit(2), Err(QueueCommitError::Full));
        assert_eq!(queue.next_sequence().map(WorkSequence::get), Some(1));

        queue
            .push_preflighted(10, None, None, ApplicationActionOrigin::Ordinary)
            .expect("unreserved slot remains available");
        assert!(queue.is_full());
        assert_eq!(queue.next_sequence().map(WorkSequence::get), Some(2));

        assert_eq!(
            queue
                .push_reserved_with_remainder(&reserved, |seq, root| action_envelope(seq, 11, root))
                .map(WorkSequence::get),
            Ok(2)
        );
        assert_eq!(queue.len(), 2);
        assert_eq!(queue.next_sequence().map(WorkSequence::get), Some(3));
        assert!(queue.is_full());

        assert_eq!(
            queue
                .push_reserved_with_remainder(&reserved, |seq, root| action_envelope(seq, 12, root))
                .map(WorkSequence::get),
            Ok(3)
        );
        assert_eq!(queue.len(), 3);
        assert_eq!(queue.next_sequence().map(WorkSequence::get), Some(4));
        assert!(queue.is_full());
        assert_eq!(queue.preflight_commit(1), Err(QueueCommitError::Full));
    }

    #[test]
    fn releasing_unused_reservation_returns_capacity_without_consuming_sequence() {
        let mut queue = WorkQueue::<u8>::new(1);
        let reserved = queue.reserve(1).expect("one slot fits");
        assert!(queue.is_full());

        queue
            .release_reservation(&reserved)
            .expect("reservation releases once");
        assert_eq!(
            queue.release_reservation(&reserved),
            Err(QueueReservationError::Exhausted)
        );

        assert!(!queue.is_full());
        assert_eq!(queue.len(), 0);
        assert_eq!(queue.next_sequence().map(WorkSequence::get), Some(1));
        assert_eq!(queue.preflight_commit(1), Ok(()));
    }

    #[cfg(feature = "internal-test-seams")]
    #[test]
    fn sequence_capacity_is_reserved_without_early_sequence_assignment() {
        let mut queue = WorkQueue::<u8>::new(4);
        queue.seed_next_sequence_for_test(u64::MAX - 1);
        let reserved = queue.reserve(2).expect("two sequence values fit");

        assert_eq!(
            queue.next_sequence().map(WorkSequence::get),
            Some(u64::MAX - 1)
        );
        assert_eq!(
            queue.preflight_commit(1),
            Err(QueueCommitError::SequenceExhausted)
        );
        assert_eq!(
            queue.push_preflighted(7, None, None, ApplicationActionOrigin::Ordinary),
            Err(7),
            "an ordinary append cannot consume a reserved sequence"
        );
        assert_eq!(
            queue
                .push_reserved_with_remainder(&reserved, |seq, root| action_envelope(seq, 1, root))
                .map(WorkSequence::get),
            Ok(u64::MAX - 1)
        );
        assert_eq!(
            queue
                .push_reserved_with_remainder(&reserved, |seq, root| action_envelope(seq, 2, root))
                .map(WorkSequence::get),
            Ok(u64::MAX)
        );
        assert_eq!(queue.next_sequence(), None);
    }

    #[cfg(feature = "internal-test-seams")]
    #[test]
    fn recycled_root_frontier_preserves_another_roots_reserved_sequence() {
        let mut queue = WorkQueue::<u8>::new(2);
        queue.seed_next_sequence_for_test(u64::MAX - 2);

        // B owns one future envelope credit. It is already included in
        // WorkQueue::reserved when the independent unary A root is admitted.
        let future_b = queue
            .reserve_for_producer(ProducerBudget::new(1, 0))
            .expect("B's finite future envelope fits");
        let unary_a = queue
            .reserve_for_producer(ProducerBudget::new(1, 0))
            .expect("A's first envelope and B's future credit fit the sequence frontier");
        assert_eq!(
            queue
                .push_reserved_with_remainder(&unary_a, |sequence, root| {
                    action_envelope(sequence, 1, root)
                })
                .map(WorkSequence::get),
            Ok(u64::MAX - 2)
        );

        let Some(WorkEnvelope::ApplicationAction(first_a)) = queue.pop() else {
            panic!("A's initial action is the FIFO head");
        };
        assert_eq!(queue.root_remaining(future_b.quota()), Ok(1));
        assert_eq!(queue.root_remaining(first_a.root_quota), Ok(1));
        queue
            .preflight_root_producer_commit(root_commit(first_a.root_quota, 1, 0))
            .expect("A's first unary renewal sees one next sequence");
        assert_eq!(
            queue
                .push_rooted(first_a.root_quota, |sequence, root| {
                    action_envelope(sequence, 2, root)
                })
                .map(WorkSequence::get),
            Ok(u64::MAX - 1)
        );
        queue
            .finish_root_action(first_a.root_quota)
            .expect("A's first action transfers to its queued child");

        let Some(WorkEnvelope::ApplicationAction(second_a)) = queue.pop() else {
            panic!("A's first renewal is the FIFO head");
        };
        assert_eq!(queue.root_remaining(future_b.quota()), Ok(1));
        assert_eq!(queue.root_remaining(second_a.root_quota), Ok(1));
        assert_eq!(
            queue.preflight_root_producer_commit(root_commit(second_a.root_quota, 1, 0),),
            Err(QueueReservationError::SequenceExhausted),
            "A must fail before consuming B's remaining sequence tail"
        );
        assert_eq!(
            queue.push_rooted(second_a.root_quota, |sequence, root| {
                action_envelope(sequence, 3, root)
            }),
            Err(QueueReservationError::SequenceExhausted),
            "the append boundary independently preserves B's sequence"
        );
        assert_eq!(queue.next_sequence().map(WorkSequence::get), Some(u64::MAX));
        assert_eq!(queue.root_remaining(future_b.quota()), Ok(1));
        queue
            .finish_root_action(second_a.root_quota)
            .expect("A's rejected callback retires and releases its unused frontier");
        assert_eq!(
            queue.reserved, 1,
            "B's exact future credit remains reserved"
        );
        assert_eq!(
            queue
                .push_rooted(future_b.quota(), |sequence, root| {
                    action_envelope(sequence, 4, root)
                })
                .map(WorkSequence::get),
            Ok(u64::MAX)
        );
        assert_eq!(queue.next_sequence(), None);
        assert_eq!(queue.reserved, 0);
    }

    #[cfg(feature = "internal-test-seams")]
    #[test]
    fn zero_output_root_can_retire_without_spending_another_roots_last_sequence() {
        let mut queue = WorkQueue::<u8>::new(2);
        queue.seed_next_sequence_for_test(u64::MAX - 1);
        let future_b = queue
            .reserve_for_producer(ProducerBudget::new(1, 0))
            .expect("B's future envelope fits");
        let a = queue
            .reserve_for_producer(ProducerBudget::new(1, 0))
            .expect("A's admitted envelope fits beside B");
        queue
            .push_reserved_with_remainder(&a, |sequence, root| action_envelope(sequence, 1, root))
            .expect("A receives the penultimate sequence");
        let Some(WorkEnvelope::ApplicationAction(action)) = queue.pop() else {
            panic!("A is the FIFO head");
        };
        queue
            .preflight_root_producer_commit(root_commit(action.root_quota, 0, 0))
            .expect("an empty output batch allocates no new sequence");
        queue
            .finish_root_action(action.root_quota)
            .expect("A retires its unused recycled credit");
        assert_eq!(queue.reserved, 1);
        assert_eq!(queue.next_sequence().map(WorkSequence::get), Some(u64::MAX));
        assert_eq!(
            queue
                .push_rooted(future_b.quota(), |sequence, root| {
                    action_envelope(sequence, 2, root)
                })
                .map(WorkSequence::get),
            Ok(u64::MAX),
            "B retains the last sequence after A's output-free completion"
        );
    }

    #[cfg(feature = "internal-test-seams")]
    #[test]
    fn cross_root_cancellation_credit_remains_sequenced_after_recycled_root_refusal() {
        use crate::{
            RuntimeLimits, TraceWorkFamily, TraceWorkIdentity, TraceWorkOwner, work::WorkRegistry,
        };

        let mut queue = WorkQueue::<u8>::new(3);
        queue.seed_next_sequence_for_test(u64::MAX - 3);
        let b = queue
            .reserve_for_producer(ProducerBudget::new(2, 0))
            .expect("B reserves its current root and future terminal credits");
        let generation =
            WorkRegistry::<u8, runenui_core::NoHostProtocol>::new(RuntimeLimits::default())
                .preview_generations(1)
                .expect("one work generation fits")
                .0[0];
        queue
            .push_reserved_with_remainder(&b, |sequence, root| action_envelope(sequence, 10, root))
            .expect("B's current action receives the first sequence");
        queue
            .retain_root_local_task_lease(b.quota(), generation)
            .expect("B retains its exact task cancellation credit");
        let Some(WorkEnvelope::ApplicationAction(b_action)) = queue.pop() else {
            panic!("B's current action is the FIFO head");
        };
        queue
            .finish_root_action(b_action.root_quota)
            .expect("B action settles while its local-task terminal remains live");

        let a = queue
            .reserve_for_producer(ProducerBudget::new(1, 0))
            .expect("A's first envelope fits beside B's reserved terminal");
        assert_eq!(
            queue
                .push_reserved_with_remainder(&a, |sequence, root| {
                    action_envelope(sequence, 20, root)
                })
                .map(WorkSequence::get),
            Ok(u64::MAX - 2)
        );
        let Some(WorkEnvelope::ApplicationAction(a_action)) = queue.pop() else {
            panic!("A's current action is the FIFO head");
        };
        assert_eq!(queue.root_remaining(b.quota()), Ok(1));
        assert_eq!(queue.root_remaining(a_action.root_quota), Ok(1));

        assert_eq!(
            queue.preflight_root_producer_commit(root_commit(a_action.root_quota, 2, 1),),
            Err(QueueReservationError::SequenceExhausted),
            "A's output cannot spend B's local-task cancellation sequence"
        );
        queue
            .finish_root_action(a_action.root_quota)
            .expect("rejected A callback retires its unused root credit");
        queue
            .release_root_work_lease_for_cancellation(b.quota(), generation)
            .expect("B's exact local-task lease is invalidated for cancellation");
        let identity = TraceWorkIdentity::new(
            TraceWorkOwner::Application,
            TraceWorkFamily::LocalTask,
            generation.get(),
            None,
        );
        assert_eq!(
            queue
                .push_local_task_cancellation(generation, identity, None)
                .map(|sequence| sequence.map(WorkSequence::get)),
            Ok(Some(u64::MAX - 1)),
            "B can still materialize its exact cancellation at the preserved sequence"
        );
        assert_eq!(
            queue
                .push_rooted(b.quota(), |sequence, root| action_envelope(
                    sequence, 11, root
                ))
                .map(WorkSequence::get),
            Ok(u64::MAX),
            "B's other terminal credit also remains usable"
        );
        assert_eq!(queue.next_sequence(), None);
        assert_eq!(queue.reserved, 0);
        let Some(WorkEnvelope::WorkCancellation(cancellation)) = queue.pop() else {
            panic!("B's exact cancellation is the FIFO head");
        };
        assert_eq!(cancellation.generation, generation);
        assert_eq!(cancellation.sequence.get(), u64::MAX - 1);
        queue
            .finish_root_action(
                cancellation
                    .root_quota
                    .expect("cancellation retains B's root"),
            )
            .expect("B's cancellation settles before its later rooted action");
        let Some(WorkEnvelope::ApplicationAction(terminal_action)) = queue.pop() else {
            panic!("B's rooted terminal action remains in FIFO");
        };
        assert_eq!(terminal_action.sequence.get(), u64::MAX);
        queue
            .finish_root_action(terminal_action.root_quota)
            .expect("B's final root action closes its quota");
    }

    #[test]
    fn foreign_reservation_cannot_be_consumed_or_released_by_another_queue() {
        let mut owner = WorkQueue::<u8>::new(1);
        let mut foreign = WorkQueue::<u8>::new(1);
        let reservation = owner.reserve(1).expect("one slot fits");

        assert_eq!(
            foreign.push_reserved_with_remainder(&reservation, |seq, root| action_envelope(
                seq, 1, root
            )),
            Err(QueueReservationError::Foreign)
        );
        assert_eq!(
            foreign.release_reservation(&reservation),
            Err(QueueReservationError::Foreign)
        );
        assert!(owner.is_full());
        assert!(!foreign.is_full());
        assert_eq!(
            owner
                .push_reserved_with_remainder(&reservation, |seq, root| action_envelope(
                    seq, 1, root
                ))
                .map(WorkSequence::get),
            Ok(1)
        );
    }

    #[test]
    fn consumed_reservation_cannot_be_reused_or_released_twice() {
        let mut queue = WorkQueue::<u8>::new(2);
        let reservation = queue.reserve(1).expect("one slot fits");
        queue
            .push_reserved_with_remainder(&reservation, |seq, root| action_envelope(seq, 1, root))
            .expect("reserved commit succeeds");

        assert_eq!(
            queue.push_reserved_with_remainder(&reservation, |seq, root| action_envelope(
                seq, 2, root
            )),
            Err(QueueReservationError::Exhausted)
        );
        assert_eq!(
            queue.release_reservation(&reservation),
            Err(QueueReservationError::Stale)
        );
        let Some(WorkEnvelope::ApplicationAction(work)) = queue.pop() else {
            panic!("accepted action remains queued");
        };
        queue
            .finish_root_action(work.root_quota)
            .expect("the final action releases its exact root quota");
        assert_eq!(
            queue.release_reservation(&reservation),
            Err(QueueReservationError::Exhausted)
        );
        assert_eq!(queue.len(), 0);
        assert_eq!(queue.next_sequence().map(WorkSequence::get), Some(2));
    }

    #[test]
    fn causal_root_lease_reuses_popped_credit_for_a_unary_chain() {
        let mut queue = WorkQueue::<u8>::new(1);
        let root = queue.reserve(1).expect("one frontier credit fits");
        queue
            .push_reserved_with_remainder(&root, |seq, lease| action_envelope(seq, 1, lease))
            .expect("first action uses the root credit");
        assert!(queue.is_full());
        assert_eq!(queue.next_sequence().map(WorkSequence::get), Some(2));

        for action in 2..=64 {
            let Some(WorkEnvelope::ApplicationAction(work)) = queue.pop() else {
                panic!("the causal action remains at the FIFO head");
            };
            assert_eq!(work.action, action - 1);
            assert_eq!(queue.reserved, 1, "pop transfers Q to root P");
            queue
                .push_rooted(work.root_quota, |seq, root| {
                    action_envelope(seq, action, root)
                })
                .expect("unary output reuses the popped frontier credit");
            queue
                .finish_root_action(work.root_quota)
                .expect("processing action transfers to the queued child");
            assert_eq!(queue.len(), 1);
            assert_eq!(queue.reserved, 0);
        }
        let Some(WorkEnvelope::ApplicationAction(final_work)) = queue.pop() else {
            panic!("last action remains queued");
        };
        queue
            .finish_root_action(final_work.root_quota)
            .expect("terminal action returns its unused frontier credit");
        assert_eq!(queue.reserved, 0);
        assert!(queue.is_empty());
        assert_eq!(queue.next_sequence().map(WorkSequence::get), Some(65));
    }

    #[test]
    fn terminal_queue_cleanup_invalidates_held_reservations() {
        let mut queue = WorkQueue::<u8>::new(2);
        let reservation = queue.reserve(2).expect("two slots fit");
        let cancelled = queue.cancel_all();

        assert_eq!(cancelled.envelopes, 0);
        assert_eq!(
            queue.push_reserved_with_remainder(&reservation, |seq, root| action_envelope(
                seq, 1, root
            )),
            Err(QueueReservationError::Stale)
        );
        assert_eq!(
            queue.release_reservation(&reservation),
            Err(QueueReservationError::Stale)
        );
        assert_eq!(queue.reserve(1).err(), Some(QueueReservationError::Closed));
        assert_eq!(queue.len(), 0);
    }
}
