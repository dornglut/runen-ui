use runenui_core::{SemanticActionRequest, UiApp};

use super::AppRuntime;
use crate::{CommandSubmission, SubmitSemanticActionError};

impl<App: UiApp> AppRuntime<App> {
    /// Appends one exact current-surface semantic-node action to the canonical FIFO.
    ///
    /// Submission performs no widget callback and never exposes the private
    /// mounted owner. The accepted request later enters the existing routed
    /// semantic-command/default path when the runtime is pumped.
    ///
    /// # Errors
    ///
    /// Returns the exact unaccepted request when surface/semantic authority,
    /// support/readiness, runtime status, queue capacity, or sequence admission
    /// rejects it.
    pub fn submit_semantic_action(
        &mut self,
        request: SemanticActionRequest,
    ) -> Result<CommandSubmission, SubmitSemanticActionError> {
        self.runtime.submit_semantic_action(request)
    }
}
