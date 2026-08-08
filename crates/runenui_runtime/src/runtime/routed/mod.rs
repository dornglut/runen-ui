mod admission;
mod commit;
mod defaults;
mod dispatch;
mod failure;
mod transaction;

use runenui_core::{
    EventSource, HostProtocol, InputModality, SemanticCommandEvent, UiEvent, WidgetInvalidation,
};

use super::Runtime;
use crate::{
    MountedNodeId, TraceContext, TraceEventContext, TraceEventFamily, TraceRecordKind,
    TraceRouteSnapshot, TraceRoutedIntegrityFailure,
    queue::SemanticCommandEnvelope,
    trace::{MandatoryTracePlan, TraceRecordDraft},
};
pub(crate) use dispatch::PointerDispatchFacts;
pub(crate) use transaction::{RoutedFailureLineage, RoutedIngressFacts, RoutedTransaction};

impl<State, Action, Protocol: HostProtocol> Runtime<State, Action, Protocol> {
    pub(crate) fn process_semantic_command(&mut self, envelope: SemanticCommandEnvelope) {
        let SemanticCommandEnvelope {
            sequence,
            target,
            command,
            origin,
            instant,
            causal_parent,
            trace_reservation,
        } = envelope;
        let facts = RoutedIngressFacts::new(
            sequence,
            target,
            origin,
            instant,
            TraceEventContext::new(TraceEventFamily::SemanticCommand, true),
            causal_parent,
            trace_reservation,
        );
        let Some(mut transaction) = (if is_focus_command(command) {
            self.begin_focus_routed_transaction(facts)
        } else {
            self.begin_routed_transaction(facts)
        }) else {
            return;
        };
        let event = UiEvent::SemanticCommand(SemanticCommandEvent::__runtime_new(command, origin));
        let routed = self.invoke_routed_callbacks(&mut transaction, &event, None);
        let defaulted =
            routed.and_then(|()| self.apply_semantic_default(&mut transaction, command));
        if let Err(failure) = defaulted {
            let current = transaction.failure_current_target.clone();
            self.poison_transaction(&transaction, failure, current.as_ref());
            return;
        }
        let failure_facts = transaction.failure_facts();
        if self.commit_routed_transaction(transaction).is_err() {
            self.poison_routed_event(
                &failure_facts,
                TraceRoutedIntegrityFailure::CommitInvariantFailure,
                None,
            );
        }
    }

    pub(crate) fn begin_routed_transaction(
        &mut self,
        facts: RoutedIngressFacts,
    ) -> Option<RoutedTransaction<Action>> {
        self.begin_routed_transaction_with_trace(facts, MandatoryTracePlan::none())
    }

    pub(crate) fn begin_routed_transaction_with_trace(
        &mut self,
        facts: RoutedIngressFacts,
        additional_trace: MandatoryTracePlan,
    ) -> Option<RoutedTransaction<Action>> {
        self.try_begin_routed_transaction_with_trace(facts, additional_trace)
            .ok()
    }

    pub(crate) fn try_begin_routed_transaction_with_trace(
        &mut self,
        facts: RoutedIngressFacts,
        additional_trace: MandatoryTracePlan,
    ) -> Result<RoutedTransaction<Action>, RoutedFailureLineage> {
        self.try_begin_routed_transaction_with_trace_and_default_commands(
            facts,
            additional_trace,
            0,
        )
    }

    pub(crate) fn try_begin_routed_transaction_with_trace_and_default_commands(
        &mut self,
        facts: RoutedIngressFacts,
        additional_trace: MandatoryTracePlan,
        mandatory_default_commands: usize,
    ) -> Result<RoutedTransaction<Action>, RoutedFailureLineage> {
        let (route, admission) = self.prepare_routed_route_with_default_commands(
            &facts,
            additional_trace,
            mandatory_default_commands,
        )?;
        let pointer_callback_targets = route.clone();
        Ok(self.start_routed_transaction(facts, route, pointer_callback_targets, admission))
    }

