use runenui_core::{
    Focusability, SemanticAction, SemanticActionData, SemanticActionRequest, SemanticActionTarget,
    SemanticCommand, SemanticKey, SemanticNodeId, SemanticRole, SurfaceId, TextSensitivity,
};

use crate::{
    CommandSubmission, MountedNodeId, SubmitCommandErrorKind, SubmitSemanticActionError,
    SubmitSemanticActionErrorKind, TraceSemanticActionRejection,
    mounted::{DirtyPhases, SemanticActionAuthority, SemanticActionAuthorityError},
};

use super::{HostProtocol, Runtime, RuntimeStatus};
use crate::focus::focusability_is_eligible;
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
        let command = semantic_command(request.action(), request.data()).unwrap_or_else(|| {
            unreachable!("semantic preflight validates action/data normalization")
        });
        let rejected_request = request.clone();
        let (surface, target, action, data) = request.into_parts();
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
        let authority = self.semantic_action_authority(surface, target, expected_key)?;
        let publication = self
            .surface_publication
            .current_semantic_publication()
            .ok_or(SubmitSemanticActionErrorKind::StaleAuthority)?;
        let node = publication
            .snapshot()
            .node(target)
            .ok_or(SubmitSemanticActionErrorKind::TargetNotInSurface)?;
        validate_semantic_action_data(action, data)?;
        if !node.supported_actions().contains(action) {
            return Err(SubmitSemanticActionErrorKind::UnsupportedAction);
        }
        let state = node.state();
        let disabled_focus_request = *action == SemanticAction::RequestFocus
            && authority.key() == &SemanticKey::PRIMARY
            && authority.focusability() == Focusability::FocusableWhenDisabled;
        if state.inert() || (state.disabled() && !disabled_focus_request) {
            return Err(SubmitSemanticActionErrorKind::UnavailableAction);
        }
        validate_m11_semantic_action(node, action, data)?;
        validate_editable_semantic_action(node, action, data)?;
        let automatic_scroll_focusable =
            self.current_automatic_scroll_focusability(authority.owner());
        if !semantic_action_is_ready(&authority, action, automatic_scroll_focusable) {
            return Err(SubmitSemanticActionErrorKind::UnavailableAction);
        }
        Ok(authority)
    }

    fn semantic_action_authority(
        &self,
        surface: &SurfaceId,
        target: &SemanticNodeId,
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
        Ok(authority)
    }
}

fn validate_semantic_action_data(
    action: &SemanticAction,
    data: Option<&SemanticActionData>,
) -> Result<(), SubmitSemanticActionErrorKind> {
    let valid = matches!(
        (action, data),
        (
            SemanticAction::SetSelection,
            Some(SemanticActionData::Selection(_))
        ) | (
            SemanticAction::ReplaceSelection,
            Some(SemanticActionData::ReplacementText(_))
        ) | (
            SemanticAction::SetValue,
            Some(SemanticActionData::NumericValue(_))
        )
    ) || (!matches!(
        action,
        SemanticAction::SetSelection | SemanticAction::ReplaceSelection | SemanticAction::SetValue
    ) && data.is_none());
    valid
        .then_some(())
        .ok_or(SubmitSemanticActionErrorKind::UnsupportedAction)
}

fn validate_m11_semantic_action(
    node: &crate::SemanticNode,
    action: &SemanticAction,
    data: Option<&SemanticActionData>,
) -> Result<(), SubmitSemanticActionErrorKind> {
    let state = node.state();
    match action {
        SemanticAction::Increment | SemanticAction::Decrement => {
            if !is_mutable_range_role(node.role()) || node.range().is_none() {
                return Err(SubmitSemanticActionErrorKind::UnsupportedAction);
            }
            if state.read_only()
                || node
                    .range()
                    .and_then(runenui_core::SemanticRange::current)
                    .is_none()
            {
                return Err(SubmitSemanticActionErrorKind::UnavailableAction);
            }
        }
        SemanticAction::SetValue => {
            if !is_mutable_range_role(node.role()) {
                return Err(SubmitSemanticActionErrorKind::UnsupportedAction);
            }
            if state.read_only() {
                return Err(SubmitSemanticActionErrorKind::UnavailableAction);
            }
            let Some(range) = node.range() else {
                return Err(SubmitSemanticActionErrorKind::UnsupportedAction);
            };
            let Some(SemanticActionData::NumericValue(value)) = data else {
                return Err(SubmitSemanticActionErrorKind::UnsupportedAction);
            };
            if range
                .minimum()
                .is_some_and(|minimum| value.get() < minimum.get())
                || range
                    .maximum()
                    .is_some_and(|maximum| value.get() > maximum.get())
            {
                return Err(SubmitSemanticActionErrorKind::UnavailableAction);
            }
        }
        SemanticAction::Expand => {
            if !is_expandable_role(node.role()) {
                return Err(SubmitSemanticActionErrorKind::UnsupportedAction);
            }
            if state.expanded() != Some(false) {
                return Err(SubmitSemanticActionErrorKind::UnavailableAction);
            }
        }
        SemanticAction::Collapse => {
            if !is_expandable_role(node.role()) {
                return Err(SubmitSemanticActionErrorKind::UnsupportedAction);
            }
            if state.expanded() != Some(true) {
                return Err(SubmitSemanticActionErrorKind::UnavailableAction);
            }
        }
        _ => {}
    }
    Ok(())
}

