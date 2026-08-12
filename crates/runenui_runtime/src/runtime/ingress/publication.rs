use core::mem;

use runenui_core::MonotonicInstant;

use super::{
    HostProtocol, MandatoryTracePlan, QueueCommitError, Runtime, RuntimeStatus,
    RuntimeTerminalReason, TraceRecordKind, TraceSequence,
};
use crate::runtime::surface_publication::SurfacePublicationAdmission;
use crate::{
    PublishSurfaceError, SurfacePublicationCounter, TracePublicationContext, TraceSurfaceContext,
    TraceSurfaceSnapshotKind,
    surface::SurfacePlanningError,
    trace::{TraceRecordDraft, TraceReservation},
};

struct PublicationAdmission {
    surface: SurfacePublicationAdmission,
    stationary_rehit: bool,
}

impl<State, Action, Protocol: HostProtocol> Runtime<State, Action, Protocol> {
    pub(crate) fn request_redraw(
        &mut self,
        causal_parent: Option<TraceSequence>,
        instant: MonotonicInstant,
    ) {
        if !self.trace.can_admit(MandatoryTracePlan::one_fact()) {
            self.enter_terminal(RuntimeTerminalReason::TraceSequenceExhausted, 0);
            return;
        }
        let Some(next) = self.surface_publication.request_redraw() else {
            self.enter_terminal(
                RuntimeTerminalReason::SurfacePublicationCounterExhausted(
                    SurfacePublicationCounter::RedrawRevision,
                ),
                0,
            );
            return;
        };
        let requested = if self.trace.is_enabled() {
            self.trace.record_draft(
                TraceRecordDraft::redraw_fact(
                    TraceRecordKind::RedrawRequested { revision: next },
                    instant,
                )
                .with_causal_parent(causal_parent),
            )
        } else {
            None
        };
        if self.trace.is_enabled() && requested.is_none() {
            self.enter_terminal(RuntimeTerminalReason::TraceSequenceExhausted, 0);
            return;
        }
        self.surface_trace.note_request(next, requested);
    }

    pub(crate) fn take_redraw_request(&mut self) -> Option<crate::RedrawRequest> {
        let instant = self.now();
        self.take_redraw_request_at(instant)
    }

    fn take_redraw_request_at(
        &mut self,
        instant: MonotonicInstant,
    ) -> Option<crate::RedrawRequest> {
        let mut request = self.surface_publication.take_redraw_request()?;
        let request_parent = self.surface_trace.request_parent(request.revision());
        request.bind_request_trace(request_parent);
        let taken = if self.trace.is_enabled() {
            self.trace.record_draft(
                TraceRecordDraft::redraw_fact(
                    TraceRecordKind::RedrawTaken {
                        revision: request.revision(),
                    },
                    instant,
                )
                .with_causal_parent(request_parent),
            )
        } else {
            None
        };
        request.bind_taken_trace(taken);
        Some(request)
    }

    pub(crate) fn acknowledge_redraw(
        &mut self,
        request: &crate::RedrawRequest,
    ) -> Result<(), crate::RedrawAcknowledgeError> {
        let instant = self.now();
        self.acknowledge_redraw_at(request, request.control_parent(), instant)
    }

    fn acknowledge_redraw_at(
        &mut self,
        request: &crate::RedrawRequest,
        causal_parent: Option<TraceSequence>,
        instant: MonotonicInstant,
    ) -> Result<(), crate::RedrawAcknowledgeError> {
        self.surface_publication.acknowledge_redraw(request)?;
        if self.trace.is_enabled() {
            self.trace.record_draft(
                TraceRecordDraft::redraw_fact(
                    TraceRecordKind::RedrawAcknowledged {
                        revision: request.revision(),
                    },
                    instant,
                )
                .with_causal_parent(causal_parent),
            );
        }
        self.surface_trace
            .clear_if_acknowledged(request.revision(), self.surface_publication.is_dirty());
        Ok(())
    }

    pub(crate) fn publish_surface(
        &mut self,
        context: &crate::SurfaceBuildContext<'_>,
    ) -> Result<crate::SurfacePublication, PublishSurfaceError> {
        let admission = self.admit_surface_publication()?;
        let instant = self.now();
        let publication =
            match self
                .surface_publication
                .publish(&mut self.tree, context, admission.surface)
            {
                Ok(publication) => publication,
                Err(SurfacePlanningError::SemanticIntegrity) => {
                    let reason = RuntimeTerminalReason::Poisoned;
                    self.enter_terminal(reason, 0);
                    return Err(PublishSurfaceError::Terminal(reason));
                }
            };
        let redraw = self.take_redraw_request_at(instant);
        let publication_reservation = mem::replace(
            &mut self.surface_trace.publication_reservation,
            TraceReservation::continuation(),
        );
        let published = self.record_surface_publication(
            publication_reservation,
            redraw
                .as_ref()
                .and_then(crate::RedrawRequest::request_parent),
            instant,
            &publication,
        );
        if admission.stationary_rehit {
            self.commit_stationary_pointer_rehit(&publication, instant, published);
        }
        if let Some(redraw) = redraw {
            self.acknowledge_redraw_at(&redraw, published, instant)
                .unwrap_or_else(|_| unreachable!("runtime-issued redraw request remains local"));
        }
        self.replenish_surface_publication_reservation();
        Ok(publication)
    }

