use super::{HostProtocol, QueueCommitError, Runtime, RuntimeTerminalReason, TraceRecordKind};

impl<State, Action, Protocol: HostProtocol> Runtime<State, Action, Protocol> {
    pub(crate) fn request_redraw(&mut self) {
        let Some(next) = self.surface_publication.request_redraw() else {
            self.enter_terminal(RuntimeTerminalReason::Poisoned, 0);
            return;
        };
        self.record_optional(
            TraceRecordKind::RedrawRequested { revision: next },
            None,
            None,
            None,
        );
    }

    pub(crate) fn take_redraw_request(&mut self) -> Option<crate::RedrawRequest> {
        let request = self.surface_publication.take_redraw_request();
        if let Some(request) = &request {
            self.record_optional(
                TraceRecordKind::RedrawTaken {
                    revision: request.revision,
                },
                None,
                None,
                None,
            );
        }
        request
    }

    pub(crate) fn acknowledge_redraw(
        &mut self,
        request: &crate::RedrawRequest,
    ) -> Result<(), crate::RedrawAcknowledgeError> {
        self.surface_publication.acknowledge_redraw(request)?;
        self.record_optional(
            TraceRecordKind::RedrawAcknowledged {
                revision: request.revision,
            },
            None,
            None,
            None,
        );
        Ok(())
    }

    pub(crate) fn publish_surface(
        &mut self,
        context: &crate::SurfaceBuildContext<'_>,
    ) -> crate::SurfacePublication {
        let rehit_reservation = self.prepare_stationary_pointer_rehit();
        let redraw = self.take_redraw_request();
        let publication = self.surface_publication.publish(&mut self.tree, context);
        if let Some(trace_reservation) = rehit_reservation {
            let instant = self.now();
            let input_context = publication.input_context();
            let causal_parent = self.trace.record_reserved(
                trace_reservation,
                TraceRecordKind::PointerStationaryRehitQueued {
                    hit_test_generation: input_context.hit_test_generation(),
                    coordinate_revision: input_context.coordinate_revision(),
                },
                self.queue
                    .next_sequence()
                    .unwrap_or_else(|| unreachable!("stationary re-hit was preflighted")),
                None,
            );
            let committed = self.queue.push_pointer_rehit_preflighted(
                input_context.clone(),
                instant,
                causal_parent,
                crate::trace::TraceReservation::continuation(),
            );
            match committed {
                Ok(_) => self.external_queue_commit_accepted(),
                Err(QueueCommitError::Full | QueueCommitError::SequenceExhausted) => {
                    unreachable!("stationary pointer re-hit queue admission was preflighted")
                }
            }
        }
        if let Some(redraw) = redraw {
            self.acknowledge_redraw(&redraw)
                .unwrap_or_else(|_| unreachable!("runtime-issued redraw request remains local"));
        }
        publication
    }

    fn prepare_stationary_pointer_rehit(&mut self) -> Option<crate::trace::TraceReservation> {
        let required = self.pointer_registry.has_streams() || self.queue.has_pointer_envelopes();
        if !required {
            return None;
        }
        if let Err(error) = self.queue.preflight_commit(1) {
            let reason = match error {
                QueueCommitError::Full => RuntimeTerminalReason::Poisoned,
                QueueCommitError::SequenceExhausted => RuntimeTerminalReason::WorkSequenceExhausted,
            };
            self.enter_terminal(reason, 0);
            return None;
        }
        let Some(reservation) = self.trace.reserve_pointer_outcome() else {
            self.enter_terminal(RuntimeTerminalReason::TraceSequenceExhausted, 0);
            return None;
        };
        Some(reservation)
    }

    pub(crate) fn note_surface_focus_validation(&mut self) {
        self.surface_publication.note_focus_validation();
    }

    pub(crate) const fn last_surface_phase_report(&self) -> &crate::SurfacePhaseReport {
        self.surface_publication.phase_report()
    }
}
