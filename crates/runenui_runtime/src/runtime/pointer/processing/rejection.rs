use runenui_core::{
    CommandOrigin, HostProtocol, PointerCaptureEvent, PointerCaptureKind, PointerId, PointerPhase,
    UiEvent, WorkSequence,
};

use super::PointerWork;
use crate::{
    RuntimeStatus, RuntimeTerminalReason, TraceEventContext, TraceEventFamily, TraceRecordKind,
    TraceSequence,
    mounted::TargetStatus,
    runtime::{
        MandatoryTracePlan, PointerDispatchFacts, ProcessApplicationActionOutcome,
        RoutedIngressFacts, Runtime,
    },
    trace::{TracePointerRejection, TraceReservation},
};

impl<State, Action, Protocol: HostProtocol> Runtime<State, Action, Protocol> {
    pub(super) fn close_unavailable_terminal_pointer(
        &mut self,
        work: &PointerWork,
        error: super::SurfaceSnapshotError,
    ) -> ProcessApplicationActionOutcome {
        if !self.trace.can_replace_reservation(
            work.trace_reservation,
            MandatoryTracePlan::pointer_processing(),
        ) {
            self.trace.release_reservation(work.trace_reservation);
            let cancelled = self.enter_terminal(RuntimeTerminalReason::TraceSequenceExhausted, 0);
            return ProcessApplicationActionOutcome::Terminal {
                reason: RuntimeTerminalReason::TraceSequenceExhausted,
                cancelled,
            };
        }
        let pointer_id = work.event.pointer_id();
        let stream = self
            .pointer_registry
            .stream(pointer_id)
            .cloned()
            .unwrap_or_else(|| unreachable!("terminal cleanup follows active-stream validation"));
        let rejected = self.trace.record_reserved(
            work.trace_reservation,
            TraceRecordKind::PointerIngressRejected {
                pointer_id,
                phase: work.event.phase(),
                outcome: map_surface_error(error),
            },
            work.sequence,
            work.causal_parent,
        );
        if let Some(owner) = stream
            .capture_owner()
            .filter(|owner| self.tree.target_status(owner) == TargetStatus::Live)
            .cloned()
        {
            return self.close_unavailable_terminal_pointer_with_live_capture(
                work, rejected, &stream, &owner,
            );
        }
        let stream = self
            .pointer_registry
            .close(pointer_id)
            .unwrap_or_else(|| unreachable!("terminal cleanup follows active-stream validation"));
        let pressed = stream.pressed_owner().is_some();
        let capture = stream.capture_owner().is_some();
        let physical_path = !stream.physical_path().is_empty();
        let cleanup = self.trace.record(
            TraceRecordKind::PointerIntegrityCleanupCommitted {
                pointer_id,
                pressed,
                capture,
                physical_path,
            },
            Some(work.sequence),
            rejected,
            None,
            None,
            None,
        );
        let parent = if capture {
            self.trace.record(
                TraceRecordKind::PointerCaptureNotificationSuppressed {
                    pointer_id,
                    kind: PointerCaptureKind::Lost,
                },
                Some(work.sequence),
                cleanup,
                None,
                None,
                None,
            )
        } else {
            cleanup
        };
        self.trace.record(
            TraceRecordKind::PointerStreamClosed { pointer_id },
            Some(work.sequence),
            parent,
            None,
            None,
            None,
        );
        ProcessApplicationActionOutcome::Completed
    }

