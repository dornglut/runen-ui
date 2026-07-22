use runenui_core::{HostProtocol, PointerCaptureKind, PointerId, PointerPhase, WorkSequence};

use super::PointerWork;
use crate::{
    RuntimeStatus, RuntimeTerminalReason, TraceRecordKind, TraceSequence,
    runtime::{MandatoryTracePlan, ProcessApplicationActionOutcome, Runtime},
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
