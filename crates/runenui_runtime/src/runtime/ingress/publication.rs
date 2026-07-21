use super::{HostProtocol, Runtime, RuntimeTerminalReason, TraceRecordKind};

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
        let redraw = self.take_redraw_request();
        let publication = self.surface_publication.publish(&mut self.tree, context);
        if let Some(redraw) = redraw {
            self.acknowledge_redraw(&redraw)
                .unwrap_or_else(|_| unreachable!("runtime-issued redraw request remains local"));
        }
        publication
    }

    pub(crate) fn note_surface_focus_validation(&mut self) {
        self.surface_publication.note_focus_validation();
    }

    pub(crate) const fn last_surface_phase_report(&self) -> &crate::SurfacePhaseReport {
        self.surface_publication.phase_report()
    }
}
