use std::collections::HashMap;

use runenui_core::HostProtocol;

use super::{
    super::Runtime,
    transaction::{RoutedFailureLineage, RoutedIngressFacts},
};
use crate::{
    MountedNodeId, TraceRoutedAdmissionRejection, TraceRoutedIntegrityFailure,
    mounted::{RouteBuildError, TargetStatus},
    queue::{
        QueueCommitError, RootProducerCancellations, RootProducerCommit,
        RootProducerResponseReleases, RootProducerWorkStarts,
    },
    trace::MandatoryTracePlan,
    work::{MountedCallbackPreflightError, WorkFamily},
};

/// Per-route output limits stay independent even when retirement callbacks
/// share one atomic output ledger and one causal-root frontier.
#[derive(Clone, Copy)]
enum RoutedOutputBudget {
    Single(usize),
    ReconciledRetirement {
        composition_cancel: Option<usize>,
        focus_out: usize,
    },
}

struct RoutedInvocationAdmission<'a> {
    facts: &'a RoutedIngressFacts,
    route_invocations: usize,
    admitted_invocations: usize,
    output_budget: RoutedOutputBudget,
    additional_trace: MandatoryTracePlan,
    mandatory_default_outputs: usize,
    callback_output_budgets: HashMap<MountedNodeId, runenui_core::WidgetProducerBudget>,
    declared_work_starts: runenui_core::WorkStartBudget,
}

impl RoutedOutputBudget {
    const fn total(&self) -> Option<usize> {
        match self {
            Self::Single(outputs) => Some(*outputs),
            Self::ReconciledRetirement {
                composition_cancel: Some(composition),
                focus_out,
            } => composition.checked_add(*focus_out),
            Self::ReconciledRetirement {
                composition_cancel: None,
                focus_out,
            } => Some(*focus_out),
        }
    }

    const fn fits_each_route(&self, limit: usize) -> bool {
        match self {
            Self::Single(outputs) => *outputs <= limit,
            Self::ReconciledRetirement {
                composition_cancel,
                focus_out,
            } => {
                *focus_out <= limit
                    && match composition_cancel {
                        Some(outputs) => *outputs <= limit,
                        None => true,
                    }
            }
        }
    }
}

pub(super) struct RoutedTransactionAdmissionPlan {
    pub(super) route_invocations: usize,
    pub(super) max_outputs: usize,
    pub(super) default_output_operations: usize,
    pub(super) callback_output_budgets: HashMap<MountedNodeId, runenui_core::WidgetProducerBudget>,
    pub(super) declared_work_starts: runenui_core::WorkStartBudget,
    queued_default_outputs: usize,
    queue_slots: usize,
    pub(super) trace: MandatoryTracePlan,
}

impl RoutedTransactionAdmissionPlan {
    fn checked(
        route_invocations: usize,
        admitted_invocations: usize,
        max_outputs: usize,
        mandatory_default_outputs: usize,
        callback_output_budgets: HashMap<MountedNodeId, runenui_core::WidgetProducerBudget>,
        declared_work_starts: runenui_core::WorkStartBudget,
    ) -> Result<Self, TraceRoutedAdmissionRejection> {
        // IME and cursor state synchronization are ordinary runtime-owned
        // framework-service outputs. Reserve both before callbacks mutate any
        // routed state so their commit cannot overrun the existing FIFO plan.
        let queued_default_outputs = mandatory_default_outputs;
        let queued_output_envelopes = max_outputs
            .checked_add(mandatory_default_outputs)
            .ok_or(TraceRoutedAdmissionRejection::CheckedArithmeticOverflow)?;
        let queue_slots = queued_output_envelopes
            .checked_mul(2)
            .ok_or(TraceRoutedAdmissionRejection::CheckedArithmeticOverflow)?;
        // IME and cursor services are registered as work, not placed in the
        // FIFO. Reserve their work/trace capacity without reserving queue slots.
        let default_output_operations = mandatory_default_outputs
            .checked_add(2)
            .ok_or(TraceRoutedAdmissionRejection::CheckedArithmeticOverflow)?;
        let trace_output_operations = max_outputs
            .checked_add(default_output_operations)
            .ok_or(TraceRoutedAdmissionRejection::CheckedArithmeticOverflow)?;
        let trace = MandatoryTracePlan::routed_event(admitted_invocations, trace_output_operations)
            .ok_or(TraceRoutedAdmissionRejection::CheckedArithmeticOverflow)?;
        Ok(Self {
            route_invocations,
            max_outputs,
            default_output_operations,
            callback_output_budgets,
            declared_work_starts,
            queued_default_outputs,
            queue_slots,
            trace,
        })
    }

