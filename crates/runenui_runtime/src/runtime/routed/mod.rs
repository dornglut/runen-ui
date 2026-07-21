mod admission;
mod commit;
mod defaults;
mod dispatch;
mod failure;
mod transaction;

use runenui_core::{HostProtocol, WidgetInvalidation};

use super::Runtime;
use crate::{TraceRecordKind, TraceRoutedIntegrityFailure, queue::SemanticCommandEnvelope};
use transaction::{RoutedCommandFacts, RoutedTransaction};

impl<State, Action, Protocol: HostProtocol> Runtime<State, Action, Protocol> {
    pub(crate) fn process_semantic_command(&mut self, envelope: SemanticCommandEnvelope) {
        let Some(mut transaction) = self.begin_routed_transaction(envelope) else {
            return;
        };
        let routed = self.invoke_routed_callbacks(&mut transaction);
        let defaulted = routed.and_then(|()| self.apply_semantic_default(&mut transaction));
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

    fn begin_routed_transaction(
        &mut self,
        envelope: SemanticCommandEnvelope,
    ) -> Option<RoutedTransaction<Action>> {
        let facts = RoutedCommandFacts::from(envelope);
        let (route, admission) = self.prepare_routed_route(&facts)?;
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
        Some(RoutedTransaction {
            sequence: facts.sequence,
            target: facts.target,
            command: facts.command,
            origin: facts.origin,
            instant: facts.instant,
            route,
            target_trace,
            parent,
            remaining_outputs: admission.max_outputs,
            propagation_stopped: false,
            default_prevented: false,
            routed_outputs: Vec::new(),
            default_outputs: Vec::new(),
            mounted_work: Vec::new(),
            subscription_dirty: Vec::new(),
            invalidation: WidgetInvalidation::NONE,
            failure_current_target: None,
        })
    }
}
