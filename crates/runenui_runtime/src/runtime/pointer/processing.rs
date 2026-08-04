//! Canonical pointer envelope validation, routing, and commit orchestration.

mod lifecycle;
mod notifications;
mod preparation;
mod rehit;
mod rejection;
mod transaction;

use runenui_core::{
    HostProtocol, MonotonicInstant, PointerCaptureEvent, PointerEvent, PointerId, PointerPhase,
    SurfaceInputContext, WorkSequence,
};

use super::{PointerCommitError, PointerRegistrationError, PointerStreamError, PointerStreamState};
use crate::{
    MountedNodeId, TraceSequence, TraceSurfaceSnapshotKind,
    queue::{PointerEnvelope, PointerEnvelopePayload},
    runtime::surface_publication::{SurfaceSnapshotError, SurfaceSnapshotKind},
    runtime::{ProcessApplicationActionOutcome, Runtime},
    trace::TraceReservation,
};
pub(super) use notifications::{PointerBoundaryNotification, PointerBoundaryPlan};

pub(super) struct PointerWork {
    pub(super) sequence: WorkSequence,
    pub(super) event: PointerEvent,
    pub(super) instant: MonotonicInstant,
    pub(super) causal_parent: Option<TraceSequence>,
    pub(super) trace_reservation: TraceReservation,
}

pub(super) struct PointerRehitWork {
    pub(super) sequence: WorkSequence,
    pub(super) context: SurfaceInputContext,
    pub(super) instant: MonotonicInstant,
    pub(super) causal_parent: Option<TraceSequence>,
    pub(super) trace_reservation: TraceReservation,
}

pub(super) struct StreamPreparation {
    pub(super) is_new: bool,
    pub(super) stream: PointerStreamState,
}

pub(super) struct PointerGeometry {
    pub(super) physical_target: Option<MountedNodeId>,
    pub(super) physical_path: Vec<MountedNodeId>,
    pub(super) snapshot: Option<TraceSurfaceSnapshotKind>,
    pub(super) diagnosis: Option<crate::TracePointerRejection>,
}

pub(super) struct PreparedPointer {
    pub(super) work: PointerWork,
    pub(super) is_new: bool,
    pub(super) stream: PointerStreamState,
    pub(super) previous_capture_owner: Option<MountedNodeId>,
    pub(super) geometry: PointerGeometry,
    pub(super) boundary_plan: PointerBoundaryPlan,
    pub(super) routed_target: Option<MountedNodeId>,
    pub(super) parent: Option<TraceSequence>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum StreamCommitKind {
    Register,
    Replace,
    Close,
}

pub(super) struct PointerCommitPlan {
    pub(super) pointer_id: PointerId,
    pub(super) stream: PointerStreamState,
    pub(super) kind: StreamCommitKind,
    pub(super) focus: Option<MountedNodeId>,
    pub(super) capture_events: Vec<PointerCaptureEvent>,
    pub(super) physical_target: Option<MountedNodeId>,
    pub(super) physical_path: Vec<MountedNodeId>,
}

impl<State, Action, Protocol: HostProtocol> Runtime<State, Action, Protocol> {
    pub(crate) fn process_pointer_envelope(
        &mut self,
        envelope: PointerEnvelope,
    ) -> ProcessApplicationActionOutcome {
        let PointerEnvelope {
            sequence,
            payload,
            instant,
            causal_parent,
            trace_reservation,
        } = envelope;
        match payload {
            PointerEnvelopePayload::Event(event) => self.process_pointer_work(PointerWork {
                sequence,
                event,
                instant,
                causal_parent,
                trace_reservation,
            }),
            PointerEnvelopePayload::StationaryRehit(context) => {
                let work = PointerRehitWork {
                    sequence,
                    context,
                    instant,
                    causal_parent,
                    trace_reservation,
                };
                self.process_stationary_pointer_rehit(&work)
            }
        }
    }

    pub(super) fn process_pointer_work(
        &mut self,
        work: PointerWork,
    ) -> ProcessApplicationActionOutcome {
        let prepared_stream = match self.prepare_pointer_stream(&work) {
            Ok(prepared) => prepared,
            Err(outcome) => return outcome,
        };
        let geometry = match self.resolve_pointer_geometry(&work, &prepared_stream.stream) {
            Ok(geometry) => geometry,
            Err(outcome) => return outcome,
        };
        let previous_path = prepared_stream.stream.physical_path().to_vec();
        let previous_capture_owner = prepared_stream.stream.capture_owner().cloned();
        let boundary_plan = if geometry.snapshot.is_some() {
            notifications::plan_boundary_transition(
                work.event.pointer_id(),
                &previous_path,
                &geometry.physical_path,
                work.event.surface_context(),
                |target| self.tree.target_status(target) == crate::mounted::TargetStatus::Live,
            )
        } else {
            PointerBoundaryPlan::unchanged(
                previous_path.last().cloned(),
                geometry.physical_target.clone(),
            )
        };
        let mut stream = prepared_stream.stream;
        stream.update_observation(
            work.event.position(),
            geometry.physical_path.clone(),
            work.event.buttons().clone(),
        );
        stream.set_surface_context(work.event.surface_context().clone());
        self.clear_non_live_pointer_owners(&mut stream);
        let routed_target = Self::pointer_routed_target(
            work.event.phase(),
            &stream,
            geometry.physical_target.as_ref(),
        );
        let parent = match self.record_pointer_prelude(
            &work,
            prepared_stream.is_new,
            &geometry,
            &boundary_plan,
        ) {
            Ok(parent) => parent,
            Err(outcome) => return outcome,
        };
        self.dispatch_prepared_pointer(PreparedPointer {
            work,
            is_new: prepared_stream.is_new,
            stream,
            previous_capture_owner,
            geometry,
            boundary_plan,
            routed_target,
            parent,
        })
    }
}

pub(super) const fn pointer_default_is_cancelable(phase: PointerPhase) -> bool {
    matches!(
        phase,
        PointerPhase::Down | PointerPhase::Up | PointerPhase::Wheel
    )
}
