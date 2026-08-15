use runenui_core::{
    Focusability, SemanticAction, SemanticActionRequest, SemanticActionTarget, SemanticCommand,
    SemanticKey,
};

use crate::{
    CommandSubmission, SubmitCommandErrorKind, SubmitSemanticActionError,
    SubmitSemanticActionErrorKind,
    mounted::SemanticActionAuthorityError,
};

use super::{HostProtocol, Runtime, RuntimeStatus};
use crate::runtime::surface_publication::SurfaceIdentityError;

impl<State, Action, Protocol: HostProtocol> Runtime<State, Action, Protocol> {
    pub(crate) fn submit_semantic_action(
        &mut self,
        request: SemanticActionRequest,
    ) -> Result<CommandSubmission, SubmitSemanticActionError> {
        let (owner, key) = match self.semantic_action_preflight(&request) {
            Ok(authority) => (authority.owner().clone(), authority.key().clone()),
            Err(kind) => return Err(SubmitSemanticActionError::new(kind, request)),
        };
        let rejected_request = request.clone();
        let (surface, target, action) = request.into_parts();
        let command = semantic_command(&action);
        let semantic_target = SemanticActionTarget::__runtime_new(surface, target, key, action);
        match self.submit_semantic_action_command(&owner, command, semantic_target) {
            Ok(submission) => Ok(submission),
            Err(kind) => {
                let semantic_kind = map_command_rejection(kind);
                self.terminalize_command_failure(kind);
                Err(SubmitSemanticActionError::new(
                    semantic_kind,
                    rejected_request,
                ))
            }
        }
    }

    fn semantic_action_preflight(
        &self,
        request: &SemanticActionRequest,
    ) -> Result<crate::mounted::SemanticActionAuthority, SubmitSemanticActionErrorKind> {
        match self.status {
            RuntimeStatus::Running => {}
            RuntimeStatus::Closed => return Err(SubmitSemanticActionErrorKind::Closed),
            RuntimeStatus::Terminal(reason) => {
                return Err(SubmitSemanticActionErrorKind::Terminal(reason));
            }
        }
        self.surface_publication
            .validate_surface_id(request.surface_id())
            .map_err(|error| match error {
                SurfaceIdentityError::Foreign => SubmitSemanticActionErrorKind::ForeignSurface,
                SurfaceIdentityError::Wrong => SubmitSemanticActionErrorKind::WrongSurface,
            })?;
        let authority = self
            .tree
            .semantic_action_authority(request.target())
            .map_err(map_authority_error)?;
        let publication = self
            .surface_publication
            .current_semantic_publication()
            .ok_or(SubmitSemanticActionErrorKind::StaleAuthority)?;
        let node = publication
            .snapshot()
            .node(request.target())
            .ok_or(SubmitSemanticActionErrorKind::TargetNotInSurface)?;
        if !node.supported_actions().contains(request.action()) {
            return Err(SubmitSemanticActionErrorKind::UnsupportedAction);
        }
        let state = node.state();
        if state.disabled() || state.inert() {
            return Err(SubmitSemanticActionErrorKind::UnavailableAction);
        }
        if !semantic_action_is_ready(&authority, request.action()) {
            return Err(SubmitSemanticActionErrorKind::UnavailableAction);
        }
        Ok(authority)
    }
}

fn semantic_action_is_ready(
    authority: &crate::mounted::SemanticActionAuthority,
    action: &SemanticAction,
) -> bool {
    let activation = authority.activation();
    if !activation.enabled() {
        return false;
    }
    match action {
        SemanticAction::Activate => {
            authority.key() != &SemanticKey::PRIMARY || activation.is_actionable()
        }
        SemanticAction::RequestFocus => {
            authority.key() == &SemanticKey::PRIMARY
                && match authority.focusability() {
                    Focusability::Focusable => true,
                    Focusability::Automatic => activation.is_actionable(),
                    Focusability::Never | Focusability::Hidden => false,
                    _ => false,
                }
        }
        SemanticAction::OpenMenu | SemanticAction::OpenContextMenu => true,
        _ => false,
    }
}

const fn semantic_command(action: &SemanticAction) -> SemanticCommand {
    match action {
        SemanticAction::Activate => SemanticCommand::Activate,
        SemanticAction::RequestFocus => SemanticCommand::RequestFocus,
        SemanticAction::OpenMenu => SemanticCommand::OpenMenu,
        SemanticAction::OpenContextMenu => SemanticCommand::OpenContextMenu,
        _ => unreachable!("M5 semantic action vocabulary is closed by accepted authority"),
    }
}

const fn map_authority_error(error: SemanticActionAuthorityError) -> SubmitSemanticActionErrorKind {
    match error {
        SemanticActionAuthorityError::ForeignTarget => SubmitSemanticActionErrorKind::ForeignTarget,
        SemanticActionAuthorityError::StaleTarget => SubmitSemanticActionErrorKind::StaleTarget,
        SemanticActionAuthorityError::MissingTarget => SubmitSemanticActionErrorKind::MissingTarget,
        SemanticActionAuthorityError::MissingOwner | SemanticActionAuthorityError::Integrity => {
            SubmitSemanticActionErrorKind::Integrity
        }
        SemanticActionAuthorityError::StaleAuthority => {
            SubmitSemanticActionErrorKind::StaleAuthority
        }
    }
}

const fn map_command_rejection(kind: SubmitCommandErrorKind) -> SubmitSemanticActionErrorKind {
    match kind {
        SubmitCommandErrorKind::Full => SubmitSemanticActionErrorKind::Full,
        SubmitCommandErrorKind::Closed => SubmitSemanticActionErrorKind::Closed,
        SubmitCommandErrorKind::Terminal(reason) => SubmitSemanticActionErrorKind::Terminal(reason),
        SubmitCommandErrorKind::WorkSequenceExhausted => {
            SubmitSemanticActionErrorKind::WorkSequenceExhausted
        }
        SubmitCommandErrorKind::TraceSequenceExhausted => {
            SubmitSemanticActionErrorKind::TraceSequenceExhausted
        }
        SubmitCommandErrorKind::ForeignTarget
        | SubmitCommandErrorKind::StaleTarget
        | SubmitCommandErrorKind::MissingTarget => SubmitSemanticActionErrorKind::Integrity,
    }
}
