use runenui_core::{EventSource, HostProtocol};

use super::{
    super::{Runtime, RuntimeTerminalReason},
    transaction::{
        RoutedFailureFacts, RoutedFailureLineage, RoutedIngressFacts, RoutedTransaction,
    },
};
use crate::{
    MountedNodeId, TraceRecordKind, TraceRoutedAdmissionRejection, TraceRoutedIntegrityFailure,
    TraceTarget, TraceTargetRejection, mounted::TargetStatus,
};

impl<State, Action, Protocol: HostProtocol> Runtime<State, Action, Protocol> {
    pub(super) fn handle_routed_admission_rejection(
        &mut self,
        capacity: TraceRoutedAdmissionRejection,
        facts: &RoutedIngressFacts,
    ) -> RoutedFailureLineage {
        let failure_parent = self.trace.record_reserved_event(
            facts.trace_reservation,
            TraceRecordKind::RoutedEventAdmissionRejected { capacity },
            facts.sequence,
            facts.causal_parent,
            Some(self.tree.trace_target(&facts.target)),
            facts.instant,
            &facts.target,
            None,
            facts.origin,
        );
        let terminal = match capacity {
            TraceRoutedAdmissionRejection::WorkSequenceExhausted => {
                Some(RuntimeTerminalReason::WorkSequenceExhausted)
            }
            TraceRoutedAdmissionRejection::WorkGenerationExhausted => {
                Some(RuntimeTerminalReason::WorkGenerationExhausted)
            }
            TraceRoutedAdmissionRejection::ReconciliationGenerationExhausted => {
                Some(RuntimeTerminalReason::ReconciliationGenerationExhausted)
            }
            TraceRoutedAdmissionRejection::TraceSequenceExhausted => {
                Some(RuntimeTerminalReason::TraceSequenceExhausted)
            }
            TraceRoutedAdmissionRejection::CheckedArithmeticOverflow => {
                Some(RuntimeTerminalReason::Poisoned)
            }
            TraceRoutedAdmissionRejection::TransactionOutputs
            | TraceRoutedAdmissionRejection::WaitingEnvelopes
            | TraceRoutedAdmissionRejection::LocalTasks
            | TraceRoutedAdmissionRejection::SendTasks
            | TraceRoutedAdmissionRejection::Timers => None,
        };
        if let Some(reason) = terminal {
            self.enter_terminal_with_parent(reason, 0, failure_parent);
        }
        RoutedFailureLineage::new(failure_parent)
    }

    pub(super) fn record_processing_target_rejection(
        &mut self,
        facts: &RoutedIngressFacts,
        status: TargetStatus,
    ) -> RoutedFailureLineage {
        let outcome = match status {
            TargetStatus::Foreign => TraceTargetRejection::Foreign,
            TargetStatus::Stale => TraceTargetRejection::Stale,
            TargetStatus::Missing => TraceTargetRejection::Missing,
            TargetStatus::Live => unreachable!("live targets are not processing rejections"),
        };
        let kind = if outcome == TraceTargetRejection::Stale
            && facts.origin.source() == EventSource::Automation
        {
            TraceRecordKind::AutomationTargetStaleAfterResolution
        } else {
            TraceRecordKind::CommandProcessingRejected { outcome }
        };
        let failure_parent = self.trace.record_reserved_event(
            facts.trace_reservation,
            kind,
            facts.sequence,
            facts.causal_parent,
            Some(TraceTarget::new(facts.target.clone(), None)),
            facts.instant,
            &facts.target,
            None,
            facts.origin,
        );
        RoutedFailureLineage::new(failure_parent)
    }

    pub(super) fn poison_routed_facts(
        &mut self,
        facts: &RoutedIngressFacts,
        failure: TraceRoutedIntegrityFailure,
        current_target: Option<&MountedNodeId>,
    ) -> RoutedFailureLineage {
        let failure_parent = self.trace.record_reserved_event(
            facts.trace_reservation,
            TraceRecordKind::RoutedIntegrityFailed { failure },
            facts.sequence,
            facts.causal_parent,
            Some(TraceTarget::new(facts.target.clone(), None)),
            facts.instant,
            &facts.target,
            current_target,
            facts.origin,
        );
        self.enter_terminal_with_parent(RuntimeTerminalReason::Poisoned, 0, failure_parent);
        RoutedFailureLineage::new(failure_parent)
    }

    pub(crate) fn poison_transaction(
        &mut self,
        transaction: &RoutedTransaction<Action>,
        failure: TraceRoutedIntegrityFailure,
        current_target: Option<&MountedNodeId>,
    ) -> RoutedFailureLineage {
        self.poison_routed_event(&transaction.failure_facts(), failure, current_target)
    }

    pub(crate) fn poison_routed_event(
        &mut self,
        facts: &RoutedFailureFacts,
        failure: TraceRoutedIntegrityFailure,
        current_target: Option<&MountedNodeId>,
    ) -> RoutedFailureLineage {
        let failure_parent = self.trace.record_event(
            TraceRecordKind::RoutedIntegrityFailed { failure },
            facts.sequence,
            facts.causal_parent,
            Some(TraceTarget::new(facts.target.clone(), None)),
            facts.instant,
            &facts.target,
            current_target,
            facts.origin,
        );
        self.enter_terminal_with_parent(RuntimeTerminalReason::Poisoned, 0, failure_parent);
        RoutedFailureLineage::new(failure_parent)
    }
}
