use runenui_core::{HostProtocol, PointerEvent, PointerPhase};

use super::super::PointerRegistry;
use super::{PointerRehitWork, PointerWork, PreparedPointer};
use crate::runtime::{ProcessApplicationActionOutcome, Runtime};
use crate::trace::TraceReservation;

impl PointerRegistry {
    pub(in crate::runtime) fn has_streams(&self) -> bool {
        !self.streams.is_empty()
    }
}

impl<State, Action, Protocol: HostProtocol> Runtime<State, Action, Protocol> {
    pub(super) fn process_stationary_pointer_rehit(
        &mut self,
        work: &PointerRehitWork,
    ) -> ProcessApplicationActionOutcome {
        let mut streams = self
            .pointer_registry
            .streams
            .iter()
            .map(|(pointer_id, stream)| (*pointer_id, stream.clone()))
            .collect::<Vec<_>>();
        streams.sort_unstable_by_key(|(_, stream)| stream.registration_sequence());
        if streams.is_empty() {
            self.trace.release_reservation(work.trace_reservation);
            return ProcessApplicationActionOutcome::Completed;
        }

        let mut trace_reservation = work.trace_reservation;
        for (pointer_id, stream) in streams {
            let mut event = PointerEvent::new(
                pointer_id,
                stream.device_kind,
                PointerPhase::Move,
                stream.position,
                work.context.clone(),
            )
            .with_buttons(stream.buttons.clone());
            if let Some(device_id) = stream.device_id {
                event = event.with_device_id(device_id);
            }
            let outcome = self.process_stationary_pointer_work(PointerWork {
                sequence: work.sequence,
                event,
                instant: work.instant,
                causal_parent: work.causal_parent,
                trace_reservation,
            });
            if matches!(outcome, ProcessApplicationActionOutcome::Terminal { .. }) {
                return outcome;
            }
            trace_reservation = TraceReservation::continuation();
        }
        ProcessApplicationActionOutcome::Completed
    }

    fn process_stationary_pointer_work(
        &mut self,
        work: PointerWork,
    ) -> ProcessApplicationActionOutcome {
        let prepared_stream = match self.prepare_pointer_stream(&work) {
            Ok(prepared) if !prepared.is_new => prepared,
            Ok(_) => {
                self.trace.release_reservation(work.trace_reservation);
                return ProcessApplicationActionOutcome::Completed;
            }
            Err(outcome) => return outcome,
        };
        let geometry = match self.resolve_pointer_geometry(&work, &prepared_stream.stream) {
            Ok(geometry) => geometry,
            Err(outcome) => return outcome,
        };
        let previous_path = prepared_stream.stream.physical_path().to_vec();
        let previous_capture_owner = prepared_stream.stream.capture_owner().cloned();
        let boundary_events = super::notifications::plan_boundary_events(
            work.event.pointer_id(),
            &previous_path,
            &geometry.physical_path,
            work.event.surface_context(),
        );
        let mut stream = prepared_stream.stream;
        stream.update_observation(
            work.event.position(),
            geometry.physical_path.clone(),
            work.event.buttons().clone(),
        );
        stream.set_surface_context(work.event.surface_context().clone());
        self.clear_non_live_pointer_owners(&mut stream);
        let parent = match self.record_pointer_prelude(
            &work,
            false,
            &geometry,
            boundary_events.len(),
        ) {
            Ok(parent) => parent,
            Err(outcome) => return outcome,
        };
        self.dispatch_prepared_pointer(PreparedPointer {
            work,
            is_new: false,
            stream,
            previous_capture_owner,
            geometry,
            boundary_events,
            routed_target: None,
            parent,
        })
    }
}