    fn preflight<State, Action, Protocol: HostProtocol>(
        &self,
        runtime: &Runtime<State, Action, Protocol>,
        root: Option<crate::queue::RootQuotaId>,
    ) -> Result<(), TraceRoutedAdmissionRejection> {
        if runtime.next_generation().is_none() {
            return Err(TraceRoutedAdmissionRejection::ReconciliationGenerationExhausted);
        }
        if let Some(root) = root {
            let _ = root;
        } else {
            runtime
                .queue
                .preflight_commit(self.queue_slots)
                .map_err(|error| match error {
                    QueueCommitError::Full => TraceRoutedAdmissionRejection::WaitingEnvelopes,
                    QueueCommitError::SequenceExhausted => {
                        TraceRoutedAdmissionRejection::WorkSequenceExhausted
                    }
                })?;
        }
        let preflight = if root.is_some() {
            runtime
                .work
                .preflight_rooted_callback_generations(self.declared_work_starts)
        } else {
            let possible_mounted_outputs = self
                .max_outputs
                .checked_add(self.default_output_operations)
                .ok_or(TraceRoutedAdmissionRejection::CheckedArithmeticOverflow)?;
            runtime
                .work
                .preflight_mounted_callback(possible_mounted_outputs)
        };
        preflight.map_err(|error| match error {
            MountedCallbackPreflightError::FamilyFull(WorkFamily::LocalTask) => {
                TraceRoutedAdmissionRejection::LocalTasks
            }
            MountedCallbackPreflightError::FamilyFull(WorkFamily::SendTask) => {
                TraceRoutedAdmissionRejection::SendTasks
            }
            MountedCallbackPreflightError::FamilyFull(WorkFamily::Timer) => {
                TraceRoutedAdmissionRejection::Timers
            }
            MountedCallbackPreflightError::FamilyFull(WorkFamily::FrameworkService) => {
                TraceRoutedAdmissionRejection::FrameworkServices
            }
            MountedCallbackPreflightError::FamilyFull(WorkFamily::HostRequest) => {
                TraceRoutedAdmissionRejection::HostRequests
            }
            MountedCallbackPreflightError::FamilyFull(WorkFamily::Subscription) => {
                unreachable!("mounted callback preflight excludes subscriptions")
            }
            MountedCallbackPreflightError::GenerationExhausted => {
                TraceRoutedAdmissionRejection::WorkGenerationExhausted
            }
        })?;
        Ok(())
    }
}

impl<State, Action, Protocol: HostProtocol> Runtime<State, Action, Protocol> {
    /// A reconciled retirement has one exact old route, with an optional
    /// composition-cancel pass before `FocusOut`. Both retain their independent
    /// per-callback output and work allowances.
    pub(super) fn prepare_reconciled_focus_route(
        &mut self,
        facts: &RoutedIngressFacts,
        composition_active: bool,
    ) -> Result<(Vec<MountedNodeId>, RoutedTransactionAdmissionPlan), RoutedFailureLineage> {
        let route = self.prepare_invocation_route(facts, true)?;
        let invocations = self.ordinary_invocation_count(facts, true, route.len())?;
        let (budgets, outputs, starts) =
            self.callback_output_budgets(facts, &route, true, &[], &[])?;
        let checked = if composition_active {
            invocations
                .checked_mul(2)
                .zip(add_work_start_budgets(starts, starts))
        } else {
            Some((invocations, starts))
        };
        let Some((invocations, starts)) = checked else {
            return Err(self.handle_routed_admission_rejection(
                TraceRoutedAdmissionRejection::CheckedArithmeticOverflow,
                facts,
            ));
        };
        let trace = if composition_active {
            MandatoryTracePlan::focus_commit()
                .checked_add(MandatoryTracePlan::composition_cleanup())
        } else {
            Some(MandatoryTracePlan::focus_commit())
        }
        .ok_or_else(|| {
            self.handle_routed_admission_rejection(
                TraceRoutedAdmissionRejection::CheckedArithmeticOverflow,
                facts,
            )
        })?;
        let admission = self.admit_routed_invocations(RoutedInvocationAdmission {
            facts,
            route_invocations: invocations,
            admitted_invocations: invocations,
            output_budget: RoutedOutputBudget::ReconciledRetirement {
                composition_cancel: composition_active.then_some(outputs),
                focus_out: outputs,
            },
            additional_trace: trace,
            mandatory_default_outputs: 0,
            callback_output_budgets: budgets,
            declared_work_starts: starts,
        })?;
        Ok((route, admission))
    }

