use runenui_core::{
    Focusability, SemanticAction, SemanticActionData, SemanticActionRequest, SemanticActionTarget,
    SemanticCommand, SemanticKey, SemanticNodeId, SurfaceId, TextSensitivity,
};

use crate::{
    CommandSubmission, MountedNodeId, SubmitCommandErrorKind, SubmitSemanticActionError,
    SubmitSemanticActionErrorKind, TraceSemanticActionRejection,
    mounted::{DirtyPhases, SemanticActionAuthority, SemanticActionAuthorityError},
};

use super::{HostProtocol, Runtime, RuntimeStatus};
use crate::runtime::surface_publication::SurfaceIdentityError;

impl<State, Action, Protocol: HostProtocol> Runtime<State, Action, Protocol> {
    pub(crate) fn submit_semantic_action(
        &mut self,
        request: SemanticActionRequest,
    ) -> Result<CommandSubmission, SubmitSemanticActionError> {
        let authority = match self.semantic_action_preflight(
            request.surface_id(),
            request.target(),
            request.action(),
            request.data(),
            None,
        ) {
            Ok(authority) => authority,
            Err(kind) => return Err(SubmitSemanticActionError::new(kind, request)),
        };
        let owner = authority.owner().clone();
        let key = authority.key().clone();
        let rejected_request = request.clone();
        let (surface, target, action, data) = request.into_parts();
        let command = semantic_command(&action);
        let semantic_target =
            SemanticActionTarget::__runtime_new(surface, target, key, action, data);
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

    pub(in crate::runtime) fn revalidate_semantic_action_target(
        &self,
        target: &SemanticActionTarget,
    ) -> Result<MountedNodeId, SubmitSemanticActionErrorKind> {
        self.semantic_action_preflight(
            target.surface_id(),
            target.target(),
            target.action(),
            target.data(),
            Some(target.semantic_key()),
        )
        .map(|authority| authority.owner().clone())
    }

    fn semantic_action_preflight(
        &self,
        surface: &SurfaceId,
        target: &SemanticNodeId,
        action: &SemanticAction,
        data: Option<&SemanticActionData>,
        expected_key: Option<&SemanticKey>,
    ) -> Result<SemanticActionAuthority, SubmitSemanticActionErrorKind> {
        match self.status {
            RuntimeStatus::Running => {}
            RuntimeStatus::Closed => return Err(SubmitSemanticActionErrorKind::Closed),
            RuntimeStatus::Terminal(reason) => {
                return Err(SubmitSemanticActionErrorKind::Terminal(reason));
            }
        }
        self.surface_publication
            .validate_surface_id(surface)
            .map_err(|error| match error {
                SurfaceIdentityError::Foreign => SubmitSemanticActionErrorKind::ForeignSurface,
                SurfaceIdentityError::Wrong => SubmitSemanticActionErrorKind::WrongSurface,
            })?;
        let authority = self
            .tree
            .semantic_action_authority(target)
            .map_err(map_authority_error)?;
        if expected_key.is_some_and(|key| key != authority.key()) {
            return Err(SubmitSemanticActionErrorKind::Integrity);
        }
        if self.tree.pending_phases().contains(DirtyPhases::SEMANTICS) {
            return Err(SubmitSemanticActionErrorKind::StaleAuthority);
        }
        let publication = self
            .surface_publication
            .current_semantic_publication()
            .ok_or(SubmitSemanticActionErrorKind::StaleAuthority)?;
        let node = publication
            .snapshot()
            .node(target)
            .ok_or(SubmitSemanticActionErrorKind::TargetNotInSurface)?;
        let data_matches = matches!(
            (action, data),
            (
                SemanticAction::SetSelection,
                Some(SemanticActionData::Selection(_))
            ) | (
                SemanticAction::ReplaceSelection,
                Some(SemanticActionData::ReplacementText(_))
            )
        ) || (!matches!(
            action,
            SemanticAction::SetSelection | SemanticAction::ReplaceSelection
        ) && data.is_none());
        if !data_matches {
            return Err(SubmitSemanticActionErrorKind::UnsupportedAction);
        }
        if !node.supported_actions().contains(action) {
            return Err(SubmitSemanticActionErrorKind::UnsupportedAction);
        }
        let state = node.state();
        if state.disabled() || state.inert() {
            return Err(SubmitSemanticActionErrorKind::UnavailableAction);
        }
        if let Some(editable) = node.editable() {
            if let Some(SemanticActionData::Selection(selection)) = data {
                let offsets = editable
                    .caret_offsets()
                    .ok_or(SubmitSemanticActionErrorKind::UnavailableAction)?;
                if selection.anchor().snapshot() != editable.snapshot()
                    || selection.active().snapshot() != editable.snapshot()
                    || offsets
                        .binary_search(&selection.anchor().byte_offset())
                        .is_err()
                    || offsets
                        .binary_search(&selection.active().byte_offset())
                        .is_err()
                {
                    return Err(SubmitSemanticActionErrorKind::UnavailableAction);
                }
            }
            let modifying = matches!(
                action,
                SemanticAction::DeleteBackward
                    | SemanticAction::DeleteForward
                    | SemanticAction::Undo
                    | SemanticAction::Redo
                    | SemanticAction::Cut
                    | SemanticAction::Paste
                    | SemanticAction::ReplaceSelection
            );
            if modifying && (state.read_only() || editable.read_only()) {
                return Err(SubmitSemanticActionErrorKind::UnavailableAction);
            }
            if editable.sensitivity() == TextSensitivity::Secret
                && matches!(action, SemanticAction::Copy | SemanticAction::Cut)
            {
                return Err(SubmitSemanticActionErrorKind::UnavailableAction);
            }
        }
        if !semantic_action_is_ready(&authority, action) {
            return Err(SubmitSemanticActionErrorKind::UnavailableAction);
        }
        Ok(authority)
    }
}

fn semantic_action_is_ready(authority: &SemanticActionAuthority, action: &SemanticAction) -> bool {
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
                    _ => false,
                }
        }
        SemanticAction::OpenMenu | SemanticAction::OpenContextMenu => true,
        SemanticAction::MoveBackward
        | SemanticAction::MoveForward
        | SemanticAction::ExtendBackward
        | SemanticAction::ExtendForward
        | SemanticAction::SelectAll
        | SemanticAction::DeleteBackward
        | SemanticAction::DeleteForward
        | SemanticAction::Undo
        | SemanticAction::Redo
        | SemanticAction::Copy
        | SemanticAction::Cut
        | SemanticAction::Paste
        | SemanticAction::SetSelection
        | SemanticAction::ReplaceSelection => authority.key() == &SemanticKey::PRIMARY,
        _ => false,
    }
}

