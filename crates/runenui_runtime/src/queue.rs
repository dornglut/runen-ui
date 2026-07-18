//! Canonical application-work queue and work sequencing.

#![allow(clippy::redundant_pub_crate)]

use core::{fmt, num::NonZeroU64};
use std::collections::VecDeque;

use crate::{MountedNodeId, work::WorkGeneration};
use crate::{RuntimeTerminalReason, TraceSequence, TraceTarget, TraceWorkIdentity};

/// Non-wrapping sequence assigned to accepted runtime work.
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct WorkSequence(NonZeroU64);

impl WorkSequence {
    /// Returns the numeric sequence value.
    #[must_use]
    pub const fn get(self) -> u64 {
        self.0.get()
    }

    const fn new(value: NonZeroU64) -> Self {
        Self(value)
    }
}

/// Borrowed classification of an action-submission failure.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SubmitActionErrorKind {
    Full,
    Closed,
    Terminal(RuntimeTerminalReason),
}

/// An unaccepted application action and the exact rejection reason.
#[must_use]
#[non_exhaustive]
pub enum SubmitActionError<Action> {
    Full(Action),
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
            Self::Closed(_) => SubmitActionErrorKind::Closed,
            Self::Terminal { reason, .. } => SubmitActionErrorKind::Terminal(*reason),
        }
    }

    /// Recovers the exact unaccepted action.
    #[must_use]
    pub fn into_action(self) -> Action {
        match self {
            Self::Full(action) | Self::Closed(action) | Self::Terminal { action, .. } => action,
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
            SubmitActionErrorKind::Closed => formatter.write_str("runtime is closed"),
            SubmitActionErrorKind::Terminal(reason) => {
                write!(formatter, "runtime is terminal: {reason}")
            }
        }
    }
}

/// Result of submitting one owned application action.
pub type SubmitActionResult<Action> = Result<WorkSequence, SubmitActionError<Action>>;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ApplicationActionOrigin {
    DirectSubmission,
    MountedActivation,
    KeyboardActivation,
    PointerActivation,
    ApplicationEffect,
}

pub(crate) struct ApplicationActionEnvelope<Action> {
    pub(crate) sequence: WorkSequence,
    pub(crate) action: Action,
    pub(crate) causal_parent: Option<TraceSequence>,
    pub(crate) target: Option<TraceTarget>,
    pub(crate) origin: ApplicationActionOrigin,
}

pub(crate) enum WorkEnvelope<Action> {
    ApplicationAction(ApplicationActionEnvelope<Action>),
    EffectStart(SequencedWork),
    WorkCancellation(CancellationEnvelope),
    TimerFiring(SequencedWork),
    MountedSubscriptionReconcile {
        sequence: WorkSequence,
        owner: MountedNodeId,
        causal_parent: Option<TraceSequence>,
    },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct SequencedWork {
    pub(crate) sequence: WorkSequence,
    pub(crate) generation: WorkGeneration,
}

pub(crate) struct CancellationEnvelope {
    pub(crate) sequence: WorkSequence,
    pub(crate) generation: WorkGeneration,
    pub(crate) identity: TraceWorkIdentity,
    pub(crate) causal_parent: Option<TraceSequence>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum QueueCommitError {
    Full,
    SequenceExhausted,
}

pub(crate) struct WorkQueue<Action> {
    capacity: usize,
    waiting: VecDeque<WorkEnvelope<Action>>,
    next_sequence: Option<NonZeroU64>,
}

impl<Action> WorkQueue<Action> {
    pub(crate) const fn new(capacity: usize) -> Self {
        Self {
            capacity,
            waiting: VecDeque::new(),
            next_sequence: NonZeroU64::new(1),
        }
    }

    pub(crate) fn is_full(&self) -> bool {
        self.waiting.len() >= self.capacity
    }

    pub(crate) fn preflight_commit(&self, count: usize) -> Result<(), QueueCommitError> {
        if self
            .waiting
            .len()
            .checked_add(count)
            .is_none_or(|required| required > self.capacity)
        {
            return Err(QueueCommitError::Full);
        }
        if count == 0 {
            return Ok(());
        }
        let next = self
            .next_sequence
            .ok_or(QueueCommitError::SequenceExhausted)?;
        let additional =
            u64::try_from(count - 1).map_err(|_| QueueCommitError::SequenceExhausted)?;
        next.get()
            .checked_add(additional)
            .ok_or(QueueCommitError::SequenceExhausted)?;
        Ok(())
    }

    pub(crate) const fn has_sequence(&self) -> bool {
        self.next_sequence.is_some()
    }

    pub(crate) fn next_sequence(&self) -> Option<WorkSequence> {
        self.next_sequence.map(WorkSequence::new)
    }

    pub(crate) fn push_preflighted(
        &mut self,
        action: Action,
        causal_parent: Option<TraceSequence>,
        target: Option<TraceTarget>,
        origin: ApplicationActionOrigin,
    ) -> Result<WorkSequence, Action> {
        let Some(next_sequence) = self.next_sequence else {
            return Err(action);
        };
        let sequence = WorkSequence::new(next_sequence);
        self.next_sequence = sequence.get().checked_add(1).and_then(NonZeroU64::new);
        self.waiting
            .push_back(WorkEnvelope::ApplicationAction(ApplicationActionEnvelope {
                sequence,
                action,
                causal_parent,
                target,
                origin,
            }));
        Ok(sequence)
    }

    pub(crate) fn push_effect_start(
        &mut self,
        generation: WorkGeneration,
    ) -> Result<WorkSequence, QueueCommitError> {
        self.push_control(|sequence| {
            WorkEnvelope::EffectStart(SequencedWork {
                sequence,
                generation,
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
            })
        })
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
        })
    }

    fn push_control(
        &mut self,
        envelope: impl FnOnce(WorkSequence) -> WorkEnvelope<Action>,
    ) -> Result<WorkSequence, QueueCommitError> {
        if self.is_full() {
            return Err(QueueCommitError::Full);
        }
        let next = self
            .next_sequence
            .ok_or(QueueCommitError::SequenceExhausted)?;
        let sequence = WorkSequence::new(next);
        self.next_sequence = sequence.get().checked_add(1).and_then(NonZeroU64::new);
        self.waiting.push_back(envelope(sequence));
        Ok(sequence)
    }

    pub(crate) fn pop(&mut self) -> Option<WorkEnvelope<Action>> {
        self.waiting.pop_front()
    }

    pub(crate) fn len(&self) -> usize {
        self.waiting.len()
    }

    pub(crate) fn is_empty(&self) -> bool {
        self.waiting.is_empty()
    }

    pub(crate) fn cancel_all(&mut self) -> usize {
        let count = self.waiting.len();
        self.waiting.clear();
        count
    }

    #[cfg(feature = "internal-test-seams")]
    pub(crate) const fn seed_next_sequence_for_test(&mut self, next: u64) {
        self.next_sequence = NonZeroU64::new(next);
    }
}
