use super::{
    Arc, FrameworkServiceCancelError, FrameworkServiceRef, FrameworkServiceResponseCompletion,
    FrameworkServiceResponseError, FrameworkServiceToken, MandatoryTracePlan, QueueCommitError,
    Runtime, RuntimeStatus, RuntimeTerminalReason, TraceRecordKind, WorkFamily, WorkSequence,
};
use crate::{
    TraceActionIdentity, TraceContext,
    queue::{ApplicationActionEnvelope, ApplicationActionOrigin, FrameworkServiceResponseEnvelope},
    runtime::RoutedTransaction,
    trace::TraceRecordDraft,
};
use runenui_core::{
    __runtime::{FrameworkServiceBinding, FrameworkServiceEffect, MountedEffect},
    ClipboardClassification, ClipboardWritePurpose, CursorShape, DragDropPhase, EditKind,
    EventSource, FrameworkServiceRequest, FrameworkServiceResponse,
};

impl<State, Action, Protocol: runenui_core::HostProtocol> Runtime<State, Action, Protocol> {
    pub(crate) fn synchronize_input_method_after_publication(
        &mut self,
        causal_parent: Option<crate::TraceSequence>,
        instant: runenui_core::MonotonicInstant,
    ) {
        if let Some((owner, effect)) = self.current_input_method_service()
            && self.prepare_framework_service(&effect.request, &effect.binding)
        {
            if matches!(
                &effect.request,
                FrameworkServiceRequest::InputMethod { enabled: true, .. }
            ) {
                self.framework_ime_may_be_enabled = true;
            }
            if self
                .commit_framework_service_effects(vec![(owner, effect)], causal_parent, instant)
                .is_err()
            {
                self.enter_terminal(RuntimeTerminalReason::Poisoned, 0);
            }
        }
    }

    pub(crate) fn stage_committed_framework_services(
        &mut self,
        transaction: &mut RoutedTransaction<Action>,
    ) {
        if let Some((owner, effect)) = self.current_input_method_service()
            && transaction.consume_mandatory_default_command().is_ok()
            && self.prepare_framework_service(&effect.request, &effect.binding)
        {
            if matches!(
                &effect.request,
                FrameworkServiceRequest::InputMethod { enabled: true, .. }
            ) {
                self.framework_ime_may_be_enabled = true;
            }
            transaction
                .mounted_work
                .push((owner, MountedEffect::FrameworkService(effect)));
        }

        // Cursor policy follows committed physical hit facts only. Focus-only and
        // automation routes must not manufacture pointer cursor transitions.
        if transaction.origin.source() != EventSource::Pointer {
            return;
        }
        let Some(surface_context) = transaction.pointer_surface_context.clone() else {
            return;
        };
        if self
            .surface_publication
            .current_surface_input_context()
            .as_ref()
            != Some(&surface_context)
        {
            return;
        }
        let Some((owner, effect)) =
            self.cursor_service_for(transaction.pointer_cursor_target.as_ref(), &surface_context)
        else {
            return;
        };
        if transaction.consume_mandatory_default_command().is_ok()
            && self.prepare_framework_service(&effect.request, &effect.binding)
        {
            transaction
                .mounted_work
                .push((owner, MountedEffect::FrameworkService(effect)));
        }
        self.stage_drag_drop_service(transaction, &surface_context);
    }

    fn cursor_service_for(
        &mut self,
        physical_target: Option<&crate::MountedNodeId>,
        surface_context: &runenui_core::SurfaceInputContext,
    ) -> Option<(crate::MountedNodeId, FrameworkServiceEffect)> {
        let owner = physical_target
            .or_else(|| self.focus.focused_node())
            .or_else(|| self.tree.root_id())
            .cloned()?;
        let shape = if self.editing.has_owner(&owner) {
            CursorShape::Text
        } else if self
            .tree
            .activation(&owner)
            .is_ok_and(|activation| activation.enabled() && activation.is_actionable())
        {
            CursorShape::Pointer
        } else {
            CursorShape::Default
        };
        let request = FrameworkServiceRequest::Cursor {
            shape,
            visible: true,
        };
        let binding = FrameworkServiceBinding::__runtime_new(
            owner.clone(),
            self.surface_publication.surface_id().clone(),
            Some(surface_context.clone()),
            None,
            None,
            None,
            None,
        );
        Some((
            owner,
            FrameworkServiceEffect::__runtime_new(request, binding),
        ))
    }

