use runenui_core::{CommandOrigin, HostProtocol, SemanticCommand, SurfaceInputContext};

use crate::{
    CommandSubmission, LogicalPoint, MountedNodeId, SubmitCommandErrorKind,
    SubmitSurfaceCommandError, SubmitSurfaceCommandErrorKind, TraceRecordKind, TraceSurfaceContext,
    TraceSurfaceIngressKind, TraceSurfaceRejection, TraceSurfaceSnapshotKind,
    UnacceptedSurfaceCommand,
    runtime::{
        Runtime,
        surface_publication::{SurfaceSnapshotError, SurfaceSnapshotKind},
    },
    trace::TraceRecordDraft,
};

use super::submission::SurfaceCommandTrace;

impl<State, Action, Protocol: HostProtocol> Runtime<State, Action, Protocol> {
    pub(crate) fn submit_surface_command(
        &mut self,
        context: SurfaceInputContext,
        point: LogicalPoint,
        command: SemanticCommand,
        origin: CommandOrigin,
    ) -> Result<CommandSubmission, SubmitSurfaceCommandError> {
        let ingress = TraceSurfaceIngressKind::LogicalCoordinate;
        if let Err(kind) = self.command_status_preflight() {
            let surface_kind = map_command_error(kind);
            self.record_surface_command_rejection(ingress, surface_kind, &context);
            return Err(Self::reject_logical_surface_command(
                surface_kind,
                context,
                point,
                command,
                origin,
            ));
        }
        let resolution = match self.surface_publication.resolve_point(&context, point) {
            Ok(resolution) => resolution,
            Err(error) => {
                let kind = map_snapshot_error(error);
                self.record_surface_command_rejection(ingress, kind, &context);
                return Err(Self::reject_logical_surface_command(
                    kind, context, point, command, origin,
                ));
            }
        };
        let trace = SurfaceCommandTrace::new(
            ingress,
            TraceSurfaceContext::accepted(&context, map_snapshot_kind(resolution.snapshot_kind())),
        );
        let target = resolution.into_target();
        match self.submit_surface_bound_command(&target, command, origin, trace) {
            Ok(submission) => Ok(submission),
            Err(kind) => {
                let surface_kind = map_command_error(kind);
                self.record_surface_command_rejection(ingress, surface_kind, &context);
                let error = Self::reject_logical_surface_command(
                    surface_kind,
                    context,
                    point,
                    command,
                    origin,
                );
                self.terminalize_command_failure(kind);
                Err(error)
            }
        }
    }

    pub(crate) fn submit_resolved_surface_command(
        &mut self,
        context: SurfaceInputContext,
        target: MountedNodeId,
        command: SemanticCommand,
        origin: CommandOrigin,
    ) -> Result<CommandSubmission, SubmitSurfaceCommandError> {
        let ingress = TraceSurfaceIngressKind::ResolvedTarget;
        if let Err(kind) = self.command_status_preflight() {
            let surface_kind = map_command_error(kind);
            self.record_surface_command_rejection(ingress, surface_kind, &context);
            return Err(Self::reject_resolved_surface_command(
                surface_kind,
                context,
                target,
                command,
                origin,
            ));
        }
        let selection = match self
            .surface_publication
            .validate_resolved_target(&context, &target)
        {
            Ok(selection) => selection,
            Err(error) => {
                let kind = map_snapshot_error(error);
                self.record_surface_command_rejection(ingress, kind, &context);
                return Err(Self::reject_resolved_surface_command(
                    kind, context, target, command, origin,
                ));
            }
        };
        let trace = SurfaceCommandTrace::new(
            ingress,
            TraceSurfaceContext::accepted(&context, map_snapshot_kind(selection.snapshot_kind())),
        );
        match self.submit_surface_bound_command(&target, command, origin, trace) {
            Ok(submission) => Ok(submission),
            Err(kind) => {
                let surface_kind = map_command_error(kind);
                self.record_surface_command_rejection(ingress, surface_kind, &context);
                let error = Self::reject_resolved_surface_command(
                    surface_kind,
                    context,
                    target,
                    command,
                    origin,
                );
                self.terminalize_command_failure(kind);
                Err(error)
            }
        }
    }

    fn record_surface_command_rejection(
        &mut self,
        ingress: TraceSurfaceIngressKind,
        kind: SubmitSurfaceCommandErrorKind,
        context: &SurfaceInputContext,
    ) {
        if !self.trace.is_enabled() {
            return;
        }
        let instant = self.now();
        self.trace.record_draft(TraceRecordDraft::surface_fact(
            TraceRecordKind::SurfaceCommandRejected {
                ingress,
                outcome: map_trace_rejection(kind),
            },
            instant,
            TraceSurfaceContext::requested(context),
        ));
    }

    const fn reject_logical_surface_command(
        kind: SubmitSurfaceCommandErrorKind,
        context: SurfaceInputContext,
        point: LogicalPoint,
        command: SemanticCommand,
        origin: CommandOrigin,
    ) -> SubmitSurfaceCommandError {
        SubmitSurfaceCommandError::new(
            kind,
            UnacceptedSurfaceCommand::Logical {
                context,
                point,
                command,
                origin,
            },
        )
    }

