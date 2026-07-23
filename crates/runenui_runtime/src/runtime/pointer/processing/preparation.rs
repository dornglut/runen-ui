use runenui_core::{HostProtocol, PointerPhase};

use super::{PointerGeometry, PointerSnapshot, PointerWork, StreamPreparation};
use crate::{
    MountedNodeId, RuntimeTerminalReason, TraceRecordKind, TraceSequence,
    mounted::TargetStatus,
    runtime::{MandatoryTracePlan, ProcessApplicationActionOutcome, Runtime},
    trace::TraceReservation,
};

impl<State, Action, Protocol: HostProtocol> Runtime<State, Action, Protocol> {
    pub(super) fn prepare_pointer_stream(
        &mut self,
        work: &PointerWork,
    ) -> Result<StreamPreparation, ProcessApplicationActionOutcome> {
        let pointer_id = work.event.pointer_id();
        let phase = work.event.phase();
        let surface = self
            .surface_publication
            .validate_surface_identity(work.event.surface_context())
            .map_err(|error| {
                self.reject_pointer(
                    work.sequence,
                    work.causal_parent,
                    work.trace_reservation,
                    pointer_id,
                    phase,
                    super::rejection::map_surface_error(error),
                )
            })?;
        let existing = match self.pointer_registry.validate(
            pointer_id,
            &surface,
            work.event.device_id(),
            work.event.device_kind(),
        ) {
            Ok(stream) => Some(stream.clone()),
            Err(super::PointerStreamError::Missing) => None,
            Err(error) => {
                return Err(self.reject_pointer(
                    work.sequence,
                    work.causal_parent,
                    work.trace_reservation,
                    pointer_id,
                    phase,
                    super::rejection::map_stream_error(error),
                ));
            }
        };
        if matches!(phase, PointerPhase::Down) {
            let Some(changed_button) = work.event.changed_button() else {
                return Err(self.reject_pointer(
                    work.sequence,
                    work.causal_parent,
                    work.trace_reservation,
                    pointer_id,
                    phase,
                    crate::trace::TracePointerRejection::DuplicateStream,
                ));
            };
            let changed_button_was_active = existing
                .as_ref()
                .is_some_and(|stream| stream.buttons.contains(changed_button));
            if changed_button_was_active || !work.event.buttons().contains(changed_button) {
                return Err(self.reject_pointer(
                    work.sequence,
                    work.causal_parent,
                    work.trace_reservation,
                    pointer_id,
                    phase,
                    crate::trace::TracePointerRejection::DuplicateStream,
                ));
            }
        }
        if matches!(phase, PointerPhase::Up | PointerPhase::Cancel) && existing.is_none() {
            return Err(self.reject_pointer(
                work.sequence,
                work.causal_parent,
                work.trace_reservation,
                pointer_id,
                phase,
                crate::trace::TracePointerRejection::MissingStream,
            ));
        }
        let is_new = existing.is_none();
        let stream = match existing {
            Some(stream) => stream,
            None => self
                .pointer_registry
                .plan_registration(
                    pointer_id,
                    surface,
                    work.event.device_id(),
                    work.event.device_kind(),
                    work.event.position(),
                    work.event.buttons().clone(),
                )
                .map_err(|error| {
                    self.reject_pointer(
                        work.sequence,
                        work.causal_parent,
                        work.trace_reservation,
                        pointer_id,
                        phase,
                        super::rejection::map_registration_error(error),
                    )
                })?,
        };
        Ok(StreamPreparation { is_new, stream })
    }