    #[allow(clippy::too_many_arguments)]
    pub(in crate::runtime) fn begin_pointer_routed_transaction(
        &mut self,
        facts: RoutedIngressFacts,
        include_ordinary_route: bool,
        target_only: &[MountedNodeId],
        deferred_target_only: &[MountedNodeId],
        deferred_invocations: usize,
        additional_trace: MandatoryTracePlan,
        may_focus: bool,
    ) -> Option<RoutedTransaction<Action>> {
        let (route, admission) = self.prepare_pointer_routed_route(
            &facts,
            include_ordinary_route,
            target_only,
            deferred_target_only,
            deferred_invocations,
            additional_trace,
            may_focus,
        )?;
        let mut pointer_callback_targets = route.clone();
        for target in target_only {
            if !pointer_callback_targets.contains(target) {
                pointer_callback_targets.push(target.clone());
            }
        }
        Some(self.start_routed_transaction(facts, route, pointer_callback_targets, admission))
    }

    fn start_routed_transaction(
        &mut self,
        facts: RoutedIngressFacts,
        route: Vec<MountedNodeId>,
        pointer_callback_targets: Vec<MountedNodeId>,
        admission: admission::RoutedTransactionAdmissionPlan,
    ) -> RoutedTransaction<Action> {
        let target_trace = self.tree.trace_target(&facts.target);
        let parent = if self.trace.is_enabled() {
            let started = self.trace.record_draft(
                TraceRecordDraft::routed_fact(
                    TraceRecordKind::RoutedEventStarted,
                    facts.instant,
                    TraceContext::routed_event(facts.event),
                )
                .with_work_sequence(Some(facts.sequence))
                .with_causal_parent(facts.causal_parent)
                .with_target(Some(target_trace.clone()))
                .with_routed_endpoints(facts.target.clone(), None, facts.origin),
            );
            let trace_route = route
                .iter()
                .map(|target| self.tree.trace_target(target))
                .collect();
            self.trace.record_draft(
                TraceRecordDraft::routed_fact(
                    TraceRecordKind::RouteSnapshotCreated {
                        invocations: admission.route_invocations,
                    },
                    facts.instant,
                    TraceContext::routed_snapshot(
                        facts.event,
                        TraceRouteSnapshot::new(trace_route, None),
                    ),
                )
                .with_work_sequence(Some(facts.sequence))
                .with_causal_parent(started)
                .with_target(Some(target_trace.clone()))
                .with_routed_endpoints(facts.target.clone(), None, facts.origin),
            )
        } else {
            None
        };
        RoutedTransaction {
            sequence: facts.sequence,
            target: facts.target,
            origin: facts.origin,
            instant: facts.instant,
            route,
            pointer_callback_targets,
            target_trace,
            parent,
            remaining_outputs: admission.max_outputs,
            remaining_default_commands: admission.mandatory_default_commands,
            propagation_stopped: false,
            default_prevented: false,
            collecting_notification_outputs: false,
            notification_outputs: Vec::new(),
            routed_outputs: Vec::new(),
            default_outputs: Vec::new(),
            mounted_work: Vec::new(),
            subscription_dirty: Vec::new(),
            pointer_capture_requests: Vec::new(),
            invalidation: WidgetInvalidation::NONE,
            failure_current_target: None,
            pending_modality: modality_for_source(facts.origin.source()),
        }
    }

    fn begin_focus_routed_transaction(
        &mut self,
        facts: RoutedIngressFacts,
    ) -> Option<RoutedTransaction<Action>> {
        let (route, admission) = self.prepare_focus_routed_route(&facts)?;
        let pointer_callback_targets = route.clone();
        Some(self.start_routed_transaction(facts, route, pointer_callback_targets, admission))
    }
}

const fn is_focus_command(command: runenui_core::SemanticCommand) -> bool {
    matches!(
        command,
        runenui_core::SemanticCommand::FocusNext
            | runenui_core::SemanticCommand::FocusPrevious
            | runenui_core::SemanticCommand::FocusLeft
            | runenui_core::SemanticCommand::FocusRight
            | runenui_core::SemanticCommand::FocusUp
            | runenui_core::SemanticCommand::FocusDown
            | runenui_core::SemanticCommand::RequestFocus
            | runenui_core::SemanticCommand::RestoreFocus
    )
}

const fn modality_for_source(source: EventSource) -> InputModality {
    match source {
        EventSource::Pointer => InputModality::Pointer,
        EventSource::Keyboard => InputModality::Keyboard,
        EventSource::Controller => InputModality::Controller,
        EventSource::Accessibility => InputModality::Accessibility,
        EventSource::Automation => InputModality::Automation,
        _ => InputModality::Programmatic,
    }
}
