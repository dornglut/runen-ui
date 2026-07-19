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
