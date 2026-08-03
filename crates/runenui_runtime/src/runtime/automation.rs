use runenui_core::{ElementId, HostProtocol, SemanticCommand};

use super::Runtime;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum AutomationSubmissionPolicy {
    Ordinary,
    InertRejection,
}

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
        debug_assert_eq!(
            self.automation_submission_policy,
            AutomationSubmissionPolicy::Ordinary,
            "automation submission scopes cannot nest"
        );
        self.automation_submission_policy = AutomationSubmissionPolicy::InertRejection;
        let result = self.submit_automation_command(authored_id, command);
        self.automation_submission_policy = AutomationSubmissionPolicy::Ordinary;
        result
    }
}