fn semantic_command(action: &SemanticAction) -> SemanticCommand {
    match action {
        SemanticAction::Activate => SemanticCommand::Activate,
        SemanticAction::RequestFocus => SemanticCommand::RequestFocus,
        SemanticAction::OpenMenu => SemanticCommand::OpenMenu,
        SemanticAction::OpenContextMenu => SemanticCommand::OpenContextMenu,
        SemanticAction::MoveBackward => SemanticCommand::MoveBackward,
        SemanticAction::MoveForward => SemanticCommand::MoveForward,
        SemanticAction::ExtendBackward => SemanticCommand::ExtendBackward,
        SemanticAction::ExtendForward => SemanticCommand::ExtendForward,
        SemanticAction::SelectAll => SemanticCommand::SelectAll,
        SemanticAction::DeleteBackward => SemanticCommand::DeleteBackward,
        SemanticAction::DeleteForward => SemanticCommand::DeleteForward,
        SemanticAction::Undo => SemanticCommand::Undo,
        SemanticAction::Redo => SemanticCommand::Redo,
        SemanticAction::Copy => SemanticCommand::Copy,
        SemanticAction::Cut => SemanticCommand::Cut,
        SemanticAction::Paste => SemanticCommand::Paste,
        SemanticAction::SetSelection => SemanticCommand::SetSelection,
        SemanticAction::ReplaceSelection => SemanticCommand::ReplaceSelection,
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

pub(in crate::runtime) const fn trace_semantic_action_rejection(
    kind: SubmitSemanticActionErrorKind,
) -> TraceSemanticActionRejection {
    match kind {
        SubmitSemanticActionErrorKind::Closed => TraceSemanticActionRejection::Closed,
        SubmitSemanticActionErrorKind::Terminal(_) => TraceSemanticActionRejection::Terminal,
        SubmitSemanticActionErrorKind::ForeignSurface => {
            TraceSemanticActionRejection::ForeignSurface
        }
        SubmitSemanticActionErrorKind::WrongSurface => TraceSemanticActionRejection::WrongSurface,
        SubmitSemanticActionErrorKind::ForeignTarget => TraceSemanticActionRejection::ForeignTarget,
        SubmitSemanticActionErrorKind::StaleTarget => TraceSemanticActionRejection::StaleTarget,
        SubmitSemanticActionErrorKind::MissingTarget => TraceSemanticActionRejection::MissingTarget,
        SubmitSemanticActionErrorKind::TargetNotInSurface => {
            TraceSemanticActionRejection::TargetNotInSurface
        }
        SubmitSemanticActionErrorKind::UnsupportedAction => {
            TraceSemanticActionRejection::UnsupportedAction
        }
        SubmitSemanticActionErrorKind::UnavailableAction => {
            TraceSemanticActionRejection::UnavailableAction
        }
        SubmitSemanticActionErrorKind::StaleAuthority => {
            TraceSemanticActionRejection::StaleAuthority
        }
        SubmitSemanticActionErrorKind::Integrity
        | SubmitSemanticActionErrorKind::Full
        | SubmitSemanticActionErrorKind::WorkSequenceExhausted
        | SubmitSemanticActionErrorKind::TraceSequenceExhausted => {
            TraceSemanticActionRejection::Integrity
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
