use runenui_core::CommandOrigin;

use super::{
    ApplicationActionEnvelope, ApplicationTraceTransaction, ApplicationTransactionInput, HashMap,
    HashSet, HostProtocol, IntoEffects, MandatoryTracePlan, MountedNodeId, MutationPhase,
    OwnedTransactionLedger, ProcessApplicationActionOutcome, ReconciliationGeneration,
    ReconciliationReport, Runtime, RuntimeStatus, RuntimeTerminalReason, SubscriptionDiff,
    SubscriptionSet, TargetStatus, TraceRecordKind, TransactionLedger, UiApp, View, WorkOwner,
    mounted_effect_into_effect, public_trace_work_identity, revoke_generation_authority,
};

impl<State, Action, Protocol: HostProtocol> Runtime<State, Action, Protocol> {
    pub(in crate::runtime) const fn next_generation(&self) -> Option<u64> {
        self.generation.checked_add(1)
    }

    pub(in crate::runtime) fn validate_focus(&mut self, id: &MountedNodeId) -> bool {
        crate::focus::is_focus_eligible(&mut self.tree, id)
    }
}

#[allow(clippy::too_many_lines)]
pub(crate) fn process_application_action<App: UiApp>(
    runtime: &mut Runtime<App::State, App::Action, App::HostProtocol>,
    envelope: ApplicationActionEnvelope<App::Action>,
) -> ProcessApplicationActionOutcome {
    let ApplicationActionEnvelope {
        sequence,
        action,
        causal_parent,
        target,
        origin: _origin,
    } = envelope;
    let before = ReconciliationGeneration(runtime.generation);
    let Some(next) = runtime.next_generation() else {
        let reason = RuntimeTerminalReason::ReconciliationGenerationExhausted;
        let cancelled = runtime.enter_terminal(reason, 1);
        return ProcessApplicationActionOutcome::Terminal { reason, cancelled };
    };
    let mut mutation_phase = MutationPhase::PreMutation;
    if !runtime
        .trace
        .can_admit(MandatoryTracePlan::application_action_base(
            runtime.focus.focused_node().is_some(),
        ))
    {
        let reason = mutation_phase.terminal_reason(RuntimeTerminalReason::TraceSequenceExhausted);
        let cancelled = runtime.enter_terminal(reason, 1);
        return ProcessApplicationActionOutcome::Terminal { reason, cancelled };
    }
    let trace_transaction = ApplicationTraceTransaction::new(runtime.now());
    let transaction_parent = runtime.trace.record_draft(
        trace_transaction
            .fact(TraceRecordKind::ApplicationActionTransactionStarted)
            .with_work_sequence(Some(sequence))
            .with_causal_parent(causal_parent)
            .with_reconciliation(Some(before), None)
            .with_target(target.clone()),
    );
    let app_state = runtime
        .state
        .as_mut()
        .unwrap_or_else(|| unreachable!("live runtime retains application state"));
    let effects = App::update(app_state, action).into_effects();
    mutation_phase = MutationPhase::Mutated;
    let ledger = match TransactionLedger::collect(effects, runtime.limits.transaction_outputs()) {
        Ok(ledger) => ledger,
        Err(_error) => {
            let reason = RuntimeTerminalReason::Poisoned;
            let cancelled = runtime.enter_terminal(reason, 1);
            return ProcessApplicationActionOutcome::Terminal { reason, cancelled };
        }
    };
    let update_output_count = ledger.len();
    runtime.trace.record_draft(
        trace_transaction
            .fact(TraceRecordKind::ApplicationStateUpdated)
            .with_work_sequence(Some(sequence))
            .with_causal_parent(causal_parent)
            .with_reconciliation(Some(before), None)
            .with_target(target.clone()),
    );
    let transient = App::root(app_state).into_element();
    let previous_focus = runtime.focus.focused_node().cloned();
    let previous_focus_route_len = runtime.focus.route_len();
    let previous_focus_trace = previous_focus
        .as_ref()
        .map(|focused| runtime.tree.trace_target(focused));
    let mut lifecycle_invalidated = Vec::new();
    let mut lifecycle_invalidated_identities = Vec::new();
    let mounted_public_slot_limit = runtime.mounted_public_slot_limit;
    let Ok(reconciliation_plan) = runtime
        .tree
        .plan_reconciliation(transient, mounted_public_slot_limit)
    else {
        let reason =
            mutation_phase.terminal_reason(RuntimeTerminalReason::MountedIdentityExhausted);
        let cancelled = runtime.enter_terminal(reason, 0);
        return ProcessApplicationActionOutcome::Terminal { reason, cancelled };
    };
    let input_cleanup_cause = super::super::focus::InputLifetimeCleanupCause::new(
        Some(sequence),
        transaction_parent,
        trace_transaction.logical_time(),
        CommandOrigin::programmatic(),
    );
    runtime.cleanup_planned_input_lifetimes(
        reconciliation_plan.invalidated_lifetimes(),
        input_cleanup_cause,
    );
    if let RuntimeStatus::Terminal(reason) = runtime.status {
        return ProcessApplicationActionOutcome::Terminal {
            reason,
            cancelled: 0,
        };
    }
    let reconcile_stats = {
        let (
            tree,
            work,
            completion_ingress,
            local_tasks,
            timers,
            send_task_mappers,
            subscriptions,
            host_requests,
        ) = (
            &mut runtime.tree,
            &mut runtime.work,
            &runtime.completion_ingress,
            &mut runtime.local_tasks,
            &mut runtime.timers,
            &mut runtime.send_task_mappers,
            &mut runtime.subscriptions,
            &mut runtime.host_requests,
        );
        tree.apply_reconciliation(reconciliation_plan, &mut |owner| {
            let owner = WorkOwner::Mounted(owner.clone());
            let generations = work.generations_for_owner(&owner);
            for generation in &generations {
                if let Some(identity) = work.trace_identity(*generation) {
                    lifecycle_invalidated_identities.push(public_trace_work_identity(identity));
                }
                revoke_generation_authority(
                    *generation,
                    work,
                    completion_ingress,
                    local_tasks,
                    timers,
                    send_task_mappers,
                    subscriptions,
                    host_requests,
                );
            }
            lifecycle_invalidated.extend(generations);
        })
    };
    let Ok(reconcile_stats) = reconcile_stats else {
        let reason = mutation_phase.terminal_reason(RuntimeTerminalReason::Poisoned);
        let cancelled = runtime.enter_terminal(reason, 0);
        return ProcessApplicationActionOutcome::Terminal { reason, cancelled };
    };
    let mounted_subscription_owners = reconcile_stats.mounted_owners.clone();
    let unmounted_work_owners = reconcile_stats.unmounted_owners.clone();
    let invalidated_subscription_owners = reconcile_stats.subscription_invalidated.clone();
    let mounted_outputs = reconcile_stats.mounted_outputs;
    runtime.generation = next;
    let after = ReconciliationGeneration(next);
    runtime.cleanup_lost_input_capabilities(input_cleanup_cause);
    if let RuntimeStatus::Terminal(reason) = runtime.status {
        return ProcessApplicationActionOutcome::Terminal {
            reason,
            cancelled: 0,
        };
    }
    let retained_focus = previous_focus
        .as_ref()
        .is_some_and(|id| runtime.validate_focus(id));
    if !retained_focus && previous_focus.is_some() {
        let reason = if runtime.tree.target_status(
            previous_focus
                .as_ref()
                .unwrap_or_else(|| unreachable!("cleared focus has a previous target")),
        ) == TargetStatus::Live
        {
            runenui_core::FocusReason::Disablement
        } else {
            runenui_core::FocusReason::Removal
        };
        runtime.commit_reconciled_focus_cleanup(super::super::focus::ReconciledFocusCleanup {
            old_target: previous_focus
                .unwrap_or_else(|| unreachable!("invalid focus has a previous target")),
            old_route_len: previous_focus_route_len,
            reason,
            sequence,
            causal_parent,
            before,
            after,
            trace_target: previous_focus_trace,
        });
    } else if retained_focus {
        runtime.trace.record_draft(
            trace_transaction
                .fact(TraceRecordKind::FocusRetained)
                .with_work_sequence(Some(sequence))
                .with_causal_parent(causal_parent)
                .with_reconciliation(Some(before), Some(after))
                .with_target(target.clone()),
        );
    }
    runtime.tree.finish_focus_validation();
    runtime.prune_focus_memory();
    runtime.report = ReconciliationReport {
        generation: after,
        live_node_count: runtime.tree.live_count(),
        mounted_count: reconcile_stats.mounted,
        updated_count: reconcile_stats.updated,
        unmounted_count: reconcile_stats.unmounted,
        moved_count: reconcile_stats.moved,
        retained_focus,
        diagnostics: reconcile_stats.diagnostics,
    };
    let Some(lifecycle_trace_plan) =
        MandatoryTracePlan::lifecycle_invalidations(lifecycle_invalidated_identities.len())
    else {
        let reason = mutation_phase.terminal_reason(RuntimeTerminalReason::TraceSequenceExhausted);
        let cancelled = runtime.enter_terminal(reason, 0);
        return ProcessApplicationActionOutcome::Terminal { reason, cancelled };
    };
    if !runtime.trace.can_admit(lifecycle_trace_plan) {
        let reason = mutation_phase.terminal_reason(RuntimeTerminalReason::TraceSequenceExhausted);
        let cancelled = runtime.enter_terminal(reason, 0);
        return ProcessApplicationActionOutcome::Terminal { reason, cancelled };
    }
    let mut lifecycle_cancellation_lineage = HashMap::new();
    for identity in lifecycle_invalidated_identities {
        let bound = runtime.record_work_fact_with_parent_at(
            TraceRecordKind::WorkCancellationBound,
            transaction_parent,
            identity.clone(),
            trace_transaction.logical_time(),
        );
        let invalidated = runtime.record_work_fact_with_parent_at(
            TraceRecordKind::WorkLogicallyInvalidated,
            bound,
            identity.clone(),
            trace_transaction.logical_time(),
        );
        lifecycle_cancellation_lineage.insert(identity.generation(), (identity, invalidated));
    }
    let tree_reconciled = runtime.trace.record_draft(
        trace_transaction
            .fact(TraceRecordKind::TreeReconciled)
            .with_work_sequence(Some(sequence))
            .with_causal_parent(causal_parent)
            .with_reconciliation(Some(before), Some(after))
            .with_target(target),
    );
    if runtime
        .reconcile_pointer_lifetimes(sequence, tree_reconciled, &unmounted_work_owners)
        .is_err()
    {
        let reason = match runtime.status {
            RuntimeStatus::Terminal(reason) => reason,
            RuntimeStatus::Running | RuntimeStatus::Closed => {
                mutation_phase.terminal_reason(RuntimeTerminalReason::Poisoned)
            }
        };
        let cancelled = runtime.enter_terminal(reason, 0);
        return ProcessApplicationActionOutcome::Terminal { reason, cancelled };
    }
    for owner in unmounted_work_owners {
        runtime
            .mounted_subscription_reconcile_pending
            .retain(|pending| pending != &owner);
    }

    let mut mounted_subscription_dirty = Vec::new();
    let mut dirty_seen = HashSet::new();
    for owner in mounted_subscription_owners
        .into_iter()
        .chain(invalidated_subscription_owners)
    {
        if runtime.tree.target_status(&owner) == TargetStatus::Live
            && !runtime
                .mounted_subscription_reconcile_pending
                .contains(&owner)
            && dirty_seen.insert(owner.clone())
        {
            mounted_subscription_dirty.push(owner);
        }
    }

    let mut total_outputs = update_output_count;
    let mut mounted_batches = Vec::with_capacity(mounted_outputs.len());
    for (owner, outputs) in mounted_outputs {
        total_outputs = match total_outputs.checked_add(outputs.len()) {
            Some(total) if total <= runtime.limits.transaction_outputs() => total,
            _ => {
                let reason = RuntimeTerminalReason::Poisoned;
                let cancelled = runtime.enter_terminal(reason, 0);
                return ProcessApplicationActionOutcome::Terminal { reason, cancelled };
            }
        };
        let effects = outputs
            .into_iter()
            .map(mounted_effect_into_effect)
            .collect();
        let mounted_ledger =
            TransactionLedger::from_outputs(effects, runtime.limits.transaction_outputs())
                .unwrap_or_else(|_| unreachable!("complete transaction allowance was checked"));
        mounted_batches.push(OwnedTransactionLedger {
            owner: WorkOwner::Mounted(owner),
            ledger: mounted_ledger,
        });
    }

    let mut subscriptions = SubscriptionSet::new();
    App::subscriptions(runtime.state(), &mut subscriptions);
    let SubscriptionDiff {
        invalidated,
        starts,
        duplicate_keys,
    } = runtime.derive_subscription_diff(
        &WorkOwner::Application,
        subscriptions.__runtime_into_declarations(),
    );
    let subscription_cancelled = invalidated.len();
    let input = ApplicationTransactionInput {
        lifecycle_invalidated,
        mounted_subscription_dirty,
        application: ledger,
        application_subscription_invalidated: invalidated,
        application_subscription_starts: starts,
        mounted: mounted_batches,
    };
    if runtime
        .plan_and_commit_application_transaction(
            input,
            &duplicate_keys,
            subscription_cancelled,
            transaction_parent,
            lifecycle_cancellation_lineage,
            trace_transaction,
        )
        .is_err()
    {
        let reason = RuntimeTerminalReason::Poisoned;
        let cancelled = runtime.enter_terminal(reason, 0);
        return ProcessApplicationActionOutcome::Terminal { reason, cancelled };
    }
    runtime.trace.record_draft(
        trace_transaction
            .fact(TraceRecordKind::UpdateEffectsCommitted {
                count: update_output_count,
            })
            .with_work_sequence(Some(sequence))
            .with_causal_parent(causal_parent),
    );
    runtime.request_redraw();
    if let RuntimeStatus::Terminal(reason) = runtime.status {
        return ProcessApplicationActionOutcome::Terminal {
            reason,
            cancelled: 0,
        };
    }
    ProcessApplicationActionOutcome::Completed
}
