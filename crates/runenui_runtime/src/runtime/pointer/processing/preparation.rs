use runenui_core::{HostProtocol, PointerPhase};

use super::{PointerBoundaryPlan, PointerGeometry, PointerWork, StreamPreparation};
use crate::{
    MountedNodeId, RuntimeTerminalReason, TraceContext, TraceEventContext, TraceEventFamily,
    TracePointerContext, TracePointerPath, TraceRecordKind, TraceSequence, TraceSurfaceContext,
    TraceTargetTransition,
    mounted::TargetStatus,
    runtime::{MandatoryTracePlan, ProcessApplicationActionOutcome, Runtime},
    trace::TraceRecordDraft,
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
        if matches!(phase, PointerPhase::Down)
            && existing.as_ref().is_some_and(|stream| {
                work.event
                    .changed_button()
                    .is_none_or(|button| stream.buttons().contains(button))
            })
        {
            return Err(self.reject_pointer(
                work.sequence,
                work.causal_parent,
                work.trace_reservation,
                pointer_id,
                phase,
                crate::trace::TracePointerRejection::DuplicateStream,
            ));
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
            let diagnosis = self
                .surface_publication
                .resolve_pointer_point(work.event.surface_context(), work.event.position())
                .err()
                .map(super::rejection::map_surface_error);
            let physical_path = stream.physical_path().to_vec();
            return Ok(PointerGeometry {
                physical_target: physical_path.last().cloned(),
                physical_path,
                snapshot: None,
                diagnosis,
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
        let snapshot = super::rejection::map_snapshot_kind(resolution.snapshot_kind());
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
            diagnosis: None,
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

    const fn pointer_trace_context(work: &PointerWork) -> TracePointerContext {
        TracePointerContext::event(
            work.event.pointer_id(),
            work.event.device_id(),
            work.event.device_kind(),
            work.event.phase(),
        )
    }

    fn trace_pointer_path(&self, path: &[MountedNodeId]) -> TracePointerPath {
        TracePointerPath::new(
            path.iter()
                .map(|target| self.tree.trace_target(target))
                .collect(),
        )
    }

    fn record_physical_pointer_observation(
        &mut self,
        work: &PointerWork,
        geometry: &PointerGeometry,
        parent: Option<TraceSequence>,
    ) -> Option<TraceSequence> {
        let Some(snapshot) = geometry.snapshot else {
            return parent;
        };
        if !self.trace.is_enabled() {
            return parent;
        }
        let context = TraceContext::pointer_observation(
            TraceEventContext::new(
                TraceEventFamily::Pointer,
                super::pointer_default_is_cancelable(work.event.phase()),
            ),
            TraceSurfaceContext::accepted(work.event.surface_context(), snapshot),
            Self::pointer_trace_context(work),
            self.trace_pointer_path(&geometry.physical_path),
        );
        self.trace.record_draft(
            TraceRecordDraft::pointer_fact(
                TraceRecordKind::PointerPhysicalTargetResolved,
                work.instant,
                context,
            )
            .with_work_sequence(Some(work.sequence))
            .with_causal_parent(parent)
            .with_target(
                geometry
                    .physical_target
                    .as_ref()
                    .map(|target| self.tree.trace_target(target)),
            ),
        )
    }

    fn record_pointer_boundary_plan(
        &mut self,
        work: &PointerWork,
        geometry: &PointerGeometry,
        boundary_plan: &PointerBoundaryPlan,
        parent: Option<TraceSequence>,
    ) -> Option<TraceSequence> {
        if !self.trace.is_enabled() {
            return parent;
        }
        let surface = geometry
            .snapshot
            .map(|snapshot| TraceSurfaceContext::accepted(work.event.surface_context(), snapshot));
        let transition = TraceTargetTransition::new(
            boundary_plan
                .previous_target
                .as_ref()
                .map(|target| self.tree.trace_target(target)),
            boundary_plan
                .current_target
                .as_ref()
                .map(|target| self.tree.trace_target(target)),
        );
        let context = TraceContext::pointer_boundary_plan(
            surface,
            Self::pointer_trace_context(work),
            self.trace_pointer_path(&boundary_plan.previous_path),
            transition,
        );
        self.trace.record_draft(
            TraceRecordDraft::pointer_fact(
                TraceRecordKind::PointerBoundaryBundlePlanned {
                    notifications: boundary_plan.notifications.len(),
                },
                work.instant,
                context,
            )
            .with_work_sequence(Some(work.sequence))
            .with_causal_parent(parent)
            .with_target(
                boundary_plan
                    .current_target
                    .as_ref()
                    .map(|target| self.tree.trace_target(target)),
            ),
        )
    }

    pub(super) fn record_pointer_prelude(
        &mut self,
        work: &PointerWork,
        is_new: bool,
        geometry: &PointerGeometry,
        boundary_plan: &PointerBoundaryPlan,
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
        let mut parent = self.trace.record_reserved(
            work.trace_reservation,
            TraceRecordKind::PointerIngressValidated {
                pointer_id,
                phase: work.event.phase(),
            },
            work.sequence,
            work.causal_parent,
        );
        parent = self.trace.record(
            TraceRecordKind::PointerStreamResolved {
                pointer_id,
                new_stream: is_new,
            },
            Some(work.sequence),
            parent,
            None,
            None,
            None,
        );
        if !is_new {
            parent = self.trace.record(
                TraceRecordKind::PointerStreamObserved { pointer_id },
                Some(work.sequence),
                parent,
                None,
                None,
                None,
            );
        }
        if let Some(outcome) = geometry.diagnosis {
            parent = self.trace.record(
                TraceRecordKind::PointerContextUnavailable {
                    pointer_id,
                    outcome,
                },
                Some(work.sequence),
                parent,
                None,
                None,
                None,
            );
        }
        if geometry.snapshot.is_none() && is_new {
            unreachable!("cancel requires an existing pointer stream")
        }
        parent = self.record_physical_pointer_observation(work, geometry, parent);
        parent = self.record_pointer_boundary_plan(work, geometry, boundary_plan, parent);
        Ok(parent)
    }
}
