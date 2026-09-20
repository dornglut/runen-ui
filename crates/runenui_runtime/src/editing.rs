//! Runtime-owned ephemeral transactional editing sessions.

#![allow(clippy::redundant_pub_crate)]

use core::num::NonZeroU64;
use std::{collections::HashMap, sync::Arc};

use runenui_core::{
    CompositionGeneration, EditGroupHint, EditIntent, EditInverse, EditKind, EditRequestId,
    EditResolution, EditResolutionOutcome, EditSelection, EditableContribution,
    EditingSessionGeneration, EditingSessionPolicy, MountedNodeId, SemanticCommand, TextAffinity,
    TextDocumentSnapshot, TextRange,
};
use runenui_text::{
    TextCaretMap, TextDisplaySelection, TextNavigation, TextNavigationMode, TextPreeditProjection,
    TextPreferredInline,
};

#[derive(Clone)]
pub(crate) struct EditActionOrigin {
    pub(crate) request: EditRequestId,
    pub(crate) owner: MountedNodeId,
    pub(crate) session: EditingSessionGeneration,
    pub(crate) predecessor: Option<EditRequestId>,
    pub(crate) base_snapshot: TextDocumentSnapshot,
}

pub(crate) struct PreparedEdit<Action> {
    pub(crate) action: Action,
    pub(crate) origin: EditActionOrigin,
}

#[derive(Clone)]
pub(crate) struct EditingSemanticProjection {
    pub(crate) snapshot: TextDocumentSnapshot,
    pub(crate) source: Arc<str>,
    pub(crate) selection: runenui_core::TextSelection,
    pub(crate) sensitivity: runenui_core::TextSensitivity,
}

pub(crate) struct EditingServiceContext {
    pub(crate) session: EditingSessionGeneration,
    pub(crate) snapshot: TextDocumentSnapshot,
    pub(crate) selection: runenui_core::TextSelection,
    pub(crate) selected_text: Arc<str>,
    pub(crate) sensitivity: runenui_core::TextSensitivity,
    pub(crate) read_only: bool,
    pub(crate) disabled: bool,
}