    fn stage_drag_drop_service(
        &mut self,
        transaction: &mut RoutedTransaction<Action>,
        surface_context: &runenui_core::SurfaceInputContext,
    ) {
        let Some(offer) = transaction.drag_drop_offer else {
            return;
        };
        let accepted = transaction.drag_drop_acceptor.is_some();
        if !matches!(offer.phase(), DragDropPhase::Hover | DragDropPhase::Cancel) && !accepted {
            return;
        }
        let Some(owner) = transaction.pointer_cursor_target.clone() else {
            return;
        };
        let editing = self.editing.framework_service_context(&owner);
        if self.editing.has_owner(&owner) && editing.is_none() {
            return;
        }
        let composition = (self.composition.owner() == Some(&owner))
            .then(|| self.composition.generation().cloned())
            .flatten();
        let binding = FrameworkServiceBinding::__runtime_new(
            owner.clone(),
            self.surface_publication.surface_id().clone(),
            Some(surface_context.clone()),
            editing.as_ref().map(|context| context.session.clone()),
            editing.as_ref().map(|context| context.snapshot),
            editing.as_ref().map(|context| context.selection),
            composition,
        );
        let request = FrameworkServiceRequest::DragDrop {
            source: transaction.sequence,
            phase: offer.phase(),
            payload: offer.payload(),
            accepted,
        };
        if transaction.consume_mandatory_default_command().is_ok()
            && self.prepare_framework_service(&request, &binding)
        {
            transaction.mounted_work.push((
                owner,
                MountedEffect::FrameworkService(FrameworkServiceEffect::__runtime_new(
                    request, binding,
                )),
            ));
        }
    }

    fn current_input_method_service(
        &self,
    ) -> Option<(crate::MountedNodeId, FrameworkServiceEffect)> {
        let surface_context = self.surface_publication.current_surface_input_context()?;
        let focused = self
            .focus
            .focused_node()
            .filter(|owner| self.tree.target_status(owner) == crate::mounted::TargetStatus::Live)
            .cloned();
        let owner = focused.clone().or_else(|| self.tree.root_id().cloned())?;
        let editing = focused
            .as_ref()
            .and_then(|owner| self.editing.input_method_context(owner));
        let composition = focused.as_ref().and_then(|owner| {
            (self.composition.owner() == Some(owner))
                .then(|| self.composition.generation().cloned())
                .flatten()
        });
        let candidate_area = editing.as_ref().and_then(|context| {
            if !context.enabled
                || context
                    .preedit
                    .as_ref()
                    .is_some_and(|preedit| composition.as_ref() != Some(preedit.generation()))
            {
                return None;
            }
            self.surface_publication
                .text_candidate_area(
                    &owner,
                    context.snapshot,
                    &context.source,
                    context.selection,
                    context.preedit.clone(),
                )
                .ok()
        });
        let enabled = focused.is_some() && editing.is_some() && candidate_area.is_some();
        if !enabled && !self.has_unresolved_enabled_input_method() {
            return None;
        }
        let binding = FrameworkServiceBinding::__runtime_new(
            owner.clone(),
            self.surface_publication.surface_id().clone(),
            Some(surface_context),
            if enabled {
                editing.as_ref().map(|context| context.session.clone())
            } else {
                None
            },
            if enabled {
                editing.as_ref().map(|context| context.snapshot)
            } else {
                None
            },
            if enabled {
                editing.as_ref().map(|context| context.selection)
            } else {
                None
            },
            enabled.then(|| composition.clone()).flatten(),
        );
        let request = FrameworkServiceRequest::InputMethod {
            enabled,
            candidate_area: if enabled { candidate_area } else { None },
            composition: enabled.then(|| composition.clone()).flatten(),
        };
        Some((
            owner,
            FrameworkServiceEffect::__runtime_new(request, binding),
        ))
    }

