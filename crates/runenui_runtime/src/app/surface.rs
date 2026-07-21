use runenui_core::{CommandOrigin, SemanticCommand, SurfaceInputContext};

use crate::{CommandSubmission, LogicalPoint, MountedNodeId, SubmitSurfaceCommandError};

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

    #[must_use]
    pub fn publish_surface(&mut self, context: &SurfaceBuildContext<'_>) -> SurfacePublication {
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
