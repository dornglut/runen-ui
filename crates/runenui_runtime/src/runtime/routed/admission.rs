use runenui_core::HostProtocol;

use super::{super::Runtime, transaction::RoutedIngressFacts};
use crate::{
    MountedNodeId, TraceRoutedAdmissionRejection, TraceRoutedIntegrityFailure,
    mounted::{RouteBuildError, TargetStatus},
    queue::QueueCommitError,
    trace::MandatoryTracePlan,
    work::{MountedCallbackPreflightError, WorkFamily},
};

#[derive(Clone, Copy)]
pub(super) struct RoutedTransactionAdmissionPlan {
    pub(super) route_invocations: usize,
    pub(super) max_outputs: usize,
    queue_slots: usize,
    pub(super) trace: MandatoryTracePlan,
}

impl RoutedTransactionAdmissionPlan {
    fn checked(
        route_invocations: usize,
        admitted_invocations: usize,
        max_outputs: usize,
    ) -> Result<Self, TraceRoutedAdmissionRejection> {
        let queue_slots = max_outputs
            .checked_mul(2)
            .ok_or(TraceRoutedAdmissionRejection::CheckedArithmeticOverflow)?;
        let trace = MandatoryTracePlan::routed_event(admitted_invocations, max_outputs)
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
        facts: &RoutedIngressFacts,
        additional_trace: MandatoryTracePlan,
    ) -> Option<(Vec<MountedNodeId>, RoutedTransactionAdmissionPlan)> {
        self.prepare_routed_invocations(facts, true, &[], &[], 0, additional_trace)
    }

    pub(super) fn prepare_pointer_routed_route(
        &mut self,
        facts: &RoutedIngressFacts,
        include_ordinary_route: bool,
        target_only: &[MountedNodeId],
        deferred_target_only: &[MountedNodeId],
        deferred_invocations: usize,
        additional_trace: MandatoryTracePlan,
    ) -> Option<(Vec<MountedNodeId>, RoutedTransactionAdmissionPlan)> {
        self.prepare_routed_invocations(
            facts,
            include_ordinary_route,
            target_only,
            deferred_target_only,
            deferred_invocations,
            additional_trace,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn prepare_routed_invocations(
        &mut self,
        facts: &RoutedIngressFacts,
        include_ordinary_route: bool,
        target_only: &[MountedNodeId],
        deferred_target_only: &[MountedNodeId],
        deferred_invocations: usize,
        additional_trace: MandatoryTracePlan,
    ) -> Option<(Vec<MountedNodeId>, RoutedTransactionAdmissionPlan)> {
        let route = self.prepare_invocation_route(facts, include_ordinary_route)?;
        if !self.preflight_target_only_bridges(facts, target_only)
            || !self.preflight_target_only_bridges(facts, deferred_target_only)
        {
            return None;
        }
        let ordinary_invocations =
            self.ordinary_invocation_count(facts, include_ordinary_route, route.len())?;
        let route_invocations = ordinary_invocations
            .checked_add(target_only.len())
            .or_else(|| {
                self.handle_routed_admission_rejection(
                    TraceRoutedAdmissionRejection::CheckedArithmeticOverflow,
                    facts,
                );
                None
            })?;
        let admitted_invocations = route_invocations
            .checked_add(deferred_invocations)
            .or_else(|| {
                self.handle_routed_admission_rejection(
                    TraceRoutedAdmissionRejection::CheckedArithmeticOverflow,
                    facts,
                );
                None
            })?;
        let admission = self.admit_routed_invocations(
            facts,
            route_invocations,
            admitted_invocations,
            additional_trace,
        )?;
        Some((route, admission))
    }

    fn prepare_invocation_route(
        &mut self,
        facts: &RoutedIngressFacts,
        include_ordinary_route: bool,
    ) -> Option<Vec<MountedNodeId>> {
        let target_status = self.tree.target_status(&facts.target);
        if target_status != TargetStatus::Live {
            self.record_processing_target_rejection(facts, target_status);
            return None;
        }
        if !include_ordinary_route {
            return Some(Vec::new());
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
        Some(route)
    }

    fn preflight_target_only_bridges(
        &mut self,
        facts: &RoutedIngressFacts,
        target_only: &[MountedNodeId],
    ) -> bool {
        for target in target_only {
            if self.tree.target_status(target) != TargetStatus::Live {
                continue;
            }
            if self
                .tree
                .preflight_event_bridges(core::slice::from_ref(target))
                .is_err()
            {
                self.poison_routed_facts(
                    facts,
                    TraceRoutedIntegrityFailure::EventBridgeMismatch,
                    Some(target),
                );
                return false;
            }
        }
        true
    }

    fn ordinary_invocation_count(
        &mut self,
        facts: &RoutedIngressFacts,
        include_ordinary_route: bool,
        route_len: usize,
    ) -> Option<usize> {
        if !include_ordinary_route {
            return Some(0);
        }
        route_len
            .checked_mul(2)
            .and_then(|count| count.checked_sub(1))
            .or_else(|| {
                self.handle_routed_admission_rejection(
                    TraceRoutedAdmissionRejection::CheckedArithmeticOverflow,
                    facts,
                );
                None
            })
    }

    fn admit_routed_invocations(
        &mut self,
        facts: &RoutedIngressFacts,
        route_invocations: usize,
        admitted_invocations: usize,
        additional_trace: MandatoryTracePlan,
    ) -> Option<RoutedTransactionAdmissionPlan> {
        let admission = RoutedTransactionAdmissionPlan::checked(
            route_invocations,
            admitted_invocations,
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
        let Some(trace_plan) = admission.trace.checked_add(additional_trace) else {
            self.handle_routed_admission_rejection(
                TraceRoutedAdmissionRejection::CheckedArithmeticOverflow,
                facts,
            );
            return None;
        };
        if !self
            .trace
            .can_replace_reservation(facts.trace_reservation, trace_plan)
        {
            self.handle_routed_admission_rejection(
                TraceRoutedAdmissionRejection::TraceSequenceExhausted,
                facts,
            );
            return None;
        }
        self.trace.release_reservation(facts.trace_reservation);
        Some(admission)
    }
}