    fn has_unresolved_enabled_input_method(&self) -> bool {
        self.framework_ime_may_be_enabled
            || self.framework_services.iter().any(|service| {
                matches!(
                    &service.request,
                    FrameworkServiceRequest::InputMethod { enabled: true, .. }
                )
            })
    }

    fn prepare_framework_service(
        &mut self,
        request: &FrameworkServiceRequest,
        binding: &FrameworkServiceBinding,
    ) -> bool {
        if !matches!(
            request,
            FrameworkServiceRequest::InputMethod { .. } | FrameworkServiceRequest::Cursor { .. }
        ) {
            return true;
        }
        let kind = request.response_kind();
        self.cancel_conflicting_state_services(kind, request, binding);
        if self
            .framework_services
            .iter()
            .any(|service| &service.request == request && &service.binding == binding)
            || self
                .framework_service_satisfied
                .iter()
                .any(|(current, current_binding)| current == request && current_binding == binding)
        {
            return false;
        }
        true
    }

    fn cancel_conflicting_state_services(
        &mut self,
        kind: runenui_core::FrameworkServiceResponseKind,
        request: &FrameworkServiceRequest,
        binding: &FrameworkServiceBinding,
    ) {
        let generations: Vec<_> = self
            .framework_services
            .iter()
            .filter(|service| {
                service.expected == kind
                    && (&service.request != request || &service.binding != binding)
            })
            .map(|service| service.generation)
            .collect();
        for generation in generations {
            if let Some(identity) = self.trace_work_identity(generation) {
                self.record_work_fact(TraceRecordKind::FrameworkServiceCancelled, identity);
            }
            self.revoke_generation(generation);
        }
        self.framework_service_satisfied
            .retain(|(current, current_binding)| {
                current.response_kind() != kind
                    || (current == request && current_binding == binding)
            });
    }

    pub(crate) fn framework_service_binding_is_current(
        &self,
        binding: &runenui_core::__runtime::FrameworkServiceBinding,
        request: &FrameworkServiceRequest,
    ) -> bool {
        let exact_owner =
            self.tree.target_status(binding.owner()) == crate::mounted::TargetStatus::Live;
        let exact_surface = self
            .surface_publication
            .validate_surface_id(binding.surface())
            .is_ok();
        let exact_surface_context = binding.surface_context().is_some_and(|context| {
            self.surface_publication
                .current_surface_input_context()
                .as_ref()
                == Some(context)
        });
        let requires_composition =
            matches!(
                request,
                FrameworkServiceRequest::ClipboardReadText { .. }
                    | FrameworkServiceRequest::ClipboardWriteText { .. }
                    | FrameworkServiceRequest::InputMethod { enabled: true, .. }
            ) || (matches!(request, FrameworkServiceRequest::DragDrop { .. })
                && binding.editing_session().is_some());
        let current_owner_composition = (self.composition.owner() == Some(binding.owner()))
            .then(|| self.composition.generation().cloned())
            .flatten();
        let exact_composition =
            !requires_composition || binding.composition().cloned() == current_owner_composition;
        let exact_editing = self.framework_service_edit_binding_is_current(binding, request);
        let requires_focus = matches!(
            request,
            FrameworkServiceRequest::ClipboardReadText { .. }
                | FrameworkServiceRequest::ClipboardWriteText { .. }
                | FrameworkServiceRequest::InputMethod { enabled: true, .. }
        ) || (matches!(request, FrameworkServiceRequest::DragDrop { .. })
            && binding.editing_session().is_some());
        let exact_focus = !requires_focus || self.focus.focused_node() == Some(binding.owner());
        let requires_editing = requires_focus;
        let has_required_editing = !requires_editing || binding.editing_session().is_some();
        let exact_candidate_area =
            self.framework_service_candidate_area_is_current(binding, request);
        exact_owner
            && exact_surface
            && exact_surface_context
            && exact_composition
            && exact_editing
            && exact_focus
            && has_required_editing
            && exact_candidate_area
    }

