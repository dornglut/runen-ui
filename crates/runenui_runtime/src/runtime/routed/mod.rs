mod admission;
mod commit;
mod defaults;
mod dispatch;
mod failure;
mod transaction;

use runenui_core::{HostProtocol, SemanticCommandEvent, UiEvent, WidgetInvalidation};

use super::Runtime;
use crate::{
    MountedNodeId, TraceRecordKind, TraceRoutedIntegrityFailure, queue::SemanticCommandEnvelope,
    trace::MandatoryTracePlan,
};
pub(in crate::runtime) use dispatch::PointerDispatchFacts;
pub(in crate::runtime) use transaction::{RoutedIngressFacts, RoutedTransaction};

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
            causal_parent,
            trace_reservation,
        );
        let Some(mut transaction) = self.begin_routed_transaction(facts) else {
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

    pub(in crate::runtime) fn begin_routed_transaction(
        &mut self,
        facts: RoutedIngressFacts,
    ) -> Option<RoutedTransaction<Action>> {
        self.begin_routed_transaction_with_trace(facts, MandatoryTracePlan::none())
    }

    pub(in crate::runtime) fn begin_routed_transaction_with_trace(
        &mut self,
        facts: RoutedIngressFacts,
        additional_trace: MandatoryTracePlan,
    ) -> Option<RoutedTransaction<Action>> {
        let (route, admission) = self.prepare_routed_route(&facts, additional_trace)?;
        let pointer_callback_targets = route.clone();
        Some(self.start_routed_transaction(facts, route, pointer_callback_targets, admission))
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
    ) -> Option<RoutedTransaction<Action>> {
        let (route, admission) = self.prepare_pointer_routed_route(
            &facts,
            include_ordinary_route,
            target_only,
            deferred_target_only,
            deferred_invocations,
            additional_trace,
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
        let started = self.trace.record_event(
            TraceRecordKind::RoutedEventStarted,
            facts.sequence,
            facts.causal_parent,
            Some(target_trace.clone()),
            facts.instant,
            &facts.target,
            None,
            facts.origin,
        );
        let parent = self.trace.record_event(
            TraceRecordKind::RouteSnapshotCreated {
                invocations: admission.route_invocations,
            },
            facts.sequence,
            started,
            Some(target_trace.clone()),
            facts.instant,
            &facts.target,
            None,
            facts.origin,
        );
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
        }
    }
}