fn validate_editable_semantic_action(
    node: &crate::SemanticNode,
    action: &SemanticAction,
    data: Option<&SemanticActionData>,
) -> Result<(), SubmitSemanticActionErrorKind> {
    let Some(editable) = node.editable() else {
        return Ok(());
    };
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
    if modifying && (node.state().read_only() || editable.read_only()) {
        return Err(SubmitSemanticActionErrorKind::UnavailableAction);
    }
    if editable.sensitivity() == TextSensitivity::Secret
        && matches!(action, SemanticAction::Copy | SemanticAction::Cut)
    {
        return Err(SubmitSemanticActionErrorKind::UnavailableAction);
    }
    Ok(())
}

fn semantic_action_is_ready(
    authority: &SemanticActionAuthority,
    action: &SemanticAction,
    automatic_scroll_focusable: Option<bool>,
) -> bool {
    let activation = authority.activation();
    if *action == SemanticAction::RequestFocus {
        return authority.key() == &SemanticKey::PRIMARY
            && focusability_is_eligible(
                authority.focusability(),
                activation,
                automatic_scroll_focusable,
            );
    }
    if !activation.enabled() {
        return false;
    }
    match action {
        SemanticAction::Activate => {
            authority.key() != &SemanticKey::PRIMARY || activation.is_actionable()
        }
        SemanticAction::RequestFocus => unreachable!("focus readiness returned above"),
        SemanticAction::OpenMenu
        | SemanticAction::OpenContextMenu
        | SemanticAction::Increment
        | SemanticAction::Decrement
        | SemanticAction::SetValue
        | SemanticAction::Expand
        | SemanticAction::Collapse => true,
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

const fn semantic_command(
    action: &SemanticAction,
    data: Option<&SemanticActionData>,
) -> Option<SemanticCommand> {
    match action {
        SemanticAction::Activate => Some(SemanticCommand::Activate),
        SemanticAction::RequestFocus => Some(SemanticCommand::RequestFocus),
        SemanticAction::OpenMenu => Some(SemanticCommand::OpenMenu),
        SemanticAction::OpenContextMenu => Some(SemanticCommand::OpenContextMenu),
        SemanticAction::MoveBackward => Some(SemanticCommand::MoveBackward),
        SemanticAction::MoveForward => Some(SemanticCommand::MoveForward),
        SemanticAction::ExtendBackward => Some(SemanticCommand::ExtendBackward),
        SemanticAction::ExtendForward => Some(SemanticCommand::ExtendForward),
        SemanticAction::SelectAll => Some(SemanticCommand::SelectAll),
        SemanticAction::DeleteBackward => Some(SemanticCommand::DeleteBackward),
        SemanticAction::DeleteForward => Some(SemanticCommand::DeleteForward),
        SemanticAction::Undo => Some(SemanticCommand::Undo),
        SemanticAction::Redo => Some(SemanticCommand::Redo),
        SemanticAction::Copy => Some(SemanticCommand::Copy),
        SemanticAction::Cut => Some(SemanticCommand::Cut),
        SemanticAction::Paste => Some(SemanticCommand::Paste),
        SemanticAction::SetSelection => Some(SemanticCommand::SetSelection),
        SemanticAction::ReplaceSelection => Some(SemanticCommand::ReplaceSelection),
        SemanticAction::Increment => Some(SemanticCommand::Increment),
        SemanticAction::Decrement => Some(SemanticCommand::Decrement),
        SemanticAction::SetValue => match data {
            Some(SemanticActionData::NumericValue(value)) => {
                Some(SemanticCommand::SetValue(*value))
            }
            _ => None,
        },
        SemanticAction::Expand => Some(SemanticCommand::Expand),
        SemanticAction::Collapse => Some(SemanticCommand::Collapse),
        _ => None,
    }
}

const fn is_mutable_range_role(role: SemanticRole) -> bool {
    matches!(
        role,
        SemanticRole::Slider
            | SemanticRole::ScrollBar
            | SemanticRole::SpinButton
            | SemanticRole::Splitter
    )
}

const fn is_expandable_role(role: SemanticRole) -> bool {
    matches!(
        role,
        SemanticRole::Button
            | SemanticRole::ComboBox
            | SemanticRole::MenuItem
            | SemanticRole::MenuItemCheckbox
            | SemanticRole::MenuItemRadio
            | SemanticRole::TreeItem
    )
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