    fn close_unavailable_terminal_pointer_with_live_capture(
        &mut self,
        work: &PointerWork,
        rejected: Option<TraceSequence>,
        stream: &super::PointerStreamState,
        owner: &crate::MountedNodeId,
    ) -> ProcessApplicationActionOutcome {
        let pointer_id = work.event.pointer_id();
        let facts = RoutedIngressFacts::new(
            work.sequence,
            owner.clone(),
            CommandOrigin::__runtime_pointer(),
            work.instant,
            TraceEventContext::new(TraceEventFamily::PointerCapture, false),
            rejected,
            TraceReservation::continuation(),
        );
        let Some(transaction) = self.begin_pointer_routed_transaction(
            facts,
            false,
            &[],
            core::slice::from_ref(owner),
            1,
            MandatoryTracePlan::pointer_commit(0)
                .unwrap_or_else(|| unreachable!("zero boundary notifications fit")),
            false,
        ) else {
            let cancelled = self.enter_terminal(RuntimeTerminalReason::Poisoned, 0);
            return ProcessApplicationActionOutcome::Terminal {
                reason: RuntimeTerminalReason::Poisoned,
                cancelled,
            };
        };
        let physical_path = stream.physical_path().to_vec();
        let physical_target = physical_path.last().cloned();
        let capture_event = PointerCaptureEvent::__runtime_new(
            pointer_id,
            PointerCaptureKind::Lost,
            owner.clone(),
            None,
            work.event.surface_context().clone(),
        );
        let pressed = stream.pressed_owner().is_some();
        let physical_path_present = !physical_path.is_empty();
        let failure_facts = transaction.failure_facts();
        let callback_owner = owner.clone();
        let result =
            self.commit_routed_transaction_with(transaction, move |runtime, transaction| {
                runtime.pointer_registry.close(pointer_id).ok_or(())?;
                transaction.parent = runtime.trace.record(
                    TraceRecordKind::PointerIntegrityCleanupCommitted {
                        pointer_id,
                        pressed,
                        capture: true,
                        physical_path: physical_path_present,
                    },
                    Some(transaction.sequence),
                    rejected,
                    None,
                    None,
                    None,
                );
                transaction.parent = runtime.trace.record(
                    TraceRecordKind::PointerCaptureTransitionQueued {
                        pointer_id,
                        kind: PointerCaptureKind::Lost,
                    },
                    Some(transaction.sequence),
                    transaction.parent,
                    None,
                    None,
                    Some(runtime.tree.trace_target(&callback_owner)),
                );
                transaction.parent = runtime.trace.record(
                    TraceRecordKind::PointerStreamClosed { pointer_id },
                    Some(transaction.sequence),
                    transaction.parent,
                    None,
                    None,
                    None,
                );
                let event = UiEvent::PointerCapture(capture_event);
                let dispatch = PointerDispatchFacts::new(
                    pointer_id,
                    physical_target.as_ref(),
                    &physical_path,
                    None,
                    false,
                );
                runtime
                    .invoke_target_only_pointer_callback(
                        transaction,
                        &event,
                        dispatch,
                        &callback_owner,
                    )
                    .map_err(|_| ())?;
                transaction.pointer_capture_requests.clear();
                Ok(())
            });
        if result.is_err() {
            self.poison_routed_event(
                &failure_facts,
                crate::TraceRoutedIntegrityFailure::CommitInvariantFailure,
                Some(owner),
            );
        }
        self.pointer_runtime_outcome()
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn reject_pointer(
        &mut self,
        sequence: WorkSequence,
        causal_parent: Option<TraceSequence>,
        trace_reservation: TraceReservation,
        pointer_id: PointerId,
        phase: PointerPhase,
        outcome: TracePointerRejection,
    ) -> ProcessApplicationActionOutcome {
        self.trace.record_reserved(
            trace_reservation,
            TraceRecordKind::PointerIngressRejected {
                pointer_id,
                phase,
                outcome,
            },
            sequence,
            causal_parent,
        );
        ProcessApplicationActionOutcome::Completed
    }

    pub(super) const fn pointer_runtime_outcome(&self) -> ProcessApplicationActionOutcome {
        match self.status() {
            RuntimeStatus::Terminal(reason) => ProcessApplicationActionOutcome::Terminal {
                reason,
                cancelled: 0,
            },
            RuntimeStatus::Running | RuntimeStatus::Closed => {
                ProcessApplicationActionOutcome::Completed
            }
        }
    }
}

pub(super) const fn map_surface_error(error: super::SurfaceSnapshotError) -> TracePointerRejection {
    match error {
        super::SurfaceSnapshotError::ForeignSurfaceContext => TracePointerRejection::ForeignRuntime,
        super::SurfaceSnapshotError::ForeignSurface => TracePointerRejection::ForeignSurface,
        super::SurfaceSnapshotError::RetiredSurfaceContext => {
            TracePointerRejection::RetiredGeneration
        }
        super::SurfaceSnapshotError::MissingSurfaceGeneration => {
            TracePointerRejection::MissingGeneration
        }
        super::SurfaceSnapshotError::CoordinateRevisionMismatch => {
            TracePointerRejection::CoordinateRevisionMismatch
        }
        super::SurfaceSnapshotError::NoTarget
        | super::SurfaceSnapshotError::TargetNotInSnapshot => TracePointerRejection::NoTarget,
    }
}

pub(super) const fn map_stream_error(error: super::PointerStreamError) -> TracePointerRejection {
    match error {
        super::PointerStreamError::Missing => TracePointerRejection::MissingStream,
        super::PointerStreamError::ForeignSurface => TracePointerRejection::ForeignStreamSurface,
        super::PointerStreamError::DeviceMismatch => TracePointerRejection::DeviceMismatch,
        super::PointerStreamError::DeviceKindMismatch => TracePointerRejection::DeviceKindMismatch,
    }
}

pub(super) const fn map_registration_error(
    error: super::PointerRegistrationError,
) -> TracePointerRejection {
    match error {
        super::PointerRegistrationError::Duplicate => TracePointerRejection::DuplicateStream,
        super::PointerRegistrationError::Full => TracePointerRejection::RegistryFull,
        super::PointerRegistrationError::RegistrationSequenceExhausted => {
            TracePointerRejection::RegistrationSequenceExhausted
        }
    }
}

pub(super) const fn map_snapshot_kind(
    kind: super::SurfaceSnapshotKind,
) -> crate::TraceSurfaceSnapshotKind {
    match kind {
        super::SurfaceSnapshotKind::Current => crate::TraceSurfaceSnapshotKind::Current,
        super::SurfaceSnapshotKind::Retained => crate::TraceSurfaceSnapshotKind::RetainedHistorical,
    }
}
