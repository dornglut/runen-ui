use runenui_core::{HostProtocol, PointerEvent};

use super::super::Runtime;
use crate::{
    PointerSubmission, RuntimeStatus, SubmitPointerError, SubmitPointerErrorKind, TraceRecordKind,
    queue::{
        InputContinuationKey, PointerEnvelope, PointerEnvelopePayload, QueueReservationError,
        WorkEnvelope,
    },
};

impl<State, Action, Protocol: HostProtocol> Runtime<State, Action, Protocol> {
    #[allow(
        clippy::too_many_lines,
        reason = "pointer admission commits trace, sequence and exact continuation ownership in one ordered boundary"
    )]
    pub(crate) fn submit_pointer(
        &mut self,
        event: PointerEvent,
    ) -> Result<PointerSubmission, SubmitPointerError> {
        self.retire_input_continuation_claims();
        match self.status {
            RuntimeStatus::Running => {}
            RuntimeStatus::Closed => {
                return Err(SubmitPointerError::new(
                    SubmitPointerErrorKind::Closed,
                    event,
                ));
            }
            RuntimeStatus::Terminal(reason) => {
                return Err(SubmitPointerError::new(
                    SubmitPointerErrorKind::Terminal(reason),
                    event,
                ));
            }
        }
        let continuation = self
            .pointer_continuation_key(&event)
            .filter(|key| self.queue.input_continuation(key).is_some());
        let profile = self
            .interaction_producer_budget
            .map(|budget| budget(self.state()))
            .filter(|profile| self.application_output_budget_fits(*profile));
        if profile.is_none() && continuation.is_none() {
            return Err(SubmitPointerError::new(
                SubmitPointerErrorKind::InvalidProducerProfile,
                event,
            ));
        }
        let reservation = if let Some(key) = &continuation {
            self.queue.preflight_input_continuation(key).and_then(|()| {
                self.queue
                    .input_continuation(key)
                    .ok_or(QueueReservationError::Stale)
            })
        } else {
            self.reserve_producer(profile.unwrap_or_else(|| unreachable!("fresh profile checked")))
        };
        let producer = match reservation {
            Ok(reservation) => reservation,
            Err(error) => {
                let kind = match error {
                    QueueReservationError::Capacity => SubmitPointerErrorKind::Full,
                    QueueReservationError::WorkCapacity(family) => {
                        SubmitPointerErrorKind::WorkCapacity(family)
                    }
                    QueueReservationError::Closed => SubmitPointerErrorKind::Closed,
                    QueueReservationError::SequenceExhausted => {
                        SubmitPointerErrorKind::WorkSequenceExhausted
                    }
                    QueueReservationError::Foreign
                    | QueueReservationError::Stale
                    | QueueReservationError::Exhausted => {
                        SubmitPointerErrorKind::InvalidProducerProfile
                    }
                };
                return Err(SubmitPointerError::new(kind, event));
            }
        };
        let pending_claims = if continuation.is_none()
            && matches!(
                event.phase(),
                runenui_core::PointerPhase::Down
                    | runenui_core::PointerPhase::Move
                    | runenui_core::PointerPhase::Wheel
            ) {
            event.buttons().iter().len().max(1)
        } else {
            0
        };
        if pending_claims != 0
            && self
                .queue
                .preflight_input_continuations(&producer, pending_claims)
                .is_err()
        {
            self.release_unused_reservation(&producer);
            return Err(SubmitPointerError::new(
                SubmitPointerErrorKind::InvalidProducerProfile,
                event,
            ));
        }
        let Some(sequence) = self.queue.next_sequence() else {
            if continuation.is_none() {
                self.release_unused_reservation(&producer);
            }
            return Err(SubmitPointerError::new(
                SubmitPointerErrorKind::WorkSequenceExhausted,
                event,
            ));
        };
        let Some(trace_reservation) = self.trace.reserve_pointer_outcome() else {
            if continuation.is_none() {
                self.release_unused_reservation(&producer);
            }
            return Err(SubmitPointerError::new(
                SubmitPointerErrorKind::TraceSequenceExhausted,
                event,
            ));
        };
        let instant = self.now();
        let accepted = self.trace.record(
            TraceRecordKind::PointerSubmissionAccepted {
                pointer_id: event.pointer_id(),
                phase: event.phase(),
            },
            Some(sequence),
            None,
            None,
            None,
            None,
        );
        if self.trace.is_enabled() && accepted.is_none() {
            self.trace.release_reservation(trace_reservation);
            if continuation.is_none() {
                self.release_unused_reservation(&producer);
            }
            return Err(SubmitPointerError::new(
                SubmitPointerErrorKind::TraceSequenceExhausted,
                event,
            ));
        }
        let committed = if let Some(key) = &continuation {
            self.queue.push_input_continuation(key, |sequence, root| {
                WorkEnvelope::Pointer(PointerEnvelope {
                    sequence,
                    root_quota: Some(root),
                    payload: PointerEnvelopePayload::Event(event),
                    instant,
                    causal_parent: accepted,
                    trace_reservation,
                })
            })
        } else {
            self.queue.push_pointer_preflighted(
                &producer,
                event,
                instant,
                accepted,
                trace_reservation,
            )
        }
        .unwrap_or_else(|_| unreachable!("pointer frontier and sequence were reserved"));
        for index in 0..pending_claims {
            self.queue
                .retain_input_continuation(
                    producer.quota(),
                    InputContinuationKey::PendingPointer(committed, index),
                )
                .unwrap_or_else(|_| unreachable!("source continuation claims were preflighted"));
        }
        debug_assert_eq!(committed, sequence);
        self.external_queue_commit_accepted();
        Ok(PointerSubmission::new(committed))
    }
}