    fn framework_service_edit_binding_is_current(
        &self,
        binding: &FrameworkServiceBinding,
        request: &FrameworkServiceRequest,
    ) -> bool {
        match (
            binding.editing_session(),
            binding.document_snapshot(),
            binding.selection(),
        ) {
            (Some(session), Some(snapshot), Some(selection))
                if matches!(
                    request,
                    FrameworkServiceRequest::InputMethod { enabled: true, .. }
                ) =>
            {
                self.editing
                    .input_method_context(binding.owner())
                    .is_some_and(|context| {
                        context.session == *session
                            && context.snapshot == snapshot
                            && context.selection == selection
                    })
            }
            (Some(session), Some(snapshot), Some(selection)) => self
                .editing
                .framework_service_binding_matches(binding.owner(), session, snapshot, selection),
            (None, None, None) => true,
            _ => false,
        }
    }

    fn framework_service_candidate_area_is_current(
        &self,
        binding: &FrameworkServiceBinding,
        request: &FrameworkServiceRequest,
    ) -> bool {
        match request {
            FrameworkServiceRequest::InputMethod {
                enabled: true,
                candidate_area: Some(candidate_area),
                composition,
            } => self
                .editing
                .input_method_context(binding.owner())
                .filter(|context| {
                    let composition_matches = context.preedit.as_ref().map_or_else(
                        || binding.composition() == composition.as_ref(),
                        |projection| composition.as_ref() == Some(projection.generation()),
                    );
                    Some(&context.session) == binding.editing_session()
                        && Some(context.snapshot) == binding.document_snapshot()
                        && Some(context.selection) == binding.selection()
                        && context.enabled
                        && composition_matches
                })
                .and_then(|context| {
                    self.surface_publication
                        .text_candidate_area(
                            binding.owner(),
                            context.snapshot,
                            &context.source,
                            context.selection,
                            context.preedit,
                        )
                        .ok()
                })
                .is_some_and(|current| current == *candidate_area),
            FrameworkServiceRequest::InputMethod { enabled: true, .. } => false,
            _ => true,
        }
    }

    pub(crate) fn cancel_stale_framework_services(&mut self) {
        let stale: Vec<_> = self
            .framework_services
            .iter()
            .filter_map(|request| {
                (!self.framework_service_binding_is_current(&request.binding, &request.request))
                    .then_some(request.generation)
            })
            .collect();
        for generation in stale {
            if let Some(identity) = self.trace_work_identity(generation) {
                self.record_work_fact(TraceRecordKind::FrameworkServiceCancelled, identity);
            }
            self.revoke_generation(generation);
        }
    }