    pub(super) fn resolve_pointer_geometry(
        &mut self,
        work: &PointerWork,
        stream: &super::PointerStreamState,
    ) -> Result<PointerGeometry, ProcessApplicationActionOutcome> {
        if matches!(work.event.phase(), PointerPhase::Cancel) {
            let physical_path = stream.physical_path().to_vec();
            return Ok(PointerGeometry {
                physical_target: physical_path.last().cloned(),
                physical_path,
                snapshot: None,
            });
        }
        let resolution = match self
            .surface_publication
            .resolve_pointer_point(work.event.surface_context(), work.event.position())
        {
            Ok(resolution) => resolution,
            Err(
                error @ (super::SurfaceSnapshotError::RetiredSurfaceContext
                | super::SurfaceSnapshotError::MissingSurfaceGeneration),
            ) if matches!(work.event.phase(), PointerPhase::Up) => {
                return Err(self.close_unavailable_terminal_pointer(work, error));
            }
            Err(error) => {
                return Err(self.reject_pointer(
                    work.sequence,
                    work.causal_parent,
                    work.trace_reservation,
                    work.event.pointer_id(),
                    work.event.phase(),
                    super::rejection::map_surface_error(error),
                ));
            }
        };
        let snapshot: PointerSnapshot = (
            super::rejection::map_snapshot_kind(resolution.snapshot_kind()),
            resolution.hit_test_generation(),
            resolution.coordinate_revision(),
        );
        let physical_target = resolution.into_target();
        let physical_path = match physical_target.as_ref() {
            Some(target) => {
                let Ok(path) = self.tree.event_route(target) else {
                    self.trace.release_reservation(work.trace_reservation);
                    let cancelled = self.enter_terminal(RuntimeTerminalReason::Poisoned, 0);
                    return Err(ProcessApplicationActionOutcome::Terminal {
                        reason: RuntimeTerminalReason::Poisoned,
                        cancelled,
                    });
                };
                path
            }
            None => Vec::new(),
        };
        Ok(PointerGeometry {
            physical_target,
            physical_path,
            snapshot: Some(snapshot),
        })
    }

    pub(super) fn clear_non_live_pointer_owners(&self, stream: &mut super::PointerStreamState) {
        if stream
            .capture_owner()
            .is_some_and(|owner| self.tree.target_status(owner) != TargetStatus::Live)
        {
            stream.set_capture_owner(None);
        }
        if stream
            .pressed_owner()
            .is_some_and(|owner| self.tree.target_status(owner) != TargetStatus::Live)
        {
            stream.set_pressed_owner(None);
        }
    }

    pub(super) fn pointer_routed_target(
        phase: PointerPhase,
        stream: &super::PointerStreamState,
        physical_target: Option<&MountedNodeId>,
    ) -> Option<MountedNodeId> {
        let capture = stream.capture_owner().cloned();
        let pressed = stream.pressed_owner().cloned();
        match phase {
            PointerPhase::Move | PointerPhase::Wheel => {
                capture.or_else(|| physical_target.cloned())
            }
            PointerPhase::Up => capture.or(pressed).or_else(|| physical_target.cloned()),
            PointerPhase::Cancel => capture.or(pressed),
            _ => physical_target.cloned(),
        }
    }

    pub(super) fn record_pointer_prelude(
        &mut self,
        work: &PointerWork,
        is_new: bool,
        physical_target: Option<&MountedNodeId>,
        snapshot: Option<PointerSnapshot>,
    ) -> Result<Option<TraceSequence>, ProcessApplicationActionOutcome> {
        if !self.trace.can_replace_reservation(
            work.trace_reservation,
            MandatoryTracePlan::pointer_processing(),
        ) {
            self.trace.release_reservation(work.trace_reservation);
            let cancelled = self.enter_terminal(RuntimeTerminalReason::TraceSequenceExhausted, 0);
            return Err(ProcessApplicationActionOutcome::Terminal {
                reason: RuntimeTerminalReason::TraceSequenceExhausted,
                cancelled,
            });
        }
        let pointer_id = work.event.pointer_id();
        let mut parent = if is_new {
            work.causal_parent
        } else {
            self.trace.record_reserved(
                work.trace_reservation,
                TraceRecordKind::PointerStreamObserved { pointer_id },
                work.sequence,
                work.causal_parent,
            )
        };
        let continuation = if is_new {
            work.trace_reservation
        } else {
            TraceReservation::continuation()
        };
        if let Some((snapshot, hit_test_generation, coordinate_revision)) = snapshot {
            let kind = TraceRecordKind::PointerPhysicalTargetResolved {
                pointer_id,
                snapshot,
                hit_test_generation,
                coordinate_revision,
            };
            parent = if is_new {
                self.trace
                    .record_reserved(continuation, kind, work.sequence, parent)
            } else {
                self.trace.record(
                    kind,
                    Some(work.sequence),
                    parent,
                    None,
                    None,
                    physical_target.map(|target| self.tree.trace_target(target)),
                )
            };
        } else if is_new {
            unreachable!("cancel requires an existing pointer stream")
        }
        Ok(parent)
    }
}