    const fn reject_resolved_surface_command(
        kind: SubmitSurfaceCommandErrorKind,
        context: SurfaceInputContext,
        target: MountedNodeId,
        command: SemanticCommand,
        origin: CommandOrigin,
    ) -> SubmitSurfaceCommandError {
        SubmitSurfaceCommandError::new(
            kind,
            UnacceptedSurfaceCommand::Resolved {
                context,
                target,
                command,
                origin,
            },
        )
    }
}

const fn map_snapshot_kind(kind: SurfaceSnapshotKind) -> TraceSurfaceSnapshotKind {
    match kind {
        SurfaceSnapshotKind::Current => TraceSurfaceSnapshotKind::Current,
        SurfaceSnapshotKind::Retained => TraceSurfaceSnapshotKind::RetainedHistorical,
    }
}

const fn map_snapshot_error(error: SurfaceSnapshotError) -> SubmitSurfaceCommandErrorKind {
    match error {
        SurfaceSnapshotError::ForeignSurfaceContext => {
            SubmitSurfaceCommandErrorKind::ForeignSurfaceContext
        }
        SurfaceSnapshotError::ForeignSurface => SubmitSurfaceCommandErrorKind::ForeignSurface,
        SurfaceSnapshotError::RetiredSurfaceContext => {
            SubmitSurfaceCommandErrorKind::RetiredSurfaceContext
        }
        SurfaceSnapshotError::MissingSurfaceGeneration => {
            SubmitSurfaceCommandErrorKind::MissingSurfaceGeneration
        }
        SurfaceSnapshotError::CoordinateRevisionMismatch => {
            SubmitSurfaceCommandErrorKind::CoordinateRevisionMismatch
        }
        SurfaceSnapshotError::NoTarget => SubmitSurfaceCommandErrorKind::NoTarget,
        SurfaceSnapshotError::TargetNotInSnapshot => {
            SubmitSurfaceCommandErrorKind::TargetNotInSnapshot
        }
    }
}

const fn map_command_error(error: SubmitCommandErrorKind) -> SubmitSurfaceCommandErrorKind {
    match error {
        SubmitCommandErrorKind::Full => SubmitSurfaceCommandErrorKind::Full,
        SubmitCommandErrorKind::Closed => SubmitSurfaceCommandErrorKind::Closed,
        SubmitCommandErrorKind::Terminal(reason) => SubmitSurfaceCommandErrorKind::Terminal(reason),
        SubmitCommandErrorKind::ForeignTarget => SubmitSurfaceCommandErrorKind::ForeignTarget,
        SubmitCommandErrorKind::StaleTarget => SubmitSurfaceCommandErrorKind::StaleTarget,
        SubmitCommandErrorKind::MissingTarget => SubmitSurfaceCommandErrorKind::MissingTarget,
        SubmitCommandErrorKind::WorkSequenceExhausted => {
            SubmitSurfaceCommandErrorKind::WorkSequenceExhausted
        }
        SubmitCommandErrorKind::TraceSequenceExhausted => {
            SubmitSurfaceCommandErrorKind::TraceSequenceExhausted
        }
    }
}

const fn map_trace_rejection(kind: SubmitSurfaceCommandErrorKind) -> TraceSurfaceRejection {
    match kind {
        SubmitSurfaceCommandErrorKind::Full => TraceSurfaceRejection::QueueFull,
        SubmitSurfaceCommandErrorKind::Closed => TraceSurfaceRejection::RuntimeClosed,
        SubmitSurfaceCommandErrorKind::Terminal(_) => TraceSurfaceRejection::RuntimeTerminal,
        SubmitSurfaceCommandErrorKind::ForeignSurfaceContext => {
            TraceSurfaceRejection::ForeignRuntime
        }
        SubmitSurfaceCommandErrorKind::ForeignSurface => TraceSurfaceRejection::ForeignSurface,
        SubmitSurfaceCommandErrorKind::RetiredSurfaceContext => {
            TraceSurfaceRejection::RetiredGeneration
        }
        SubmitSurfaceCommandErrorKind::MissingSurfaceGeneration => {
            TraceSurfaceRejection::MissingGeneration
        }
        SubmitSurfaceCommandErrorKind::CoordinateRevisionMismatch => {
            TraceSurfaceRejection::CoordinateRevisionMismatch
        }
        SubmitSurfaceCommandErrorKind::NoTarget => TraceSurfaceRejection::NoTarget,
        SubmitSurfaceCommandErrorKind::TargetNotInSnapshot => {
            TraceSurfaceRejection::TargetNotInSnapshot
        }
        SubmitSurfaceCommandErrorKind::ForeignTarget => TraceSurfaceRejection::ForeignTarget,
        SubmitSurfaceCommandErrorKind::StaleTarget => TraceSurfaceRejection::StaleTarget,
        SubmitSurfaceCommandErrorKind::MissingTarget => TraceSurfaceRejection::MissingTarget,
        SubmitSurfaceCommandErrorKind::WorkSequenceExhausted => {
            TraceSurfaceRejection::WorkSequenceExhausted
        }
        SubmitSurfaceCommandErrorKind::TraceSequenceExhausted => {
            TraceSurfaceRejection::TraceSequenceExhausted
        }
    }
}