    fn admit_surface_publication(&mut self) -> Result<PublicationAdmission, PublishSurfaceError> {
        match self.status {
            RuntimeStatus::Running => {}
            RuntimeStatus::Terminal(reason) => return Err(PublishSurfaceError::Terminal(reason)),
            RuntimeStatus::Closed => return Err(PublishSurfaceError::Closed),
        }

        let surface = self
            .surface_publication
            .admit_publication()
            .map_err(|counter| {
                let reason = RuntimeTerminalReason::SurfacePublicationCounterExhausted(counter);
                self.enter_terminal(reason, 0);
                PublishSurfaceError::Terminal(reason)
            })?;

        let stationary_rehit =
            self.pointer_registry.has_streams() || self.queue.has_pointer_envelopes();
        if stationary_rehit {
            match self.queue.preflight_commit(1) {
                Ok(()) => {}
                Err(QueueCommitError::Full) => return Err(PublishSurfaceError::Full),
                Err(QueueCommitError::SequenceExhausted) => {
                    let reason = RuntimeTerminalReason::WorkSequenceExhausted;
                    self.enter_terminal(reason, 0);
                    return Err(PublishSurfaceError::Terminal(reason));
                }
            }
        }

        let trace_plan = MandatoryTracePlan::surface_publication(
            self.surface_publication.is_dirty(),
            stationary_rehit,
        );
        if !self
            .trace
            .can_replace_reservation(self.surface_trace.publication_reservation, trace_plan)
        {
            let reason = RuntimeTerminalReason::TraceSequenceExhausted;
            self.enter_terminal(reason, 0);
            return Err(PublishSurfaceError::Terminal(reason));
        }

        Ok(PublicationAdmission {
            surface,
            stationary_rehit,
        })
    }

    fn commit_stationary_pointer_rehit(
        &mut self,
        publication: &crate::SurfacePublication,
        instant: MonotonicInstant,
        published: Option<TraceSequence>,
    ) {
        let input_context = publication.input_context();
        let work_sequence = self
            .queue
            .next_sequence()
            .unwrap_or_else(|| unreachable!("stationary re-hit work sequence was preflighted"));
        let causal_parent = if self.trace.is_enabled() {
            self.trace.record(
                TraceRecordKind::PointerStationaryRehitQueued {
                    hit_test_generation: input_context.hit_test_generation(),
                    coordinate_revision: input_context.coordinate_revision(),
                },
                Some(work_sequence),
                published,
                None,
                None,
                None,
            )
        } else {
            None
        };
        let committed = self.queue.push_pointer_rehit_preflighted(
            input_context.clone(),
            instant,
            causal_parent,
            TraceReservation::continuation(),
        );
        match committed {
            Ok(_) => self.external_queue_commit_accepted(),
            Err(QueueCommitError::Full | QueueCommitError::SequenceExhausted) => {
                unreachable!("stationary pointer re-hit queue admission was preflighted")
            }
        }
    }

    fn record_surface_publication(
        &mut self,
        reservation: TraceReservation,
        causal_parent: Option<TraceSequence>,
        instant: MonotonicInstant,
        publication: &crate::SurfacePublication,
    ) -> Option<TraceSequence> {
        if !self.trace.is_enabled() {
            self.trace.release_reservation(reservation);
            return None;
        }
        let input_context = publication.input_context();
        let publication_context = TracePublicationContext::new(
            TraceSurfaceContext::accepted(input_context, TraceSurfaceSnapshotKind::Current),
            self.report.generation(),
            publication.frame().nodes().len(),
            self.surface_publication.phase_report().executed().to_vec(),
        );
        let published = self.trace.record_reserved_draft(
            reservation,
            TraceRecordDraft::publication_fact(
                TraceRecordKind::SurfacePublished,
                instant,
                publication_context,
            )
            .with_causal_parent(causal_parent),
        );
        if published.is_none() {
            self.enter_terminal(RuntimeTerminalReason::TraceSequenceExhausted, 0);
        }
        published
    }

    fn replenish_surface_publication_reservation(&mut self) {
        let Some(reservation) = self.trace.reserve_surface_publication() else {
            self.enter_terminal(RuntimeTerminalReason::TraceSequenceExhausted, 0);
            return;
        };
        self.surface_trace.publication_reservation = reservation;
    }

    pub(crate) fn note_surface_focus_validation(&mut self) {
        self.surface_publication.note_focus_validation();
    }

    pub(crate) const fn last_surface_phase_report(&self) -> &crate::SurfacePhaseReport {
        self.surface_publication.phase_report()
    }
}
