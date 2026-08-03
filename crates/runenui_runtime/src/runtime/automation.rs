use runenui_core::{ElementId, HostProtocol, SemanticCommand};

use super::Runtime;

impl<State, Action, Protocol: HostProtocol> Runtime<State, Action, Protocol> {
    /// Runs public authored-ID automation ingress under its accepted rejection
    /// policy: sequence exhaustion returns the exact request and leaves the
    /// runtime running. Resolution and command commit still use the existing
    /// canonical authorities in `input` and `ingress`.
    pub(crate) fn submit_public_automation_command(
        &mut self,
        authored_id: ElementId,
        command: SemanticCommand,
    ) -> Result<crate::AutomationSubmission, crate::SubmitAutomationError> {
        debug_assert!(
            !self.automation_rejection_is_inert,
            "automation submission scopes cannot nest"
        );
        self.automation_rejection_is_inert = true;
        let result = self.submit_automation_command(authored_id, command);
        self.automation_rejection_is_inert = false;
        result
    }
}