    pub(crate) fn pending_framework_services(&self) -> Vec<FrameworkServiceRef<'_>> {
        self.framework_services
            .iter()
            .filter(|request| {
                self.work
                    .is_running_family(request.generation, WorkFamily::FrameworkService)
            })
            .map(|request| FrameworkServiceRef {
                token: FrameworkServiceToken {
                    namespace: Arc::clone(&self.framework_service_namespace),
                    generation: request.generation,
                },
                request: &request.request,
                binding: &request.binding,
            })
            .collect()
    }

    pub(crate) fn complete_framework_service(
        &mut self,
        token: &FrameworkServiceToken,
        response: FrameworkServiceResponse,
    ) -> Result<WorkSequence, FrameworkServiceResponseError> {
        match self.status {
            RuntimeStatus::Closed => return Err(FrameworkServiceResponseError::Closed(response)),
            RuntimeStatus::Terminal(reason) => {
                return Err(FrameworkServiceResponseError::Terminal { response, reason });
            }
            RuntimeStatus::Running => {}
        }
        if !Arc::ptr_eq(&self.framework_service_namespace, &token.namespace) {
            return Err(FrameworkServiceResponseError::ForeignRuntime(response));
        }
        let Some(request) = self.framework_services.iter().find(|request| {
            request.generation == token.generation
                && self
                    .work
                    .is_running_family(request.generation, WorkFamily::FrameworkService)
        }) else {
            return Err(FrameworkServiceResponseError::Stale(response));
        };
        if request.expected != response.kind() {
            let identity = self
                .trace_work_identity(token.generation)
                .unwrap_or_else(|| unreachable!("live service has trace identity"));
            if !self.trace.can_admit(MandatoryTracePlan::one_fact()) {
                let reason = RuntimeTerminalReason::TraceSequenceExhausted;
                self.enter_terminal(reason, 0);
                return Err(FrameworkServiceResponseError::Terminal { response, reason });
            }
            self.record_work_fact(TraceRecordKind::FrameworkServiceResponseRejected, identity);
            return Err(FrameworkServiceResponseError::MismatchedKind(response));
        }
        if !self.callback_output_preflight(
            Some((token.generation, WorkFamily::FrameworkService)),
            MandatoryTracePlan::framework_service_completion(),
        ) {
            if matches!(self.status, RuntimeStatus::Running) {
                return Err(FrameworkServiceResponseError::Full(response));
            }
            let RuntimeStatus::Terminal(reason) = self.status else {
                unreachable!("callback preflight only closes through a terminal transition")
            };
            return Err(FrameworkServiceResponseError::Terminal { response, reason });
        }
        if !self
            .completion_ingress
            .claim_direct_framework_service_response(token.generation)
        {
            return Err(FrameworkServiceResponseError::Stale(response));
        }
        let identity = self
            .trace_work_identity(token.generation)
            .unwrap_or_else(|| unreachable!("live service has trace identity"));
        let queued = self.record_work_fact(
            TraceRecordKind::FrameworkServiceResponseQueued,
            identity.clone(),
        );
        let sequence = self
            .queue
            .push_framework_service_response(
                token.generation,
                response,
                identity,
                queued.or_else(|| self.work.trace_parent(token.generation)),
            )
            .unwrap_or_else(|_| unreachable!("framework service response was preflighted"));
        self.external_queue_commit_accepted();
        Ok(sequence)
    }

    pub(crate) fn framework_service_response_completion(
        &mut self,
        token: &FrameworkServiceToken,
        response: FrameworkServiceResponse,
    ) -> Result<FrameworkServiceResponseCompletion, FrameworkServiceResponseError> {
        match self.status {
            RuntimeStatus::Closed => return Err(FrameworkServiceResponseError::Closed(response)),
            RuntimeStatus::Terminal(reason) => {
                return Err(FrameworkServiceResponseError::Terminal { response, reason });
            }
            RuntimeStatus::Running => {}
        }
        if !Arc::ptr_eq(&self.framework_service_namespace, &token.namespace) {
            return Err(FrameworkServiceResponseError::ForeignRuntime(response));
        }
        if !self
            .work
            .is_running_family(token.generation, WorkFamily::FrameworkService)
            || !self
                .completion_ingress
                .framework_service_response_is_open(token.generation)
        {
            return Err(FrameworkServiceResponseError::Stale(response));
        }
        let Some(request) = self
            .framework_services
            .iter()
            .find(|request| request.generation == token.generation)
        else {
            return Err(FrameworkServiceResponseError::Stale(response));
        };
        if request.expected != response.kind() {
            let identity = self
                .trace_work_identity(token.generation)
                .unwrap_or_else(|| unreachable!("live service has trace identity"));
            if !self.trace.can_admit(MandatoryTracePlan::one_fact()) {
                let reason = RuntimeTerminalReason::TraceSequenceExhausted;
                self.enter_terminal(reason, 0);
                return Err(FrameworkServiceResponseError::Terminal { response, reason });
            }
            self.record_work_fact(TraceRecordKind::FrameworkServiceResponseRejected, identity);
            return Err(FrameworkServiceResponseError::MismatchedKind(response));
        }
        Ok(FrameworkServiceResponseCompletion::new(
            token.generation,
            response,
            self.completion_ingress.sender(),
            self.trace_work_identity(token.generation)
                .unwrap_or_else(|| unreachable!("live service has trace identity")),
            self.work.trace_parent(token.generation),
        ))
    }

    pub(crate) fn cancel_framework_service(
        &mut self,
        token: &FrameworkServiceToken,
    ) -> Result<WorkSequence, FrameworkServiceCancelError> {
        match self.status {
            RuntimeStatus::Closed => return Err(FrameworkServiceCancelError::Closed),
            RuntimeStatus::Terminal(reason) => {
                return Err(FrameworkServiceCancelError::Terminal(reason));
            }
            RuntimeStatus::Running => {}
        }
        if !Arc::ptr_eq(&self.framework_service_namespace, &token.namespace) {
            return Err(FrameworkServiceCancelError::ForeignRuntime);
        }
        if !self
            .work
            .is_running_family(token.generation, WorkFamily::FrameworkService)
        {
            return Err(FrameworkServiceCancelError::Stale);
        }
        match self.queue.preflight_commit(1) {
            Ok(()) => {}
            Err(QueueCommitError::Full) => return Err(FrameworkServiceCancelError::Full),
            Err(QueueCommitError::SequenceExhausted) => {
                let reason = RuntimeTerminalReason::WorkSequenceExhausted;
                self.enter_terminal(reason, 0);
                return Err(FrameworkServiceCancelError::Terminal(reason));
            }
        }
        if !self
            .trace
            .can_admit(MandatoryTracePlan::work_cancellation())
        {
            let reason = RuntimeTerminalReason::TraceSequenceExhausted;
            self.enter_terminal(reason, 0);
            return Err(FrameworkServiceCancelError::Terminal(reason));
        }
        let identity = self
            .trace_work_identity(token.generation)
            .unwrap_or_else(|| unreachable!("live service has trace identity"));
        let lineage = self.record_invalidation_facts(
            core::slice::from_ref(&identity),
            self.work.trace_parent(token.generation),
        );
        let (_, parent) = lineage
            .get(&token.generation.get())
            .cloned()
            .unwrap_or_else(|| unreachable!("service cancellation retains trace lineage"));
        let sequence = self
            .queue
            .push_cancellation(token.generation, identity, parent)
            .unwrap_or_else(|_| unreachable!("service cancellation was preflighted"));
        self.invalidate_generation_now(token.generation);
        self.external_queue_commit_accepted();
        Ok(sequence)
    }

    pub(crate) fn process_framework_service_response(
        &mut self,
        envelope: FrameworkServiceResponseEnvelope,
    ) -> Option<ApplicationActionEnvelope<Action>> {
        let FrameworkServiceResponseEnvelope {
            sequence,
            generation,
            response,
            trace_identity,
            causal_parent,
        } = envelope;
        if !self
            .work
            .is_running_family(generation, WorkFamily::FrameworkService)
        {
            self.record_work_fact_with_parent(
                TraceRecordKind::WorkCompletionRejectedStale,
                causal_parent,
                trace_identity,
            );
            self.completion_ingress
                .release_framework_service_response(generation);
            return None;
        }
        let Some(index) = self
            .framework_services
            .iter()
            .position(|request| request.generation == generation)
        else {
            self.record_work_fact_with_parent(
                TraceRecordKind::FrameworkServiceResponseRejected,
                causal_parent,
                trace_identity,
            );
            self.revoke_generation(generation);
            return None;
        };
        let request = &self.framework_services[index];
        let request_value = request.request.clone();
        let binding = request.binding.clone();
        if request.expected != response.kind()
            || !self.framework_service_binding_is_current(&binding, &request.request)
        {
            self.record_work_fact_with_parent(
                TraceRecordKind::FrameworkServiceResponseRejected,
                causal_parent,
                trace_identity,
            );
            self.revoke_generation(generation);
            return None;
        }
        let accepted = self.record_work_fact_from_envelope(
            TraceRecordKind::FrameworkServiceResponseAccepted,
            sequence,
            trace_identity.clone(),
        );
        if let Some((service, outcome)) = framework_service_trace_outcome(&request_value, &response)
        {
            self.record_work_fact_from_envelope(
                TraceRecordKind::FrameworkServiceResponseOutcome { service, outcome },
                sequence,
                trace_identity.clone(),
            );
        }
        self.note_framework_service_response(&request_value, &binding, &response);
        let prepared =
            match self.prepare_framework_service_edit(&request_value, &binding, &response) {
                Ok(prepared) => prepared,
                Err(reason) => {
                    self.enter_terminal(reason, 0);
                    self.revoke_generation(generation);
                    return None;
                }
            };
        let action_envelope = prepared.map(|prepared| {
            self.framework_service_edit_action_envelope(sequence, accepted, &binding, prepared)
        });
        self.record_work_fact_with_parent(
            TraceRecordKind::WorkCompletionMapped,
            accepted.or(causal_parent),
            trace_identity,
        );
        self.revoke_generation(generation);
        action_envelope
    }

    fn prepare_framework_service_edit(
        &mut self,
        request: &FrameworkServiceRequest,
        binding: &FrameworkServiceBinding,
        response: &FrameworkServiceResponse,
    ) -> Result<Option<crate::editing::PreparedEdit<Action>>, RuntimeTerminalReason> {
        let cut = matches!(
            (request, response),
            (
                FrameworkServiceRequest::ClipboardWriteText {
                    purpose: ClipboardWritePurpose::Cut,
                    ..
                },
                FrameworkServiceResponse::ClipboardWriteText(Ok(()))
            )
        );
        let paste = matches!(
            (request, response),
            (
                FrameworkServiceRequest::ClipboardReadText { .. },
                FrameworkServiceResponse::ClipboardReadText(Ok(_))
            )
        );
        if !cut && !paste {
            return Ok(None);
        }
        let action_trace =
            MandatoryTracePlan::application_action_base(self.focus.focused_node().is_some(), true);
        let Some(combined_plan) =
            MandatoryTracePlan::framework_service_completion().checked_add(action_trace)
        else {
            return Err(RuntimeTerminalReason::TraceSequenceExhausted);
        };
        if !self.trace.can_admit(combined_plan) {
            return Err(RuntimeTerminalReason::TraceSequenceExhausted);
        }
        let replacement = match (request, response) {
            (
                FrameworkServiceRequest::ClipboardWriteText {
                    purpose: ClipboardWritePurpose::Cut,
                    ..
                },
                FrameworkServiceResponse::ClipboardWriteText(Ok(())),
            ) => Some(""),
            (
                FrameworkServiceRequest::ClipboardReadText { max_bytes },
                FrameworkServiceResponse::ClipboardReadText(Ok(text)),
            ) if text.text().len() <= *max_bytes
                && clipboard_classification_admitted(
                    text.classification(),
                    self.editing.sensitivity(binding.owner()),
                ) =>
            {
                Some(text.text())
            }
            _ => None,
        };
        let Some(replacement) = replacement else {
            return Ok(None);
        };
        let kind = if cut { EditKind::Cut } else { EditKind::Paste };
        let namespace = self.tree.runtime_namespace();
        Ok(self
            .editing
            .prepare_framework_service_edit(&namespace, binding, replacement, kind)
            .ok())
    }

    fn framework_service_edit_action_envelope(
        &mut self,
        sequence: WorkSequence,
        accepted: Option<crate::TraceSequence>,
        binding: &FrameworkServiceBinding,
        prepared: crate::editing::PreparedEdit<Action>,
    ) -> ApplicationActionEnvelope<Action> {
        let target = self.tree.trace_target(binding.owner());
        let action_label = if self.trace.is_enabled() {
            self.trace_action_labeler
                .and_then(|labeler| labeler(&prepared.action))
        } else {
            None
        };
        let action_accepted = self.trace.record_draft(
            TraceRecordDraft::action_fact(
                TraceRecordKind::ActionSubmissionAccepted,
                self.now(),
                TraceContext::action_record(TraceActionIdentity::editing::<Action>(
                    action_label,
                    &prepared.origin,
                )),
            )
            .with_work_sequence(Some(sequence))
            .with_causal_parent(accepted)
            .with_target(Some(target.clone())),
        );
        ApplicationActionEnvelope {
            sequence,
            action: prepared.action,
            causal_parent: action_accepted.or(accepted),
            target: Some(target),
            origin: ApplicationActionOrigin::Edit(prepared.origin),
        }
    }

    fn note_framework_service_response(
        &mut self,
        request: &FrameworkServiceRequest,
        binding: &FrameworkServiceBinding,
        response: &FrameworkServiceResponse,
    ) {
        let kind = request.response_kind();
        match (request, response) {
            (
                FrameworkServiceRequest::InputMethod { enabled, .. },
                FrameworkServiceResponse::InputMethod(Ok(())),
            ) => {
                self.framework_ime_may_be_enabled = *enabled;
                self.framework_service_satisfied
                    .retain(|(current, _)| current.response_kind() != kind);
                self.framework_service_satisfied
                    .push((request.clone(), binding.clone()));
            }
            (FrameworkServiceRequest::Cursor { .. }, FrameworkServiceResponse::Cursor(Ok(())))
            | (
                FrameworkServiceRequest::InputMethod { .. },
                FrameworkServiceResponse::InputMethod(Err(_)),
            )
            | (FrameworkServiceRequest::Cursor { .. }, FrameworkServiceResponse::Cursor(Err(_))) => {
                self.framework_service_satisfied
                    .retain(|(current, _)| current.response_kind() != kind);
                if matches!(response, FrameworkServiceResponse::Cursor(Ok(()))) {
                    self.framework_service_satisfied
                        .push((request.clone(), binding.clone()));
                }
            }
            _ => {}
        }
    }
}

