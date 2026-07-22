use runenui_core::{HostProtocol, PointerEvent};

use super::super::Runtime;
use crate::{
    PointerSubmission, RuntimeStatus, SubmitPointerError, SubmitPointerErrorKind, TraceRecordKind,
    queue::QueueCommitError,
};

impl<State, Action, Protocol: HostProtocol> Runtime<State, Action, Protocol> {
    pub(crate) fn submit_pointer(
        &mut self,
        event: PointerEvent,
    ) -> Result<PointerSubmission, SubmitPointerError> {
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
        if self.queue.is_full() {
            return Err(SubmitPointerError::new(SubmitPointerErrorKind::Full, event));
        }
        let Some(sequence) = self.queue.next_sequence() else {
            return Err(SubmitPointerError::new(
                SubmitPointerErrorKind::WorkSequenceExhausted,
                event,
            ));
        };
        let Some(trace_reservation) = self.trace.reserve_pointer_outcome() else {
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
            return Err(SubmitPointerError::new(
                SubmitPointerErrorKind::TraceSequenceExhausted,
                event,
            ));
        }
        match self
            .queue
            .push_pointer_preflighted(event, instant, accepted, trace_reservation)
        {
            Ok(committed) => {
                debug_assert_eq!(committed, sequence);
                self.external_queue_commit_accepted();
                Ok(PointerSubmission::new(committed))
            }
            Err(QueueCommitError::Full) => {
                self.trace.release_reservation(trace_reservation);
                unreachable!("pointer queue capacity was preflighted")
            }
            Err(QueueCommitError::SequenceExhausted) => {
                self.trace.release_reservation(trace_reservation);
                unreachable!("pointer sequence authority was preflighted")
            }
        }
    }
}