pub(crate) struct EditingImeContext {
    pub(crate) session: EditingSessionGeneration,
    pub(crate) snapshot: TextDocumentSnapshot,
    pub(crate) selection: runenui_core::TextSelection,
    pub(crate) source: Arc<str>,
    pub(crate) preedit: Option<Arc<TextPreeditProjection>>,
    pub(crate) enabled: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum EditPrepareError {
    MissingOwner,
    Unavailable,
    PendingCapacity,
    RequestExhausted,
    InvalidCoordinates,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum EditingReconcileError {
    Capacity,
    SessionGenerationExhausted,
    SameRevisionTextDrift,
    ResolutionCardinality,
    ForeignResolution,
    InconsistentResolution,
}

struct PendingEdit {
    request: EditRequestId,
    predecessor: Option<EditRequestId>,
    base_snapshot: TextDocumentSnapshot,
    proposed_text: Arc<str>,
    proposed_selection: EditSelection,
    replacement_start: usize,
    replacement_end: usize,
    replacement_text: Arc<str>,
}

struct EditingSession<Action> {
    owner: MountedNodeId,
    generation: EditingSessionGeneration,
    contribution: EditableContribution<Action>,
    selection: EditSelection,
    projected_text: Arc<str>,
    pending: Vec<PendingEdit>,
    invalid_suffix: bool,
    preedit: Option<Arc<TextPreeditProjection>>,
    preferred_inline: Option<TextPreferredInline>,
}

struct DrainingSession {
    owner: MountedNodeId,
    generation: EditingSessionGeneration,
    document: runenui_core::TextDocumentId,
    authoritative_snapshot: TextDocumentSnapshot,
    authoritative_text: Arc<str>,
    pending: Vec<PendingEdit>,
    invalid_suffix: bool,
}

pub(crate) struct EditingRegistry<Action> {
    active: HashMap<MountedNodeId, EditingSession<Action>>,
    draining: Vec<DrainingSession>,
    next_session: Option<NonZeroU64>,
    next_request: Option<NonZeroU64>,
    session_limit: usize,
    pending_limit: usize,
}

impl<Action> EditingRegistry<Action> {
    pub(crate) fn new(session_limit: usize, pending_limit: usize) -> Self {
        Self {
            active: HashMap::new(),
            draining: Vec::new(),
            next_session: NonZeroU64::new(1),
            next_request: NonZeroU64::new(1),
            session_limit,
            pending_limit,
        }
    }

    #[cfg(feature = "internal-test-seams")]
    pub(crate) const fn seed_next_request_for_test(&mut self, next: Option<u64>) {
        self.next_request = match next {
            Some(next) => NonZeroU64::new(next),
            None => None,
        };
    }

    #[cfg(feature = "internal-test-seams")]
    pub(crate) const fn seed_next_session_for_test(&mut self, next: Option<u64>) {
        self.next_session = match next {
            Some(next) => NonZeroU64::new(next),
            None => None,
        };
    }

    #[cfg(feature = "internal-test-seams")]
    pub(crate) fn session_counts_for_test(&self) -> (usize, usize) {
        (self.active.len(), self.draining.len())
    }

    pub(crate) fn can_prepare(&self, owner: &MountedNodeId) -> Result<(), EditPrepareError> {
        self.can_prepare_after_reserved(owner, 0)
    }

    pub(crate) fn can_prepare_after_reserved(
        &self,
        owner: &MountedNodeId,
        reserved: usize,
    ) -> Result<(), EditPrepareError> {
        let session = self
            .active
            .get(owner)
            .ok_or(EditPrepareError::MissingOwner)?;
        if session.invalid_suffix
            || session.contribution.disabled()
            || session.contribution.read_only()
        {
            return Err(EditPrepareError::Unavailable);
        }
        if session
            .pending
            .len()
            .checked_add(reserved)
            .is_none_or(|reserved_total| reserved_total >= self.pending_limit)
        {
            return Err(EditPrepareError::PendingCapacity);
        }
        let next_request = self
            .next_request
            .ok_or(EditPrepareError::RequestExhausted)?;
        let reserved = u64::try_from(reserved).map_err(|_| EditPrepareError::RequestExhausted)?;
        if next_request.get().checked_add(reserved).is_none() {
            return Err(EditPrepareError::RequestExhausted);
        }
        Ok(())
    }

    pub(crate) fn has_owner(&self, owner: &MountedNodeId) -> bool {
        self.active.contains_key(owner)
    }

    pub(crate) fn shutdown(&mut self) {
        self.active.clear();
        self.draining.clear();
    }

    pub(crate) fn sensitivity(
        &self,
        owner: &MountedNodeId,
    ) -> Option<runenui_core::TextSensitivity> {
        self.active
            .get(owner)
            .map(|session| session.contribution.sensitivity())
    }

    pub(crate) fn framework_service_context(
        &self,
        owner: &MountedNodeId,
    ) -> Option<EditingServiceContext> {
        let session = self.active.get(owner)?;
        if session.invalid_suffix || !session.pending.is_empty() || session.preedit.is_some() {
            return None;
        }
        let snapshot = session.contribution.snapshot();
        let selection = session
            .selection
            .bind(snapshot, &session.projected_text)
            .ok()?;
        let start = selection
            .anchor()
            .byte_offset()
            .min(selection.active().byte_offset());
        let end = selection
            .anchor()
            .byte_offset()
            .max(selection.active().byte_offset());
        Some(EditingServiceContext {
            session: session.generation.clone(),
            snapshot,
            selection,
            selected_text: Arc::from(&session.projected_text[start..end]),
            sensitivity: session.contribution.sensitivity(),
            read_only: session.contribution.read_only(),
            disabled: session.contribution.disabled(),
        })
    }

    pub(crate) fn input_method_context(&self, owner: &MountedNodeId) -> Option<EditingImeContext> {
        let session = self.active.get(owner)?;
        if session.invalid_suffix || !session.pending.is_empty() {
            return None;
        }
        let snapshot = session.contribution.snapshot();
        let selection = session
            .selection
            .bind(snapshot, &session.projected_text)
            .ok()?;
        let source = Arc::clone(&session.projected_text);
        let preedit = session.preedit.as_ref().map(Arc::clone);
        if let Some(preedit) = preedit.as_ref()
            && (preedit.snapshot() != snapshot || preedit.document_text() != source.as_ref())
        {
            return None;
        }
        Some(EditingImeContext {
            session: session.generation.clone(),
            snapshot,
            selection,
            source,
            preedit,
            enabled: !session.contribution.read_only() && !session.contribution.disabled(),
        })
    }

    pub(crate) fn framework_service_binding_matches(
        &self,
        owner: &MountedNodeId,
        generation: &EditingSessionGeneration,
        snapshot: TextDocumentSnapshot,
        selection: runenui_core::TextSelection,
    ) -> bool {
        self.framework_service_context(owner)
            .is_some_and(|current| {
                current.session == *generation
                    && current.snapshot == snapshot
                    && current.selection == selection
            })
    }

    pub(crate) fn semantic_projections(&self) -> HashMap<MountedNodeId, EditingSemanticProjection> {
        self.active
            .iter()
            .filter_map(|(owner, session)| {
                if session.invalid_suffix
                    || !session.pending.is_empty()
                    || session.preedit.is_some()
                {
                    return None;
                }
                let selection = session
                    .selection
                    .bind(session.contribution.snapshot(), session.contribution.text())
                    .ok()?;
                Some((
                    owner.clone(),
                    EditingSemanticProjection {
                        snapshot: session.contribution.snapshot(),
                        source: Arc::from(session.contribution.text()),
                        selection,
                        sensitivity: session.contribution.sensitivity(),
                    },
                ))
            })
            .collect()
    }

    pub(crate) fn preedit_projections(&self) -> HashMap<MountedNodeId, Arc<TextPreeditProjection>> {
        self.active
            .iter()
            .filter_map(|(owner, session)| {
                session
                    .preedit
                    .as_ref()
                    .map(|projection| (owner.clone(), Arc::clone(projection)))
            })
            .collect()
    }

    pub(crate) fn caret_source(
        &self,
        owner: &MountedNodeId,
    ) -> Result<(TextDocumentSnapshot, Arc<str>), EditPrepareError> {
        let session = self
            .active
            .get(owner)
            .ok_or(EditPrepareError::MissingOwner)?;
        if session.invalid_suffix || !session.pending.is_empty() || session.preedit.is_some() {
            return Err(EditPrepareError::Unavailable);
        }
        Ok((
            session.contribution.snapshot(),
            Arc::clone(&session.projected_text),
        ))
    }

    pub(crate) fn prepare_insert(
        &mut self,
        namespace: &runenui_core::__runtime::RuntimeNamespace,
        owner: &MountedNodeId,
        replacement_text: &str,
        composition: Option<CompositionGeneration>,
    ) -> Result<PreparedEdit<Action>, EditPrepareError> {
        let session = self
            .active
            .get(owner)
            .ok_or(EditPrepareError::MissingOwner)?;
        let start = session.selection.anchor().min(session.selection.active());
        let end = session.selection.anchor().max(session.selection.active());
        self.prepare_replacement(
            namespace,
            owner,
            start,
            end,
            replacement_text,
            EditKind::Insert,
            composition,
        )
    }

    pub(crate) fn prepare_replace_selection(
        &mut self,
        namespace: &runenui_core::__runtime::RuntimeNamespace,
        owner: &MountedNodeId,
        replacement_text: &str,
    ) -> Result<PreparedEdit<Action>, EditPrepareError> {
        let session = self
            .active
            .get(owner)
            .ok_or(EditPrepareError::MissingOwner)?;
        let start = session.selection.anchor().min(session.selection.active());
        let end = session.selection.anchor().max(session.selection.active());
        self.prepare_replacement(
            namespace,
            owner,
            start,
            end,
            replacement_text,
            EditKind::Replace,
            None,
        )
    }

    pub(crate) fn set_selection(
        &mut self,
        owner: &MountedNodeId,
        selection: runenui_core::TextSelection,
        caret_map: &TextCaretMap,
    ) -> Result<bool, EditPrepareError> {
        self.validate_selection(owner, selection, caret_map)?;
        let session = self
            .active
            .get_mut(owner)
            .ok_or(EditPrepareError::MissingOwner)?;
        let next = runenui_core::EditSelection::from_selection(selection);
        let changed = session.selection != next;
        session.selection = next;
        session.preferred_inline = None;
        Ok(changed)
    }

    pub(crate) fn validate_selection(
        &self,
        owner: &MountedNodeId,
        selection: runenui_core::TextSelection,
        caret_map: &TextCaretMap,
    ) -> Result<(), EditPrepareError> {
        let session = self
            .active
            .get(owner)
            .ok_or(EditPrepareError::MissingOwner)?;
        if session.invalid_suffix
            || session.contribution.disabled()
            || !session.pending.is_empty()
            || session.preedit.is_some()
            || selection.anchor().snapshot() != session.contribution.snapshot()
            || selection.active().snapshot() != session.contribution.snapshot()
        {
            return Err(EditPrepareError::Unavailable);
        }
        let display = TextDisplaySelection::from_document(selection);
        caret_map
            .validate_position(display.anchor())
            .map_err(|_| EditPrepareError::InvalidCoordinates)?;
        caret_map
            .validate_position(display.active())
            .map_err(|_| EditPrepareError::InvalidCoordinates)
    }

    pub(crate) fn prepare_command(
        &mut self,
        namespace: &runenui_core::__runtime::RuntimeNamespace,
        owner: &MountedNodeId,
        command: SemanticCommand,
        caret_map: Option<&TextCaretMap>,
    ) -> Result<Option<PreparedEdit<Action>>, EditPrepareError> {
        match command {
            SemanticCommand::MoveBackward
            | SemanticCommand::MoveForward
            | SemanticCommand::ExtendBackward
            | SemanticCommand::ExtendForward
            | SemanticCommand::SelectAll => {
                self.move_selection(
                    owner,
                    command,
                    caret_map.ok_or(EditPrepareError::Unavailable)?,
                )?;
                Ok(None)
            }
            SemanticCommand::DeleteBackward | SemanticCommand::DeleteForward => {
                let session = self
                    .active
                    .get(owner)
                    .ok_or(EditPrepareError::MissingOwner)?;
                if !session.pending.is_empty() || session.preedit.is_some() {
                    return Err(EditPrepareError::Unavailable);
                }
                let mut start = session.selection.anchor().min(session.selection.active());
                let mut end = session.selection.anchor().max(session.selection.active());
                if start == end {
                    let map = caret_map.ok_or(EditPrepareError::Unavailable)?;
                    let bound = session
                        .selection
                        .bind(session.contribution.snapshot(), &session.projected_text)
                        .map_err(|_| EditPrepareError::InvalidCoordinates)?;
                    let result = map
                        .navigate(
                            &TextDisplaySelection::from_document(bound),
                            if command == SemanticCommand::DeleteBackward {
                                TextNavigation::PreviousLogical
                            } else {
                                TextNavigation::NextLogical
                            },
                            TextNavigationMode::Move,
                            None,
                        )
                        .map_err(|_| EditPrepareError::InvalidCoordinates)?;
                    let moved =
                        edit_selection_from_display(result.selection(), &session.projected_text)?;
                    if command == SemanticCommand::DeleteBackward {
                        start = moved.active();
                    } else {
                        end = moved.active();
                    }
                }
                if start == end {
                    return Ok(None);
                }
                self.prepare_replacement(
                    namespace,
                    owner,
                    start,
                    end,
                    "",
                    if command == SemanticCommand::DeleteBackward {
                        EditKind::DeleteBackward
                    } else {
                        EditKind::DeleteForward
                    },
                    None,
                )
                .map(Some)
            }
            SemanticCommand::Undo | SemanticCommand::Redo => self
                .prepare_control_intent(
                    namespace,
                    owner,
                    if command == SemanticCommand::Undo {
                        EditKind::Undo
                    } else {
                        EditKind::Redo
                    },
                )
                .map(Some),
            _ => Ok(None),
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn prepare_replacement(
        &mut self,
        namespace: &runenui_core::__runtime::RuntimeNamespace,
        owner: &MountedNodeId,
        start: usize,
        end: usize,
        replacement_text: &str,
        kind: EditKind,
        composition: Option<CompositionGeneration>,
    ) -> Result<PreparedEdit<Action>, EditPrepareError> {
        self.can_prepare(owner)?;
        let request_value = self
            .next_request
            .ok_or(EditPrepareError::RequestExhausted)?;
        let next_request = request_value.get().checked_add(1).and_then(NonZeroU64::new);
        let session = self
            .active
            .get_mut(owner)
            .ok_or(EditPrepareError::MissingOwner)?;
        let selection = session.selection;
        let snapshot = session.contribution.snapshot();
        let replacement = TextRange::new(snapshot, &session.projected_text, start, end)
            .map_err(|_| EditPrepareError::InvalidCoordinates)?;
        let mut proposed = String::with_capacity(
            session
                .projected_text
                .len()
                .saturating_sub(end.saturating_sub(start))
                .saturating_add(replacement_text.len()),
        );
        proposed.push_str(&session.projected_text[..start]);
        proposed.push_str(replacement_text);
        proposed.push_str(&session.projected_text[end..]);
        let caret = start
            .checked_add(replacement_text.len())
            .ok_or(EditPrepareError::InvalidCoordinates)?;
        let proposed_selection =
            EditSelection::collapsed(&proposed, caret, collapsed_affinity(&proposed, caret))
                .map_err(|_| EditPrepareError::InvalidCoordinates)?;
        let prior_selection = selection;
        let inverse = EditInverse::__runtime_new(
            start..caret,
            Arc::<str>::from(&session.projected_text[start..end]),
            prior_selection,
        );
        let request = namespace.__runtime_edit_request_id(request_value.get());
        let predecessor = session
            .pending
            .last()
            .map(|pending| pending.request.clone());
        let intent = EditIntent::__runtime_new(
            request.clone(),
            session.generation.clone(),
            predecessor.clone(),
            replacement,
            Arc::<str>::from(replacement_text),
            proposed_selection,
            kind,
            Some(inverse),
            Some(EditGroupHint::new(session.pending.first().map_or_else(
                || request.get(),
                |pending| pending.request.get(),
            ))),
            session.contribution.sensitivity(),
            composition,
        );
        let action = session.contribution.map_intent(intent);
        let proposed_text: Arc<str> = proposed.into();
        session.pending.push(PendingEdit {
            request: request.clone(),
            predecessor: predecessor.clone(),
            base_snapshot: snapshot,
            proposed_text: Arc::clone(&proposed_text),
            proposed_selection,
            replacement_start: start,
            replacement_end: end,
            replacement_text: Arc::from(replacement_text),
        });
        session.projected_text = proposed_text;
        session.selection = proposed_selection;
        session.preferred_inline = None;
        self.next_request = next_request;
        Ok(PreparedEdit {
            action,
            origin: EditActionOrigin {
                request,
                owner: owner.clone(),
                session: session.generation.clone(),
                predecessor,
                base_snapshot: snapshot,
            },
        })
    }

    pub(crate) fn prepare_framework_service_edit(
        &mut self,
        namespace: &runenui_core::__runtime::RuntimeNamespace,
        binding: &runenui_core::__runtime::FrameworkServiceBinding,
        replacement_text: &str,
        kind: runenui_core::EditKind,
    ) -> Result<PreparedEdit<Action>, EditPrepareError> {
        let owner = binding.owner();
        let (Some(generation), Some(snapshot), Some(selection)) = (
            binding.editing_session(),
            binding.document_snapshot(),
            binding.selection(),
        ) else {
            return Err(EditPrepareError::Unavailable);
        };
        if !self.framework_service_binding_matches(owner, generation, snapshot, selection) {
            return Err(EditPrepareError::Unavailable);
        }
        let session = self
            .active
            .get(owner)
            .ok_or(EditPrepareError::MissingOwner)?;
        if session.contribution.read_only() || session.contribution.disabled() {
            return Err(EditPrepareError::Unavailable);
        }
        if kind == runenui_core::EditKind::Cut && selection.is_collapsed() {
            return Err(EditPrepareError::Unavailable);
        }
        self.prepare_replacement(
            namespace,
            owner,
            selection
                .anchor()
                .byte_offset()
                .min(selection.active().byte_offset()),
            selection
                .anchor()
                .byte_offset()
                .max(selection.active().byte_offset()),
            replacement_text,
            kind,
            None,
        )
    }

    fn prepare_control_intent(
        &mut self,
        namespace: &runenui_core::__runtime::RuntimeNamespace,
        owner: &MountedNodeId,
        kind: EditKind,
    ) -> Result<PreparedEdit<Action>, EditPrepareError> {
        self.can_prepare(owner)?;
        let request_value = self
            .next_request
            .ok_or(EditPrepareError::RequestExhausted)?;
        let session = self
            .active
            .get_mut(owner)
            .ok_or(EditPrepareError::MissingOwner)?;
        let snapshot = session.contribution.snapshot();
        let offset = session.selection.active();
        let replacement = TextRange::new(snapshot, &session.projected_text, offset, offset)
            .map_err(|_| EditPrepareError::InvalidCoordinates)?;
        let request = namespace.__runtime_edit_request_id(request_value.get());
        let predecessor = session
            .pending
            .last()
            .map(|pending| pending.request.clone());
        let intent = EditIntent::__runtime_new(
            request.clone(),
            session.generation.clone(),
            predecessor.clone(),
            replacement,
            Arc::<str>::from(""),
            session.selection,
            kind,
            None,
            None,
            session.contribution.sensitivity(),
            None,
        );
        let action = session.contribution.map_intent(intent);
        session.pending.push(PendingEdit {
            request: request.clone(),
            predecessor: predecessor.clone(),
            base_snapshot: snapshot,
            proposed_text: Arc::clone(&session.projected_text),
            proposed_selection: session.selection,
            replacement_start: offset,
            replacement_end: offset,
            replacement_text: Arc::from(""),
        });
        self.next_request = request_value.get().checked_add(1).and_then(NonZeroU64::new);
        Ok(PreparedEdit {
            action,
            origin: EditActionOrigin {
                request,
                owner: owner.clone(),
                session: session.generation.clone(),
                predecessor,
                base_snapshot: snapshot,
            },
        })
    }

    fn move_selection(
        &mut self,
        owner: &MountedNodeId,
        command: SemanticCommand,
        caret_map: &TextCaretMap,
    ) -> Result<(), EditPrepareError> {
        let session = self
            .active
            .get_mut(owner)
            .ok_or(EditPrepareError::MissingOwner)?;
        if session.invalid_suffix
            || session.contribution.disabled()
            || !session.pending.is_empty()
            || session.preedit.is_some()
        {
            return Err(EditPrepareError::Unavailable);
        }
        if command == SemanticCommand::SelectAll {
            session.selection = EditSelection::new(
                &session.projected_text,
                0,
                TextAffinity::Downstream,
                session.projected_text.len(),
                TextAffinity::Upstream,
            )
            .map_err(|_| EditPrepareError::InvalidCoordinates)?;
            session.preferred_inline = None;
            return Ok(());
        }
        let bound = session
            .selection
            .bind(session.contribution.snapshot(), &session.projected_text)
            .map_err(|_| EditPrepareError::InvalidCoordinates)?;
        let result = caret_map
            .navigate(
                &TextDisplaySelection::from_document(bound),
                if matches!(
                    command,
                    SemanticCommand::MoveBackward | SemanticCommand::ExtendBackward
                ) {
                    TextNavigation::PreviousLogical
                } else {
                    TextNavigation::NextLogical
                },
                if matches!(
                    command,
                    SemanticCommand::ExtendBackward | SemanticCommand::ExtendForward
                ) {
                    TextNavigationMode::Extend
                } else {
                    TextNavigationMode::Move
                },
                session.preferred_inline,
            )
            .map_err(|_| EditPrepareError::InvalidCoordinates)?;
        session.selection =
            edit_selection_from_display(result.selection(), &session.projected_text)?;
        session.preferred_inline = result.preferred_inline();
        Ok(())
    }

    pub(crate) fn initial_reconcile(
        &mut self,
        namespace: &runenui_core::__runtime::RuntimeNamespace,
        contributions: Vec<(MountedNodeId, EditableContribution<Action>)>,
    ) -> Result<(), EditingReconcileError> {
        self.validate_documents(&contributions)?;
        if contributions.len() > self.session_limit {
            return Err(EditingReconcileError::Capacity);
        }
        self.preflight_session_generations(contributions.len())?;
        for (owner, contribution) in contributions {
            let generation = self.allocate_session(namespace)?;
            let selection = EditSelection::from_selection(contribution.initial_selection());
            let projected_text = Arc::<str>::from(contribution.text());
            self.active.insert(
                owner.clone(),
                EditingSession {
                    owner,
                    generation,
                    contribution,
                    selection,
                    projected_text,
                    pending: Vec::new(),
                    invalid_suffix: false,
                    preedit: None,
                    preferred_inline: None,
                },
            );
        }
        Ok(())
    }

    #[allow(clippy::needless_pass_by_value, clippy::too_many_lines)]
    pub(crate) fn reconcile(
        &mut self,
        namespace: &runenui_core::__runtime::RuntimeNamespace,
        contributions: Vec<(MountedNodeId, EditableContribution<Action>)>,
        origin: &crate::queue::ApplicationActionOrigin,
        resolution: Option<EditResolution>,
    ) -> Result<(), EditingReconcileError> {
        self.validate_documents(&contributions)?;
        for (owner, contribution) in &contributions {
            if let Some(previous) = self.active.get(owner)
                && previous.contribution.snapshot() == contribution.snapshot()
                && previous.contribution.text() != contribution.text()
            {
                return Err(EditingReconcileError::SameRevisionTextDrift);
            }
        }
        let edit_origin = match origin {
            crate::queue::ApplicationActionOrigin::Ordinary => {
                if resolution.is_some() {
                    return Err(EditingReconcileError::ResolutionCardinality);
                }
                None
            }
            crate::queue::ApplicationActionOrigin::Edit(edit_origin) => {
                let resolution = resolution
                    .as_ref()
                    .ok_or(EditingReconcileError::ResolutionCardinality)?;
                if resolution.request() != &edit_origin.request {
                    return Err(EditingReconcileError::ForeignResolution);
                }
                Some((edit_origin, resolution))
            }
        };

        let mut incoming: HashMap<_, _> = contributions.into_iter().collect();
        let required_generations = self.required_new_generations(&incoming, edit_origin);
        let total_after = self.required_session_capacity(&incoming, edit_origin);
        if total_after > self.session_limit {
            return Err(EditingReconcileError::Capacity);
        }
        self.preflight_session_generations(required_generations)?;
        let old = core::mem::take(&mut self.active);
        for (owner, mut session) in old {
            let Some(contribution) = incoming.remove(&owner) else {
                self.retire(session);
                continue;
            };
            let exact = contribution.snapshot() == session.contribution.snapshot()
                && contribution.text() == session.contribution.text()
                && contribution.session_policy() == EditingSessionPolicy::PreserveExact;
            let resolving_edit = edit_origin.filter(|(edit_origin, _)| {
                edit_origin.owner == owner && edit_origin.session == session.generation
            });
            if let Some((resolving_origin, resolution)) = resolving_edit {
                Self::resolve_live(&mut session, &contribution, resolving_origin, resolution)?;
                session.contribution = contribution;
                self.active.insert(owner, session);
            } else if exact {
                session.contribution = contribution;
                self.active.insert(owner, session);
            } else {
                self.retire(session);
                let generation = self.allocate_session(namespace)?;
                let selection = EditSelection::from_selection(contribution.initial_selection());
                let projected_text = Arc::<str>::from(contribution.text());
                self.active.insert(
                    owner.clone(),
                    EditingSession {
                        owner,
                        generation,
                        contribution,
                        selection,
                        projected_text,
                        pending: Vec::new(),
                        invalid_suffix: false,
                        preedit: None,
                        preferred_inline: None,
                    },
                );
            }
        }
        for (owner, contribution) in incoming {
            let generation = self.allocate_session(namespace)?;
            let selection = EditSelection::from_selection(contribution.initial_selection());
            let projected_text = Arc::<str>::from(contribution.text());
            self.active.insert(
                owner.clone(),
                EditingSession {
                    owner,
                    generation,
                    contribution,
                    selection,
                    projected_text,
                    pending: Vec::new(),
                    invalid_suffix: false,
                    preedit: None,
                    preferred_inline: None,
                },
            );
        }
        if let Some((edit_origin, resolution)) = edit_origin
            && !self
                .active
                .values()
                .any(|session| session.generation == edit_origin.session)
        {
            self.resolve_draining(edit_origin, resolution)?;
        }
        self.draining.retain(|session| !session.pending.is_empty());
        Ok(())
    }

    #[allow(clippy::too_many_lines)]
    fn resolve_live(
        session: &mut EditingSession<Action>,
        contribution: &EditableContribution<Action>,
        origin: &EditActionOrigin,
        resolution: &EditResolution,
    ) -> Result<(), EditingReconcileError> {
        let index = session
            .pending
            .iter()
            .position(|pending| &pending.request == resolution.request())
            .ok_or(EditingReconcileError::ForeignResolution)?;
        if index != 0 {
            return Err(EditingReconcileError::InconsistentResolution);
        }
        let pending = session.pending.remove(0);
        if pending.request != origin.request
            || pending.predecessor != origin.predecessor
            || pending.base_snapshot != origin.base_snapshot
        {
            return Err(EditingReconcileError::InconsistentResolution);
        }
        if resolution.resulting_snapshot().document() != pending.base_snapshot.document() {
            return Err(EditingReconcileError::InconsistentResolution);
        }
        match resolution.outcome() {
            EditResolutionOutcome::Accepted => {
                if resolution.resulting_snapshot() != contribution.snapshot()
                    || contribution.text() != pending.proposed_text.as_ref()
                    || (contribution.text() != session.contribution.text()
                        && contribution.snapshot() == session.contribution.snapshot())
                {
                    return Err(EditingReconcileError::InconsistentResolution);
                }
                session.selection = pending.proposed_selection;
            }
            EditResolutionOutcome::Rejected => {
                if resolution.resulting_snapshot() != contribution.snapshot()
                    || contribution.snapshot() != session.contribution.snapshot()
                    || contribution.text() != session.contribution.text()
                {
                    return Err(EditingReconcileError::InconsistentResolution);
                }
                session.invalid_suffix = !session.pending.is_empty();
                session.selection = EditSelection::from_selection(contribution.initial_selection());
                session.projected_text = Arc::<str>::from(contribution.text());
                return Ok(());
            }
            EditResolutionOutcome::Transformed { mapping } => {
                if resolution.resulting_snapshot() != contribution.snapshot() {
                    return Err(EditingReconcileError::InconsistentResolution);
                }
                let replaced = mapping.replaced();
                let replacement_end = replaced
                    .start()
                    .checked_add(mapping.replacement_bytes())
                    .ok_or(EditingReconcileError::InconsistentResolution)?;
                if replaced.snapshot() != pending.base_snapshot
                    || !valid_transformation(&pending.proposed_text, contribution.text(), *mapping)
                    || (contribution.text() != session.contribution.text()
                        && contribution.snapshot() == session.contribution.snapshot())
                {
                    return Err(EditingReconcileError::InconsistentResolution);
                }
                let rebased = rebase_pending_suffix(
                    contribution.text(),
                    &mut session.pending,
                    replaced.start(),
                    replaced.end(),
                    replacement_end,
                );
                if rebased {
                    if let Some(last) = session.pending.last() {
                        session.selection = last.proposed_selection;
                        session.projected_text = Arc::clone(&last.proposed_text);
                    } else {
                        session.selection =
                            EditSelection::from_selection(contribution.initial_selection());
                        session.projected_text = Arc::<str>::from(contribution.text());
                    }
                } else {
                    session.invalid_suffix = !session.pending.is_empty();
                    session.selection =
                        EditSelection::from_selection(contribution.initial_selection());
                    session.projected_text = Arc::<str>::from(contribution.text());
                }
                return Ok(());
            }
            _ => return Err(EditingReconcileError::InconsistentResolution),
        }
        if session.pending.is_empty() {
            session.projected_text = Arc::<str>::from(contribution.text());
        } else {
            let mut previous = Some(pending.request);
            for dependent in &mut session.pending {
                if dependent.predecessor != previous {
                    return Err(EditingReconcileError::InconsistentResolution);
                }
                previous = Some(dependent.request.clone());
            }
            let Some(last) = session.pending.last() else {
                return Err(EditingReconcileError::InconsistentResolution);
            };
            session.projected_text = Arc::clone(&last.proposed_text);
        }
        Ok(())
    }

    pub(crate) fn stage_preedit(
        &mut self,
        owner: &MountedNodeId,
        generation: CompositionGeneration,
        preedit: &str,
        selection: Option<runenui_core::CompositionRange>,
    ) -> Result<(), EditPrepareError> {
        let session = self
            .active
            .get_mut(owner)
            .ok_or(EditPrepareError::MissingOwner)?;
        if session.invalid_suffix
            || session.contribution.disabled()
            || session.contribution.read_only()
            || !session.pending.is_empty()
        {
            return Err(EditPrepareError::Unavailable);
        }
        let start = session.selection.anchor().min(session.selection.active());
        let end = session.selection.anchor().max(session.selection.active());
        let snapshot = session.contribution.snapshot();
        let replacement = TextRange::new(snapshot, session.contribution.text(), start, end)
            .map_err(|_| EditPrepareError::InvalidCoordinates)?;
        let projection = TextPreeditProjection::new(
            snapshot,
            Arc::<str>::from(session.contribution.text()),
            replacement,
            generation,
            Arc::<str>::from(preedit),
            selection,
        )
        .map_err(|_| EditPrepareError::InvalidCoordinates)?;
        session.preedit = Some(Arc::new(projection));
        Ok(())
    }

    pub(crate) fn clear_preedit(
        &mut self,
        owner: &MountedNodeId,
        generation: &CompositionGeneration,
    ) -> bool {
        if let Some(session) = self.active.get_mut(owner)
            && session
                .preedit
                .as_ref()
                .is_some_and(|preedit| preedit.generation() == generation)
        {
            session.preedit = None;
            true
        } else {
            false
        }
    }

    fn resolve_draining(
        &mut self,
        origin: &EditActionOrigin,
        resolution: &EditResolution,
    ) -> Result<(), EditingReconcileError> {
        let matching_active = self.active.values().find(|active| {
            active.contribution.snapshot() == resolution.resulting_snapshot()
                && active.contribution.snapshot().document() == origin.base_snapshot.document()
        });
        let origin_owner = &origin.owner;
        let origin_generation = &origin.session;
        let session = self
            .draining
            .iter_mut()
            .find(|session| {
                &session.owner == origin_owner && &session.generation == origin_generation
            })
            .ok_or(EditingReconcileError::ForeignResolution)?;
        let Some(pending) = session.pending.first() else {
            return Err(EditingReconcileError::ForeignResolution);
        };
        if pending.request != origin.request
            || pending.base_snapshot != origin.base_snapshot
            || pending.predecessor != origin.predecessor
        {
            return Err(EditingReconcileError::ForeignResolution);
        }
        match resolution.outcome() {
            EditResolutionOutcome::Rejected => {
                if resolution.resulting_snapshot() != pending.base_snapshot
                    && matching_active.is_none()
                {
                    return Err(EditingReconcileError::InconsistentResolution);
                }
            }
            EditResolutionOutcome::Accepted => {
                if matching_active.is_none()
                    || session.document != resolution.resulting_snapshot().document()
                    || !self.active.values().any(|active| {
                        active.contribution.snapshot() == resolution.resulting_snapshot()
                            && active.contribution.text() == pending.proposed_text.as_ref()
                    })
                {
                    return Err(EditingReconcileError::InconsistentResolution);
                }
            }
            EditResolutionOutcome::Transformed { mapping } => {
                if matching_active.is_none()
                    || session.document != resolution.resulting_snapshot().document()
                    || matching_active.is_none_or(|active| {
                        !valid_transformation(
                            &pending.proposed_text,
                            active.contribution.text(),
                            *mapping,
                        )
                    })
                {
                    return Err(EditingReconcileError::InconsistentResolution);
                }
            }
            _ => return Err(EditingReconcileError::InconsistentResolution),
        }
        session.pending.remove(0);
        if !session.pending.is_empty() {
            session.invalid_suffix = true;
        }
        Ok(())
    }

    fn retire(&mut self, session: EditingSession<Action>) {
        if !session.pending.is_empty() {
            self.draining.push(DrainingSession {
                document: session.contribution.snapshot().document(),
                authoritative_snapshot: session.contribution.snapshot(),
                authoritative_text: Arc::from(session.contribution.text()),
                owner: session.owner,
                generation: session.generation,
                pending: session.pending,
                invalid_suffix: session.invalid_suffix,
            });
        }
    }

    fn allocate_session(
        &mut self,
        namespace: &runenui_core::__runtime::RuntimeNamespace,
    ) -> Result<EditingSessionGeneration, EditingReconcileError> {
        let next = self
            .next_session
            .ok_or(EditingReconcileError::SessionGenerationExhausted)?;
        self.next_session = next.get().checked_add(1).and_then(NonZeroU64::new);
        Ok(namespace.__runtime_editing_session_generation(next.get()))
    }

    fn required_session_capacity(
        &self,
        incoming: &HashMap<MountedNodeId, EditableContribution<Action>>,
        edit_origin: Option<(&EditActionOrigin, &EditResolution)>,
    ) -> usize {
        let newly_draining = self.active.iter().filter(|(owner, session)| {
            if session.pending.is_empty() {
                return false;
            }
            let resolves_and_finishes = session.pending.len() == 1
                && edit_origin.is_some_and(|(origin, _)| {
                    origin.owner == **owner && origin.session == session.generation
                });
            let Some(contribution) = incoming.get(*owner) else {
                return !resolves_and_finishes;
            };
            let exact = contribution.snapshot() == session.contribution.snapshot()
                && contribution.text() == session.contribution.text()
                && contribution.session_policy() == EditingSessionPolicy::PreserveExact;
            let resolves_here = edit_origin.is_some_and(|(origin, _)| {
                origin.owner == **owner && origin.session == session.generation
            });
            !exact && !resolves_here
        });
        let resolved_draining = edit_origin.is_some_and(|(origin, _)| {
            let origin_owner = &origin.owner;
            let origin_generation = &origin.session;
            self.draining.iter().any(|session| {
                &session.owner == origin_owner
                    && &session.generation == origin_generation
                    && session.pending.len() == 1
            })
        });
        incoming
            .len()
            .saturating_add(self.draining.len())
            .saturating_add(newly_draining.count())
            .saturating_sub(usize::from(resolved_draining))
    }

    fn required_new_generations(
        &self,
        incoming: &HashMap<MountedNodeId, EditableContribution<Action>>,
        edit_origin: Option<(&EditActionOrigin, &EditResolution)>,
    ) -> usize {
        incoming
            .iter()
            .filter(|(owner, contribution)| {
                let Some(session) = self.active.get(*owner) else {
                    return true;
                };
                let exact = contribution.snapshot() == session.contribution.snapshot()
                    && contribution.text() == session.contribution.text()
                    && contribution.session_policy() == EditingSessionPolicy::PreserveExact;
                let resolves_here = edit_origin.is_some_and(|(origin, _)| {
                    origin.owner == **owner && origin.session == session.generation
                });
                !exact && !resolves_here
            })
            .count()
    }

    fn preflight_session_generations(&self, required: usize) -> Result<(), EditingReconcileError> {
        if required == 0 {
            return Ok(());
        }
        let first = self
            .next_session
            .ok_or(EditingReconcileError::SessionGenerationExhausted)?
            .get();
        let additional = u64::try_from(required - 1)
            .map_err(|_| EditingReconcileError::SessionGenerationExhausted)?;
        first
            .checked_add(additional)
            .ok_or(EditingReconcileError::SessionGenerationExhausted)?;
        Ok(())
    }

    fn validate_documents(
        &self,
        contributions: &[(MountedNodeId, EditableContribution<Action>)],
    ) -> Result<(), EditingReconcileError> {
        let mut texts = HashMap::<TextDocumentSnapshot, &str>::new();
        for (_, contribution) in contributions {
            if let Some(existing) = texts.insert(contribution.snapshot(), contribution.text())
                && existing != contribution.text()
            {
                return Err(EditingReconcileError::SameRevisionTextDrift);
            }
        }
        for (_, contribution) in contributions {
            if self.active.values().any(|session| {
                session.contribution.snapshot() == contribution.snapshot()
                    && session.contribution.text() != contribution.text()
            }) || self.draining.iter().any(|session| {
                session.authoritative_snapshot == contribution.snapshot()
                    && session.authoritative_text.as_ref() != contribution.text()
            }) {
                return Err(EditingReconcileError::SameRevisionTextDrift);
            }
        }
        Ok(())
    }
}

fn rebase_pending_suffix(
    authoritative: &str,
    pending: &mut [PendingEdit],
    mut changed_start: usize,
    mut changed_old_end: usize,
    mut changed_new_end: usize,
) -> bool {
    let mut projected = authoritative.to_owned();
    for dependent in pending {
        let original_start = dependent.replacement_start;
        let original_end = dependent.replacement_end;
        if original_end > changed_start && original_start < changed_old_end {
            return false;
        }
        let Some(mapped_start) = map_offset(
            original_start,
            changed_start,
            changed_old_end,
            changed_new_end,
        ) else {
            return false;
        };
        let Some(mapped_end) = map_offset(
            original_end,
            changed_start,
            changed_old_end,
            changed_new_end,
        ) else {
            return false;
        };
        if mapped_start > mapped_end
            || mapped_end > projected.len()
            || !projected.is_char_boundary(mapped_start)
            || !projected.is_char_boundary(mapped_end)
        {
            return false;
        }
        projected.replace_range(mapped_start..mapped_end, &dependent.replacement_text);
        let Some(caret) = mapped_start.checked_add(dependent.replacement_text.len()) else {
            return false;
        };
        let Ok(selection) =
            EditSelection::collapsed(&projected, caret, collapsed_affinity(&projected, caret))
        else {
            return false;
        };
        dependent.replacement_start = mapped_start;
        dependent.replacement_end = mapped_end;
        dependent.proposed_text = Arc::from(projected.as_str());
        dependent.proposed_selection = selection;

        let old_delta =
            dependent.replacement_text.len() as i128 - (original_end - original_start) as i128;
        let new_delta =
            dependent.replacement_text.len() as i128 - (mapped_end - mapped_start) as i128;
        if original_end <= changed_start {
            let Ok(shifted_start) = usize::try_from(changed_start as i128 + old_delta) else {
                return false;
            };
            let Ok(shifted_old_end) = usize::try_from(changed_old_end as i128 + old_delta) else {
                return false;
            };
            let Ok(shifted_new_end) = usize::try_from(changed_new_end as i128 + new_delta) else {
                return false;
            };
            changed_start = shifted_start;
            changed_old_end = shifted_old_end;
            changed_new_end = shifted_new_end;
        }
    }
    true
}

fn map_offset(
    offset: usize,
    changed_start: usize,
    changed_old_end: usize,
    changed_new_end: usize,
) -> Option<usize> {
    if offset <= changed_start {
        Some(offset)
    } else if offset >= changed_old_end {
        let delta = changed_new_end as i128 - changed_old_end as i128;
        usize::try_from(offset as i128 + delta).ok()
    } else {
        None
    }
}

const fn collapsed_affinity(source: &str, offset: usize) -> TextAffinity {
    if !source.is_empty() && offset == source.len() {
        TextAffinity::Upstream
    } else {
        TextAffinity::Downstream
    }
}

fn valid_transformation(old: &str, new: &str, mapping: runenui_core::EditChangeMap) -> bool {
    let replaced = mapping.replaced();
    let Some(replacement_end) = replaced.start().checked_add(mapping.replacement_bytes()) else {
        return false;
    };
    let mapped_len = old
        .len()
        .checked_sub(replaced.end().saturating_sub(replaced.start()))
        .and_then(|length| length.checked_add(mapping.replacement_bytes()));
    let Some(old_prefix) = old.get(..replaced.start()) else {
        return false;
    };
    let Some(old_suffix) = old.get(replaced.end()..) else {
        return false;
    };
    let Some(new_prefix) = new.get(..replaced.start()) else {
        return false;
    };
    let Some(new_suffix) = new.get(replacement_end..) else {
        return false;
    };
    mapped_len == Some(new.len()) && old_prefix == new_prefix && old_suffix == new_suffix
}

fn edit_selection_from_display(
    selection: &TextDisplaySelection,
    source: &str,
) -> Result<EditSelection, EditPrepareError> {
    let runenui_core::TextDisplayPosition::Document(anchor) = selection.anchor() else {
        return Err(EditPrepareError::InvalidCoordinates);
    };
    let runenui_core::TextDisplayPosition::Document(active) = selection.active() else {
        return Err(EditPrepareError::InvalidCoordinates);
    };
    EditSelection::new(
        source,
        anchor.byte_offset(),
        anchor.affinity(),
        active.byte_offset(),
        active.affinity(),
    )
    .map_err(|_| EditPrepareError::InvalidCoordinates)
}

#[cfg(test)]
mod tests {
    use super::*;
    use runenui_core::{
        __runtime::RuntimeNamespace, TextDocumentId, TextDocumentRevision, TextPosition,
        TextSelection, TextSensitivity,
    };

    fn snapshot(revision: u64) -> TextDocumentSnapshot {
        TextDocumentSnapshot::new(TextDocumentId::new(9), TextDocumentRevision::new(revision))
    }

    fn contribution(
        revision: u64,
        text: &str,
        selection: usize,
        policy: EditingSessionPolicy,
    ) -> EditableContribution<EditIntent> {
        let snapshot = snapshot(revision);
        let position = TextPosition::new(
            snapshot,
            text,
            selection,
            collapsed_affinity(text, selection),
        )
        .unwrap_or_else(|_| unreachable!("fixture selection is valid"));
        EditableContribution::new(
            snapshot,
            text,
            TextSelection::collapsed(position),
            TextSensitivity::Public,
            false,
            false,
            policy,
            |intent| intent,
        )
        .unwrap_or_else(|_| unreachable!("fixture contribution is valid"))
    }

    #[test]
    fn multiple_presentations_are_independent_and_same_revision_drift_is_non_mutating() {
        let namespace = RuntimeNamespace::__runtime_new();
        let first = namespace.__runtime_mounted_id(1, 1);
        let second = namespace.__runtime_mounted_id(2, 1);
        let mut registry = EditingRegistry::new(4, 4);
        registry
            .initial_reconcile(
                &namespace,
                vec![
                    (
                        first.clone(),
                        contribution(0, "same", 0, EditingSessionPolicy::PreserveExact),
                    ),
                    (
                        second.clone(),
                        contribution(0, "same", 4, EditingSessionPolicy::PreserveExact),
                    ),
                ],
            )
            .unwrap_or_else(|_| unreachable!("fixture sessions reconcile"));
        assert_ne!(
            registry.active[&first].generation,
            registry.active[&second].generation
        );
        assert_eq!(registry.active[&first].selection.active(), 0);
        assert_eq!(registry.active[&second].selection.active(), 4);

        let first_generation = registry.active[&first].generation.clone();
        let second_generation = registry.active[&second].generation.clone();
        let result = registry.reconcile(
            &namespace,
            vec![
                (
                    first.clone(),
                    contribution(0, "drift", 0, EditingSessionPolicy::PreserveExact),
                ),
                (
                    second.clone(),
                    contribution(0, "same", 4, EditingSessionPolicy::PreserveExact),
                ),
            ],
            &crate::queue::ApplicationActionOrigin::Ordinary,
            None,
        );
        assert_eq!(result, Err(EditingReconcileError::SameRevisionTextDrift));
        assert_eq!(registry.active[&first].generation, first_generation);
        assert_eq!(registry.active[&second].generation, second_generation);
        assert_eq!(registry.active[&first].contribution.text(), "same");
    }

    #[test]
    fn capacity_and_generation_exhaustion_preflight_without_partial_sessions() {
        let namespace = RuntimeNamespace::__runtime_new();
        let first = namespace.__runtime_mounted_id(1, 1);
        let second = namespace.__runtime_mounted_id(2, 1);
        let contributions = || {
            vec![
                (
                    first.clone(),
                    contribution(0, "a", 0, EditingSessionPolicy::PreserveExact),
                ),
                (
                    second.clone(),
                    contribution(1, "b", 0, EditingSessionPolicy::PreserveExact),
                ),
            ]
        };
        let mut capacity = EditingRegistry::new(1, 1);
        assert_eq!(
            capacity.initial_reconcile(&namespace, contributions()),
            Err(EditingReconcileError::Capacity)
        );
        assert!(capacity.active.is_empty());

        let mut exhausted = EditingRegistry::new(2, 1);
        exhausted.next_session = NonZeroU64::new(u64::MAX);
        assert_eq!(
            exhausted.initial_reconcile(&namespace, contributions()),
            Err(EditingReconcileError::SessionGenerationExhausted)
        );
        assert!(exhausted.active.is_empty());
    }

    #[test]
    fn reset_retires_generation_and_old_chain_drains_without_aba_revival() {
        let namespace = RuntimeNamespace::__runtime_new();
        let owner = namespace.__runtime_mounted_id(1, 1);
        let mut registry = EditingRegistry::new(4, 4);
        registry
            .initial_reconcile(
                &namespace,
                vec![(
                    owner.clone(),
                    contribution(0, "ab", 2, EditingSessionPolicy::PreserveExact),
                )],
            )
            .unwrap_or_else(|_| unreachable!("fixture session reconciles"));
        let prepared = registry
            .prepare_insert(&namespace, &owner, "x", None)
            .unwrap_or_else(|_| unreachable!("fixture insert prepares"));
        let retired_generation = prepared.origin.session.clone();

        registry
            .reconcile(
                &namespace,
                vec![(
                    owner.clone(),
                    contribution(0, "ab", 0, EditingSessionPolicy::Reset),
                )],
                &crate::queue::ApplicationActionOrigin::Ordinary,
                None,
            )
            .unwrap_or_else(|_| unreachable!("fixture reset reconciles"));
        assert_ne!(registry.active[&owner].generation, retired_generation);
        assert_eq!(registry.draining.len(), 1);
        assert_eq!(registry.draining[0].generation, retired_generation);

        let resolution = EditResolution::rejected(prepared.origin.request.clone(), snapshot(0));
        registry
            .reconcile(
                &namespace,
                vec![(
                    owner.clone(),
                    contribution(0, "ab", 0, EditingSessionPolicy::PreserveExact),
                )],
                &crate::queue::ApplicationActionOrigin::Edit(prepared.origin),
                Some(resolution),
            )
            .unwrap_or_else(|_| unreachable!("retired rejection reconciles"));
        assert!(registry.draining.is_empty());
        assert_eq!(registry.active[&owner].selection.active(), 0);
    }

    #[test]
    fn retired_acceptance_requires_a_matching_authoritative_presentation() {
        let namespace = RuntimeNamespace::__runtime_new();
        let owner = namespace.__runtime_mounted_id(1, 1);
        let mut registry = EditingRegistry::new(2, 2);
        registry
            .initial_reconcile(
                &namespace,
                vec![(
                    owner.clone(),
                    contribution(0, "ab", 2, EditingSessionPolicy::PreserveExact),
                )],
            )
            .unwrap_or_else(|_| unreachable!("fixture session reconciles"));
        let prepared = registry
            .prepare_insert(&namespace, &owner, "x", None)
            .unwrap_or_else(|_| unreachable!("fixture insert prepares"));
        registry
            .reconcile(
                &namespace,
                Vec::new(),
                &crate::queue::ApplicationActionOrigin::Ordinary,
                None,
            )
            .unwrap_or_else(|_| unreachable!("fixture retirement reconciles"));
        assert_eq!((registry.active.len(), registry.draining.len()), (0, 1));
        let resolution = EditResolution::accepted(prepared.origin.request.clone(), snapshot(1));
        assert_eq!(
            registry.reconcile(
                &namespace,
                Vec::new(),
                &crate::queue::ApplicationActionOrigin::Edit(prepared.origin),
                Some(resolution),
            ),
            Err(EditingReconcileError::InconsistentResolution)
        );
    }

    #[test]
    fn owner_slot_and_document_aba_drain_without_transferring_session_authority() {
        let namespace = RuntimeNamespace::__runtime_new();
        let retired_owner = namespace.__runtime_mounted_id(1, 1);
        let replacement_owner = namespace.__runtime_mounted_id(1, 2);
        let mut registry = EditingRegistry::new(3, 2);
        registry
            .initial_reconcile(
                &namespace,
                vec![(
                    retired_owner.clone(),
                    contribution(0, "ab", 2, EditingSessionPolicy::PreserveExact),
                )],
            )
            .unwrap_or_else(|_| unreachable!("retired fixture session reconciles"));
        let prepared = registry
            .prepare_insert(&namespace, &retired_owner, "x", None)
            .unwrap_or_else(|_| unreachable!("retired fixture edit prepares"));

        registry
            .reconcile(
                &namespace,
                vec![(
                    replacement_owner.clone(),
                    contribution(0, "ab", 0, EditingSessionPolicy::PreserveExact),
                )],
                &crate::queue::ApplicationActionOrigin::Ordinary,
                None,
            )
            .unwrap_or_else(|_| unreachable!("replacement presentation reconciles"));
        let replacement_generation = registry.active[&replacement_owner].generation.clone();
        assert_eq!((registry.active.len(), registry.draining.len()), (1, 1));
        assert_eq!(registry.active[&replacement_owner].selection.active(), 0);

        registry
            .reconcile(
                &namespace,
                vec![(
                    replacement_owner.clone(),
                    contribution(0, "ab", 0, EditingSessionPolicy::PreserveExact),
                )],
                &crate::queue::ApplicationActionOrigin::Edit(prepared.origin.clone()),
                Some(EditResolution::rejected(
                    prepared.origin.request,
                    snapshot(0),
                )),
            )
            .unwrap_or_else(|_| unreachable!("retired edit rejection drains"));
        assert!(registry.draining.is_empty());
        assert_eq!(
            registry.active[&replacement_owner].generation,
            replacement_generation
        );
        assert_eq!(registry.active[&replacement_owner].selection.active(), 0);
    }

    #[test]
    fn overlapping_transformed_suffix_is_rejected_without_rewriting_it() {
        let namespace = RuntimeNamespace::__runtime_new();
        let request = namespace.__runtime_edit_request_id(2);
        let predecessor = Some(namespace.__runtime_edit_request_id(1));
        let selection = EditSelection::collapsed("ab", 2, TextAffinity::Upstream)
            .unwrap_or_else(|_| unreachable!("fixture selection is valid"));
        let mut pending = [PendingEdit {
            request,
            predecessor,
            base_snapshot: snapshot(0),
            proposed_text: Arc::from("ab"),
            proposed_selection: selection,
            replacement_start: 2,
            replacement_end: 3,
            replacement_text: Arc::from(""),
        }];

        assert!(!rebase_pending_suffix("abX", &mut pending, 2, 3, 3));
        assert_eq!(pending[0].replacement_start, 2);
        assert_eq!(pending[0].replacement_end, 3);
        assert_eq!(pending[0].proposed_text.as_ref(), "ab");
    }
}
