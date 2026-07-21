use runenui_core::HostProtocol;

use super::{super::Runtime, transaction::RoutedCommandFacts};
use crate::{
    MountedNodeId, TraceRoutedAdmissionRejection, TraceRoutedIntegrityFailure,
    mounted::{RouteBuildError, TargetStatus},
    queue::QueueCommitError,
    trace::MandatoryTracePlan,
    work::{MountedCallbackPreflightError, WorkFamily},
};

pub(super) struct RoutedTransactionAdmissionPlan {
    pub(super) route_invocations: usize,
    pub(super) max_outputs: usize,
    queue_slots: usize,
    pub(super) trace: MandatoryTracePlan,
}

impl RoutedTransactionAdmissionPlan {
    fn checked(
        route_invocations: usize,
        max_outputs: usize,
    ) -> Result<Self, TraceRoutedAdmissionRejection> {
        let queue_slots = max_outputs
            .checked_mul(2)
            .ok_or(TraceRoutedAdmissionRejection::CheckedArithmeticOverflow)?;
        let trace = MandatoryTracePlan::routed_event(route_invocations, max_outputs)
            .ok_or(TraceRoutedAdmissionRejection::CheckedArithmeticOverflow)?;
        Ok(Self {
            route_invocations,
            max_outputs,
            queue_slots,
            trace,
        })
    }

    fn preflight<State, Action, Protocol: HostProtocol>(
        &self,
        runtime: &Runtime<State, Action, Protocol>,
    ) -> Result<(), TraceRoutedAdmissionRejection> {
        if self.max_outputs == 0 {
            return Err(TraceRoutedAdmissionRejection::TransactionOutputs);
        }
        if runtime.next_generation().is_none() {
            return Err(TraceRoutedAdmissionRejection::ReconciliationGenerationExhausted);
        }
        runtime
            .queue
            .preflight_commit(self.queue_slots)
            .map_err(|error| match error {
                QueueCommitError::Full => TraceRoutedAdmissionRejection::WaitingEnvelopes,
                QueueCommitError::SequenceExhausted => {
                    TraceRoutedAdmissionRejection::WorkSequenceExhausted
                }
            })?;
        runtime
            .work
            .preflight_mounted_callback(self.max_outputs)
            .map_err(|error| match error {
                MountedCallbackPreflightError::FamilyFull(WorkFamily::LocalTask) => {
                    TraceRoutedAdmissionRejection::LocalTasks
                }
                MountedCallbackPreflightError::FamilyFull(WorkFamily::SendTask) => {
                    TraceRoutedAdmissionRejection::SendTasks
                }
                MountedCallbackPreflightError::FamilyFull(WorkFamily::Timer) => {
                    TraceRoutedAdmissionRejection::Timers
                }
                MountedCallbackPreflightError::FamilyFull(
                    WorkFamily::Subscription | WorkFamily::HostRequest,
                ) => unreachable!("mounted callback preflight excludes these families"),
                MountedCallbackPreflightError::GenerationExhausted => {
                    TraceRoutedAdmissionRejection::WorkGenerationExhausted
                }
            })?;
        Ok(())
    }
}

impl<State, Action, Protocol: HostProtocol> Runtime<State, Action, Protocol> {
    pub(super) fn prepare_routed_route(
        &mut self,
        facts: &RoutedCommandFacts,
    ) -> Option<(Vec<MountedNodeId>, RoutedTransactionAdmissionPlan)> {
        let target_status = self.tree.target_status(&facts.target);
        if target_status != TargetStatus::Live {
            self.record_processing_target_rejection(facts, target_status);
            return None;
        }
        let route = match self.tree.event_route(&facts.target) {
            Ok(route) => route,
            Err(RouteBuildError::Target(status)) => {
                self.record_processing_target_rejection(facts, status);
                return None;
            }
            Err(RouteBuildError::BrokenTopology) => {
                self.poison_routed_facts(facts, TraceRoutedIntegrityFailure::BrokenTopology, None);
                return None;
            }
            Err(RouteBuildError::BridgeMismatch) => {
                self.poison_routed_facts(
                    facts,
                    TraceRoutedIntegrityFailure::EventBridgeMismatch,
                    None,
                );
                return None;
            }
        };
        if self.tree.preflight_event_bridges(&route).is_err() {
            self.poison_routed_facts(
                facts,
                TraceRoutedIntegrityFailure::EventBridgeMismatch,
                None,
            );
            return None;
        }
        let Some(route_invocations) = route
            .len()
            .checked_mul(2)
            .and_then(|count| count.checked_sub(1))
        else {
            self.handle_routed_admission_rejection(
                TraceRoutedAdmissionRejection::CheckedArithmeticOverflow,
                facts,
            );
            return None;
        };
        let admission = RoutedTransactionAdmissionPlan::checked(
            route_invocations,
            self.limits.transaction_outputs(),
        )
        .and_then(|plan| {
            plan.preflight(self)?;
            Ok(plan)
        });
        let admission = match admission {
            Ok(plan) => plan,
            Err(capacity) => {
                self.handle_routed_admission_rejection(capacity, facts);
                return None;
            }
        };
        if !self
            .trace
            .can_replace_reservation(facts.trace_reservation, admission.trace)
        {
            self.handle_routed_admission_rejection(
                TraceRoutedAdmissionRejection::TraceSequenceExhausted,
                facts,
            );
            return None;
        }
        self.trace.release_reservation(facts.trace_reservation);
        Some((route, admission))
    }
}
