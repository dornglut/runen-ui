use runenui_core::{
    CommandOrigin, SemanticActionRequest, SemanticCommand, SurfaceInputContext,
};

use crate::{
    CommandSubmission, LogicalPoint, MountedNodeId, PublishSurfaceError, SubmitSemanticActionError,
    SubmitSurfaceCommandError,
};

use super::{
    AppRuntime, FocusState, ReconciliationReport, SurfaceBuildContext, SurfacePublication, Trace,
    UiApp,
};

impl<App: UiApp> AppRuntime<App> {
    #[must_use]
    pub const fn focus(&self) -> &FocusState {
        self.runtime.focus()
    }
    #[must_use]
    pub const fn state(&self) -> &App::State {
        self.runtime.state()
    }
    #[must_use]
    pub const fn trace(&self) -> &Trace {
        self.runtime.trace()
    }
    #[must_use]
    pub const fn reconciliation_report(&self) -> &ReconciliationReport {
        self.runtime.report()
    }

    /// Appends one exact current-surface semantic-node action to the canonical FIFO.
    ///
    /// Submission invokes no widget callback and exposes no mounted-owner routing
    /// shortcut. Accepted work later enters the existing routed semantic-command
    /// and default path when the runtime is pumped.
    ///
    /// # Errors
    ///
    /// Returns the exact unaccepted request when current surface/semantic
    /// authority, support/readiness, runtime status, canonical queue, work
    /// sequence, or enabled trace cannot admit the action.
    pub fn submit_semantic_action(
        &mut self,
        request: SemanticActionRequest,
    ) -> Result<CommandSubmission, SubmitSemanticActionError> {
        self.runtime.submit_semantic_action(request)
    }

    /// Resolves one logical point against an exact displayed snapshot and queues
    /// the semantic command for the resolved mounted target.
    ///
    /// # Errors
    ///
    /// Returns the exact unaccepted request when the context, snapshot, target,
    /// runtime status, canonical queue, work sequence, or enabled trace cannot
    /// accept the command.
    pub fn submit_surface_command(
        &mut self,
        context: SurfaceInputContext,
        point: LogicalPoint,
        command: SemanticCommand,
        origin: CommandOrigin,
    ) -> Result<CommandSubmission, SubmitSurfaceCommandError> {
        self.runtime
            .submit_surface_command(context, point, command, origin)
    }

    /// Validates a low-level resolved target against an exact displayed snapshot
    /// before queuing the semantic command through canonical ingress.
    ///
    /// # Errors
    ///
    /// Returns the exact unaccepted request when the context, snapshot
    /// membership, target lifetime, runtime status, canonical queue, work
    /// sequence, or enabled trace cannot accept the command.
    pub fn submit_resolved_surface_command(
        &mut self,
        context: SurfaceInputContext,
        target: MountedNodeId,
        command: SemanticCommand,
        origin: CommandOrigin,
    ) -> Result<CommandSubmission, SubmitSurfaceCommandError> {
        self.runtime
            .submit_resolved_surface_command(context, target, command, origin)
    }

    /// Publishes the current logical surface after all knowable runtime admission
    /// requirements have been proven.
    ///
    /// # Errors
    ///
    /// Returns [`PublishSurfaceError::Full`] for recoverable stationary-pointer
    /// re-hit backpressure without committing a new publication. Closed and
    /// terminal runtimes return their exact status instead of invoking widget
    /// capability callbacks.
    pub fn publish_surface(
        &mut self,
        context: &SurfaceBuildContext<'_>,
    ) -> Result<SurfacePublication, PublishSurfaceError> {
        self.runtime.publish_surface(context)
    }
    #[must_use]
    pub const fn last_surface_phase_report(&self) -> &crate::SurfacePhaseReport {
        self.runtime.last_surface_phase_report()
    }
    #[must_use]
    pub fn into_state(self) -> App::State {
        self.runtime.into_state()
    }
}