const fn clipboard_classification_admitted(
    classification: ClipboardClassification,
    sensitivity: Option<runenui_core::TextSensitivity>,
) -> bool {
    use runenui_core::TextSensitivity;

    match (classification, sensitivity) {
        (
            ClipboardClassification::Public,
            Some(TextSensitivity::Public | TextSensitivity::Secret),
        )
        | (ClipboardClassification::Sensitive, Some(TextSensitivity::Secret)) => true,
        // Unknown clipboard provenance is not safe to insert into any document.
        // Sensitive data must also never flow into a public document.
        _ => false,
    }
}

fn framework_service_trace_outcome(
    request: &FrameworkServiceRequest,
    response: &FrameworkServiceResponse,
) -> Option<(
    crate::TraceFrameworkServiceKind,
    crate::TraceFrameworkServiceOutcome,
)> {
    use crate::{TraceFrameworkServiceKind as Kind, TraceFrameworkServiceOutcome as Outcome};

    let kind = match request {
        FrameworkServiceRequest::ClipboardReadText { .. } => Kind::ClipboardReadText,
        FrameworkServiceRequest::ClipboardWriteText { purpose, .. } => {
            Kind::ClipboardWriteText(*purpose)
        }
        FrameworkServiceRequest::InputMethod { .. } => Kind::InputMethod,
        FrameworkServiceRequest::Cursor { .. } => Kind::Cursor,
        FrameworkServiceRequest::DragDrop { .. } => Kind::DragDrop,
        _ => return None,
    };
    let outcome = match (request, response) {
        (
            FrameworkServiceRequest::ClipboardReadText { .. },
            FrameworkServiceResponse::ClipboardReadText(Ok(text)),
        ) => Outcome::ClipboardText {
            classification: text.classification(),
            bytes: text.text().len(),
        },
        (
            _,
            FrameworkServiceResponse::ClipboardWriteText(Ok(()))
            | FrameworkServiceResponse::InputMethod(Ok(()))
            | FrameworkServiceResponse::Cursor(Ok(())),
        ) => Outcome::Succeeded,
        (
            _,
            FrameworkServiceResponse::ClipboardReadText(Err(failure))
            | FrameworkServiceResponse::ClipboardWriteText(Err(failure))
            | FrameworkServiceResponse::InputMethod(Err(failure))
            | FrameworkServiceResponse::Cursor(Err(failure))
            | FrameworkServiceResponse::DragDrop(Err(failure)),
        ) => Outcome::Failed(*failure),
        (
            FrameworkServiceRequest::DragDrop {
                phase,
                payload,
                accepted,
                ..
            },
            FrameworkServiceResponse::DragDrop(Ok(())),
        ) => Outcome::DragDrop {
            phase: *phase,
            payload: payload.kind(),
            items: payload.item_count().get(),
            accepted: *accepted,
        },
        _ => return None,
    };
    Some((kind, outcome))
}