    pub(super) fn prepare_routed_route_with_default_outputs(
        &mut self,
        facts: &RoutedIngressFacts,
        additional_trace: MandatoryTracePlan,
        mandatory_default_outputs: usize,
    ) -> Result<(Vec<MountedNodeId>, RoutedTransactionAdmissionPlan), RoutedFailureLineage> {
        self.try_prepare_routed_invocations(
            facts,
            true,
            &[],
            &[],
            0,
            additional_trace,
            mandatory_default_outputs,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn prepare_pointer_routed_route(
        &mut self,
        facts: &RoutedIngressFacts,
        include_ordinary_route: bool,
        target_only: &[MountedNodeId],
        deferred_target_only: &[MountedNodeId],
        deferred_invocations: usize,
        additional_trace: MandatoryTracePlan,
        may_focus: bool,
    ) -> Option<(Vec<MountedNodeId>, RoutedTransactionAdmissionPlan)> {
        let mut deferred_targets = deferred_target_only.to_vec();
        let (deferred_invocations, additional_trace) = if may_focus {
            let focus_targets = self.tree.publication_preorder_ids();
            for target in focus_targets.iter().cloned() {
                if !deferred_targets.contains(&target) {
                    deferred_targets.push(target);
                }
            }
            let Some(focus_invocations) = focus_notification_invocations(focus_targets.len())
            else {
                self.handle_routed_admission_rejection(
                    TraceRoutedAdmissionRejection::CheckedArithmeticOverflow,
                    facts,
                );
                return None;
            };
            let Some(invocations) = deferred_invocations.checked_add(focus_invocations) else {
                self.handle_routed_admission_rejection(
                    TraceRoutedAdmissionRejection::CheckedArithmeticOverflow,
                    facts,
                );
                return None;
            };
            let Some(trace) = additional_trace.checked_add(MandatoryTracePlan::focus_commit())
            else {
                self.handle_routed_admission_rejection(
                    TraceRoutedAdmissionRejection::CheckedArithmeticOverflow,
                    facts,
                );
                return None;
            };
            (invocations, trace)
        } else {
            (deferred_invocations, additional_trace)
        };
        self.try_prepare_routed_invocations(
            facts,
            include_ordinary_route,
            target_only,
            &deferred_targets,
            deferred_invocations,
            additional_trace,
            facts.default_queue_start_outputs,
        )
        .ok()
    }

    pub(super) fn prepare_focus_routed_route(
        &mut self,
        facts: &RoutedIngressFacts,
        additional_trace: MandatoryTracePlan,
        mandatory_default_outputs: usize,
    ) -> Result<(Vec<MountedNodeId>, RoutedTransactionAdmissionPlan), RoutedFailureLineage> {
        let targets = self.tree.publication_preorder_ids();
        let Some(deferred_invocations) = focus_notification_invocations(targets.len()) else {
            return Err(self.handle_routed_admission_rejection(
                TraceRoutedAdmissionRejection::CheckedArithmeticOverflow,
                facts,
            ));
        };
        let trace = MandatoryTracePlan::focus_commit()
            .checked_add(additional_trace)
            .ok_or_else(|| {
                self.handle_routed_admission_rejection(
                    TraceRoutedAdmissionRejection::CheckedArithmeticOverflow,
                    facts,
                )
            })?;
        self.try_prepare_routed_invocations(
            facts,
            true,
            &[],
            &targets,
            deferred_invocations,
            trace,
            mandatory_default_outputs,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn try_prepare_routed_invocations(
        &mut self,
        facts: &RoutedIngressFacts,
        include_ordinary_route: bool,
        target_only: &[MountedNodeId],
        deferred_target_only: &[MountedNodeId],
        deferred_invocations: usize,
        additional_trace: MandatoryTracePlan,
        mandatory_default_outputs: usize,
    ) -> Result<(Vec<MountedNodeId>, RoutedTransactionAdmissionPlan), RoutedFailureLineage> {
        let route = self.prepare_invocation_route(facts, include_ordinary_route)?;
        self.preflight_target_only_bridges(facts, target_only)?;
        self.preflight_target_only_bridges(facts, deferred_target_only)?;
        let ordinary_invocations =
            self.ordinary_invocation_count(facts, include_ordinary_route, route.len())?;
        let route_invocations = ordinary_invocations
            .checked_add(target_only.len())
            .ok_or_else(|| {
                self.handle_routed_admission_rejection(
                    TraceRoutedAdmissionRejection::CheckedArithmeticOverflow,
                    facts,
                )
            })?;
        let admitted_invocations = route_invocations
            .checked_add(deferred_invocations)
            .ok_or_else(|| {
                self.handle_routed_admission_rejection(
                    TraceRoutedAdmissionRejection::CheckedArithmeticOverflow,
                    facts,
                )
            })?;
        let (callback_output_budgets, callback_outputs, mut declared_work_starts) = self
            .callback_output_budgets(
                facts,
                &route,
                include_ordinary_route,
                target_only,
                deferred_target_only,
            )?;
        declared_work_starts =
            add_work_start_budgets(declared_work_starts, facts.default_work_starts).ok_or_else(
                || {
                    self.handle_routed_admission_rejection(
                        TraceRoutedAdmissionRejection::CheckedArithmeticOverflow,
                        facts,
                    )
                },
            )?;
        let max_outputs = callback_outputs
            .checked_add(facts.default_output_budget)
            .ok_or_else(|| {
                self.handle_routed_admission_rejection(
                    TraceRoutedAdmissionRejection::CheckedArithmeticOverflow,
                    facts,
                )
            })?;
        let admission = self.admit_routed_invocations(RoutedInvocationAdmission {
            facts,
            route_invocations,
            admitted_invocations,
            output_budget: RoutedOutputBudget::Single(max_outputs),
            additional_trace,
            mandatory_default_outputs,
            callback_output_budgets,
            declared_work_starts,
        })?;
        Ok((route, admission))
    }

    fn prepare_invocation_route(
        &mut self,
        facts: &RoutedIngressFacts,
        include_ordinary_route: bool,
    ) -> Result<Vec<MountedNodeId>, RoutedFailureLineage> {
        let target_status = self.tree.target_status(&facts.target);
        if target_status != TargetStatus::Live {
            return Err(self.record_processing_target_rejection(facts, target_status));
        }
        if !include_ordinary_route {
            return Ok(Vec::new());
        }
        let route = match self.tree.event_route(&facts.target) {
            Ok(route) => route,
            Err(RouteBuildError::Target(status)) => {
                return Err(self.record_processing_target_rejection(facts, status));
            }
            Err(RouteBuildError::BrokenTopology) => {
                return Err(self.poison_routed_facts(
                    facts,
                    TraceRoutedIntegrityFailure::BrokenTopology,
                    None,
                ));
            }
            Err(RouteBuildError::BridgeMismatch) => {
                return Err(self.poison_routed_facts(
                    facts,
                    TraceRoutedIntegrityFailure::EventBridgeMismatch,
                    None,
                ));
            }
        };
        if self.tree.preflight_event_bridges(&route).is_err() {
            return Err(self.poison_routed_facts(
                facts,
                TraceRoutedIntegrityFailure::EventBridgeMismatch,
                None,
            ));
        }
        Ok(route)
    }

    fn preflight_target_only_bridges(
        &mut self,
        facts: &RoutedIngressFacts,
        target_only: &[MountedNodeId],
    ) -> Result<(), RoutedFailureLineage> {
        for target in target_only {
            if self.tree.target_status(target) != TargetStatus::Live {
                continue;
            }
            if self
                .tree
                .preflight_event_bridges(core::slice::from_ref(target))
                .is_err()
            {
                return Err(self.poison_routed_facts(
                    facts,
                    TraceRoutedIntegrityFailure::EventBridgeMismatch,
                    Some(target),
                ));
            }
        }
        Ok(())
    }

    fn ordinary_invocation_count(
        &mut self,
        facts: &RoutedIngressFacts,
        include_ordinary_route: bool,
        route_len: usize,
    ) -> Result<usize, RoutedFailureLineage> {
        if !include_ordinary_route {
            return Ok(0);
        }
        route_len
            .checked_mul(2)
            .and_then(|count| count.checked_sub(1))
            .ok_or_else(|| {
                self.handle_routed_admission_rejection(
                    TraceRoutedAdmissionRejection::CheckedArithmeticOverflow,
                    facts,
                )
            })
    }

    fn admit_routed_invocations(
        &mut self,
        request: RoutedInvocationAdmission<'_>,
    ) -> Result<RoutedTransactionAdmissionPlan, RoutedFailureLineage> {
        let RoutedInvocationAdmission {
            facts,
            route_invocations,
            admitted_invocations,
            output_budget,
            additional_trace,
            mandatory_default_outputs,
            callback_output_budgets,
            declared_work_starts,
        } = request;
        // At most one accepted focus default plus one interaction-invalidation
        // retirement can commit in a routed transaction. Reserve both before
        // any widget callback mutates state or queues a derived command.
        if !self.focus.can_reserve_steps(2) {
            return Err(self.handle_routed_admission_rejection(
                TraceRoutedAdmissionRejection::CheckedArithmeticOverflow,
                facts,
            ));
        }
        let Some(max_outputs) = output_budget.total() else {
            return Err(self.handle_routed_admission_rejection(
                TraceRoutedAdmissionRejection::CheckedArithmeticOverflow,
                facts,
            ));
        };
        if !output_budget.fits_each_route(self.limits.transaction_outputs()) {
            return Err(self.handle_routed_admission_rejection(
                TraceRoutedAdmissionRejection::TransactionOutputs,
                facts,
            ));
        }
        self.preflight_routed_profile(
            facts,
            max_outputs,
            mandatory_default_outputs,
            declared_work_starts,
        )?;
        let admission = RoutedTransactionAdmissionPlan::checked(
            route_invocations,
            admitted_invocations,
            max_outputs,
            mandatory_default_outputs,
            callback_output_budgets,
            declared_work_starts,
        )
        .map_err(|capacity| self.handle_routed_admission_rejection(capacity, facts))?;
        self.preflight_routed_root_commit(facts, &admission)?;
        admission
            .preflight(self, facts.root_quota)
            .map_err(|capacity| self.handle_routed_admission_rejection(capacity, facts))?;
        let trace_plan = admission
            .trace
            .checked_add(additional_trace)
            .ok_or_else(|| {
                self.handle_routed_admission_rejection(
                    TraceRoutedAdmissionRejection::CheckedArithmeticOverflow,
                    facts,
                )
            })?;
        if !self
            .trace
            .can_replace_reservation(facts.trace_reservation, trace_plan)
        {
            return Err(self.handle_routed_admission_rejection(
                TraceRoutedAdmissionRejection::TraceSequenceExhausted,
                facts,
            ));
        }
        self.trace.release_reservation(facts.trace_reservation);
        Ok(admission)
    }

    fn preflight_routed_profile(
        &mut self,
        facts: &RoutedIngressFacts,
        max_outputs: usize,
        mandatory_default_outputs: usize,
        declared_work_starts: runenui_core::WorkStartBudget,
    ) -> Result<(), RoutedFailureLineage> {
        let Some(root) = facts.root_quota else {
            return Ok(());
        };
        let Some(profile) = self.queue.root_profile(root) else {
            return Err(self.poison_routed_facts(
                facts,
                TraceRoutedIntegrityFailure::OutputAllowanceExceeded,
                None,
            ));
        };
        let output_count = max_outputs.checked_add(mandatory_default_outputs);
        let start_budget_exceeded = [
            runenui_core::WorkFamily::LocalTask,
            runenui_core::WorkFamily::SendTask,
            runenui_core::WorkFamily::Timer,
            runenui_core::WorkFamily::HostRequest,
            runenui_core::WorkFamily::FrameworkService,
        ]
        .into_iter()
        .any(|family| {
            declared_work_starts.for_family(family) > profile.work_starts().for_family(family)
        });
        if output_count.is_none_or(|required| required > profile.outputs()) || start_budget_exceeded
        {
            return Err(self.poison_routed_facts(
                facts,
                TraceRoutedIntegrityFailure::OutputAllowanceExceeded,
                None,
            ));
        }
        Ok(())
    }

    fn preflight_routed_root_commit(
        &mut self,
        facts: &RoutedIngressFacts,
        admission: &RoutedTransactionAdmissionPlan,
    ) -> Result<(), RoutedFailureLineage> {
        let Some(root) = facts.root_quota else {
            return Ok(());
        };
        let default_start_count = facts
            .default_queue_start_outputs
            .min(admission.queued_default_outputs);
        let queued = admission
            .queued_default_outputs
            .checked_sub(default_start_count)
            .and_then(|defaults| admission.max_outputs.checked_add(defaults))
            .and_then(|queued| {
                admission
                    .declared_work_starts
                    .total()
                    // Framework services register directly at commit. Their
                    // response/cancellation claims are charged below by the
                    // root ledger; there is no queued EffectStart envelope.
                    .and_then(|starts| {
                        starts.checked_sub(
                            admission
                                .declared_work_starts
                                .for_family(runenui_core::WorkFamily::FrameworkService),
                        )
                    })
                    .and_then(|starts| queued.checked_add(starts))
            })
            .ok_or_else(|| {
                self.handle_routed_admission_rejection(
                    TraceRoutedAdmissionRejection::CheckedArithmeticOverflow,
                    facts,
                )
            })?;
        let starts = admission.declared_work_starts;
        let root_commit = RootProducerCommit {
            root,
            queued_envelopes: queued,
            starts: RootProducerWorkStarts {
                host_requests: starts.for_family(runenui_core::WorkFamily::HostRequest),
                local_tasks: starts.for_family(runenui_core::WorkFamily::LocalTask),
                send_tasks: starts.for_family(runenui_core::WorkFamily::SendTask),
                timers: starts.for_family(runenui_core::WorkFamily::Timer),
                repeating_timers: starts.for_family(runenui_core::WorkFamily::Timer),
                framework_services: starts.for_family(runenui_core::WorkFamily::FrameworkService),
            },
            cancellations: RootProducerCancellations {
                host_requests: 0,
                framework_services: 0,
                local_tasks: 0,
                send_tasks: 0,
                timers: 0,
            },
            response_releases: RootProducerResponseReleases {
                host_requests: 0,
                framework_services: 0,
            },
        };
        if let Err(error) = self.queue.preflight_root_producer_commit(root_commit) {
            return Err(match error {
                crate::queue::RootProducerCommitError::Reservation(
                    crate::queue::QueueReservationError::SequenceExhausted,
                ) => self.handle_routed_admission_rejection(
                    TraceRoutedAdmissionRejection::WorkSequenceExhausted,
                    facts,
                ),
                crate::queue::RootProducerCommitError::FrontierInsufficient { .. }
                | crate::queue::RootProducerCommitError::Reservation(_) => self
                    .poison_routed_facts(
                        facts,
                        TraceRoutedIntegrityFailure::OutputAllowanceExceeded,
                        None,
                    ),
            });
        }
        Ok(())
    }

    fn callback_output_budgets(
        &mut self,
        facts: &RoutedIngressFacts,
        route: &[MountedNodeId],
        include_ordinary_route: bool,
        target_only: &[MountedNodeId],
        deferred_target_only: &[MountedNodeId],
    ) -> Result<
        (
            HashMap<MountedNodeId, runenui_core::WidgetProducerBudget>,
            usize,
            runenui_core::WorkStartBudget,
        ),
        RoutedFailureLineage,
    > {
        let mut budgets = HashMap::new();
        let mut total = 0usize;
        let mut starts = [0usize; 5];
        if include_ordinary_route && let Some((target, ancestors)) = route.split_last() {
            for owner in ancestors {
                self.add_callback_budget(facts, &mut budgets, &mut total, &mut starts, owner, 2)?;
            }
            self.add_callback_budget(facts, &mut budgets, &mut total, &mut starts, target, 1)?;
        }
        for owner in target_only {
            self.add_callback_budget(facts, &mut budgets, &mut total, &mut starts, owner, 1)?;
        }
        for owner in deferred_target_only {
            self.add_callback_budget(facts, &mut budgets, &mut total, &mut starts, owner, 4)?;
        }
        Ok((
            budgets,
            total,
            runenui_core::WorkStartBudget::new(
                starts[0], starts[1], starts[2], starts[3], starts[4],
            ),
        ))
    }

    fn add_callback_budget(
        &mut self,
        facts: &RoutedIngressFacts,
        budgets: &mut HashMap<MountedNodeId, runenui_core::WidgetProducerBudget>,
        total: &mut usize,
        starts: &mut [usize; 5],
        owner: &MountedNodeId,
        invocation_count: usize,
    ) -> Result<(), RoutedFailureLineage> {
        let Ok(budget) = self.tree.event_budget(owner) else {
            return Err(self.poison_routed_facts(
                facts,
                TraceRoutedIntegrityFailure::CallbackBridgeFailure,
                Some(owner),
            ));
        };
        let Some(weighted) = budget.outputs().checked_mul(invocation_count) else {
            return Err(self.handle_routed_admission_rejection(
                TraceRoutedAdmissionRejection::CheckedArithmeticOverflow,
                facts,
            ));
        };
        *total = total.checked_add(weighted).ok_or_else(|| {
            self.handle_routed_admission_rejection(
                TraceRoutedAdmissionRejection::CheckedArithmeticOverflow,
                facts,
            )
        })?;
        for (index, family) in [
            runenui_core::WorkFamily::LocalTask,
            runenui_core::WorkFamily::SendTask,
            runenui_core::WorkFamily::Timer,
            runenui_core::WorkFamily::HostRequest,
            runenui_core::WorkFamily::FrameworkService,
        ]
        .into_iter()
        .enumerate()
        {
            let Some(weighted_starts) = budget
                .work_starts()
                .for_family(family)
                .checked_mul(invocation_count)
            else {
                return Err(self.handle_routed_admission_rejection(
                    TraceRoutedAdmissionRejection::CheckedArithmeticOverflow,
                    facts,
                ));
            };
            let Some(next) = starts[index].checked_add(weighted_starts) else {
                return Err(self.handle_routed_admission_rejection(
                    TraceRoutedAdmissionRejection::CheckedArithmeticOverflow,
                    facts,
                ));
            };
            starts[index] = next;
        }
        budgets.entry(owner.clone()).or_insert(budget);
        Ok(())
    }
}

fn focus_notification_invocations(live_nodes: usize) -> Option<usize> {
    live_nodes
        .checked_mul(4)
        .and_then(|count| count.checked_sub(2))
}

fn add_work_start_budgets(
    left: runenui_core::WorkStartBudget,
    right: runenui_core::WorkStartBudget,
) -> Option<runenui_core::WorkStartBudget> {
    use runenui_core::WorkFamily;
    let sum = |family| {
        left.for_family(family)
            .checked_add(right.for_family(family))
    };
    Some(runenui_core::WorkStartBudget::new(
        sum(WorkFamily::LocalTask)?,
        sum(WorkFamily::SendTask)?,
        sum(WorkFamily::Timer)?,
        sum(WorkFamily::HostRequest)?,
        sum(WorkFamily::FrameworkService)?,
    ))
}

#[cfg(test)]
mod tests {
    use super::focus_notification_invocations;

    #[test]
    fn focus_notification_admission_count_is_checked() {
        assert_eq!(focus_notification_invocations(1), Some(2));
        assert_eq!(focus_notification_invocations(3), Some(10));
        assert_eq!(focus_notification_invocations(usize::MAX), None);
    }
}
