use runenui_core::{
    CommandOrigin, HostProtocol, InputDeviceId, MonotonicInstant, PointerCaptureEvent,
    PointerCaptureKind, PointerDeviceKind, PointerId, PointerPhase, SurfaceInputContext, UiEvent,
    WorkSequence,
};

use super::{PointerStreamState, PointerWork};
use crate::{
    MountedNodeId, RuntimeStatus, RuntimeTerminalReason, TraceContext, TraceDeliveryOutcome,
    TraceEventContext, TraceEventFamily, TracePointerCleanup, TracePointerContext,
    TracePointerPath, TraceRecordKind, TraceRouteSnapshot, TraceSequence, TraceSurfaceContext,
    TraceTargetTransition,
    mounted::TargetStatus,
    runtime::{
        MandatoryTracePlan, PointerDispatchFacts, ProcessApplicationActionOutcome,
        RoutedIngressFacts, Runtime,
    },
    trace::{TracePointerRejection, TraceRecordDraft, TraceReservation},
};

#[derive(Clone)]
struct RejectedPointerCleanupTrace {
    pointer_id: PointerId,
    device_id: Option<InputDeviceId>,
    device_kind: PointerDeviceKind,
    pressed_owner: Option<MountedNodeId>,
    capture_owner: Option<MountedNodeId>,
    physical_path: Vec<MountedNodeId>,
    surface_context: Option<SurfaceInputContext>,
}

impl RejectedPointerCleanupTrace {
    fn from_stream(pointer_id: PointerId, stream: &PointerStreamState) -> Self {
        Self {
            pointer_id,
            device_id: stream.device_id(),
            device_kind: stream.device_kind(),
            pressed_owner: stream.pressed_owner().cloned(),
            capture_owner: stream.capture_owner().cloned(),
            physical_path: stream.physical_path().to_vec(),
            surface_context: stream.surface_context().cloned(),
        }
    }
}

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
        let cleanup_trace = RejectedPointerCleanupTrace::from_stream(pointer_id, &stream);
        self.pointer_registry
            .close(pointer_id)
            .unwrap_or_else(|| unreachable!("terminal cleanup follows active-stream validation"));
        let cleanup = self.record_rejected_pointer_cleanup(
            &cleanup_trace,
            work.sequence,
            rejected,
            work.instant,
        );
        let parent = if cleanup_trace.capture_owner.is_some() {
            self.record_rejected_capture_loss(
                &cleanup_trace,
                &[],
                TraceDeliveryOutcome::Suppressed,
                work.sequence,
                cleanup,
                work.instant,
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
        stream: &PointerStreamState,
        owner: &MountedNodeId,
    ) -> ProcessApplicationActionOutcome {
        let pointer_id = work.event.pointer_id();
        let facts = Self::rejected_capture_ingress_facts(work, owner, rejected);
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
        let cleanup_trace = RejectedPointerCleanupTrace::from_stream(pointer_id, stream);
        let physical_path = cleanup_trace.physical_path.clone();
        let physical_target = physical_path.last().cloned();
        let capture_event = PointerCaptureEvent::__runtime_new(
            pointer_id,
            PointerCaptureKind::Lost,
            owner.clone(),
            None,
            work.event.surface_context().clone(),
        );
        let failure_facts = transaction.failure_facts();
        let callback_owner = owner.clone();
        let instant = work.instant;
        let result =
            self.commit_routed_transaction_with(transaction, move |runtime, transaction| {
                runtime.pointer_registry.close(pointer_id).ok_or(())?;
                transaction.parent = runtime.record_rejected_pointer_cleanup(
                    &cleanup_trace,
                    transaction.sequence,
                    rejected,
                    instant,
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
                transaction.parent = runtime.record_rejected_capture_loss(
                    &cleanup_trace,
                    &physical_path,
                    TraceDeliveryOutcome::Delivered,
                    transaction.sequence,
                    transaction.parent,
                    instant,
                );
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

    fn record_rejected_pointer_cleanup(
        &mut self,
        cleanup: &RejectedPointerCleanupTrace,
        sequence: WorkSequence,
        parent: Option<TraceSequence>,
        instant: MonotonicInstant,
    ) -> Option<TraceSequence> {
        if !self.trace.is_enabled() {
            return parent;
        }
        let physical_path = TracePointerPath::new(
            cleanup
                .physical_path
                .iter()
                .map(|node| self.tree.trace_target(node))
                .collect(),
        );
        let pressed_owner = cleanup.pressed_owner.as_ref().map(|owner| {
            TraceTargetTransition::new(Some(self.tree.trace_target(owner)), None)
        });
        let capture_owner = cleanup.capture_owner.as_ref().map(|owner| {
            TraceTargetTransition::new(Some(self.tree.trace_target(owner)), None)
        });
        let pointer = TracePointerContext::stream(
            cleanup.pointer_id,
            cleanup.device_id,
            cleanup.device_kind,
        );
        let surface = cleanup
            .surface_context
            .as_ref()
            .map(TraceSurfaceContext::requested);
        let context = TraceContext::pointer_integrity_cleanup(
            surface,
            pointer,
            physical_path,
            TracePointerCleanup::new(
                pressed_owner,
                capture_owner,
                !cleanup.physical_path.is_empty(),
            ),
        );
        self.trace.record_draft(
            TraceRecordDraft::pointer_fact(
                TraceRecordKind::PointerIntegrityCleanupCommitted,
                instant,
                context,
            )
            .with_work_sequence(Some(sequence))
            .with_causal_parent(parent),
        )
    }

    fn record_rejected_capture_loss(
        &mut self,
        cleanup: &RejectedPointerCleanupTrace,
        physical_path: &[MountedNodeId],
        delivery: TraceDeliveryOutcome,
        sequence: WorkSequence,
        parent: Option<TraceSequence>,
        instant: MonotonicInstant,
    ) -> Option<TraceSequence> {
        if !self.trace.is_enabled() {
            return parent;
        }
        let owner = cleanup
            .capture_owner
            .as_ref()
            .unwrap_or_else(|| unreachable!("capture cleanup retains its previous owner"));
        let target = self.tree.trace_target(owner);
        let route = TraceRouteSnapshot::new(vec![target.clone()], None);
        let physical_path = TracePointerPath::new(
            physical_path
                .iter()
                .map(|node| self.tree.trace_target(node))
                .collect(),
        );
        let transition = TraceTargetTransition::new(Some(target.clone()), None);
        let pointer = TracePointerContext::stream(
            cleanup.pointer_id,
            cleanup.device_id,
            cleanup.device_kind,
        );
        let surface = cleanup
            .surface_context
            .as_ref()
            .map(TraceSurfaceContext::requested);
        let context = TraceContext::pointer_capture_notification(
            surface,
            pointer,
            route,
            physical_path,
            transition,
            delivery,
        );
        self.trace.record_draft(
            TraceRecordDraft::pointer_fact(
                TraceRecordKind::PointerCaptureNotificationResolved {
                    kind: PointerCaptureKind::Lost,
                },
                instant,
                context,
            )
            .with_work_sequence(Some(sequence))
            .with_causal_parent(parent)
            .with_target(Some(target)),
        )
    }

    fn rejected_capture_ingress_facts(
        work: &PointerWork,
        owner: &MountedNodeId,
        rejected: Option<TraceSequence>,
    ) -> RoutedIngressFacts {
        RoutedIngressFacts::new(
            work.sequence,
            owner.clone(),
            CommandOrigin::__runtime_pointer(),
            work.instant,
            TraceEventContext::new(TraceEventFamily::PointerCapture, false),
            rejected,
            TraceReservation::continuation(),
        )
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
