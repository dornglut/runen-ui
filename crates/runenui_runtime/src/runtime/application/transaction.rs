use crate::transaction::TransactionLedger;
use runenui_core::{__runtime::Effect, MonotonicInstant};

use super::{
    ApplicationTraceTransaction, ApplicationTransactionInput, CommitError, HashMap, HashSet,
    HostProtocol, LiveSubscription, MandatoryTracePlan, PlannedApplicationTransaction,
    PlannedOutput, PlannedStartPayload, PlannedWorkSemanticEvent, Runtime, TraceActionCategory,
    TraceRecordKind, TraceSequence, TraceWorkIdentity, WorkOwner,
    required_application_transaction_trace_records_from_parts, trace_work_family, trace_work_owner,
};

/// Neutral trace ownership shared by application and routed work-plan commits.
#[derive(Debug)]
pub(in crate::runtime) struct PlannedWorkTrace {
    transaction_parent: Option<TraceSequence>,
    logical_time: MonotonicInstant,
    pre_recorded_lineage: HashMap<u64, (TraceWorkIdentity, Option<TraceSequence>)>,
}

pub(in crate::runtime::application) struct ApplicationTransactionCommitContext {
    subscription_duplicates: HashSet<runenui_core::WorkKey>,
    subscription_cancelled: usize,
    transaction_parent: Option<TraceSequence>,
    cancellation_lineage: HashMap<u64, (TraceWorkIdentity, Option<TraceSequence>)>,
    trace_transaction: ApplicationTraceTransaction,
    root_quota: Option<crate::queue::RootQuotaId>,
}

impl ApplicationTransactionCommitContext {
    pub(in crate::runtime::application) const fn new(
        subscription_duplicates: HashSet<runenui_core::WorkKey>,
        subscription_cancelled: usize,
        transaction_parent: Option<TraceSequence>,
        cancellation_lineage: HashMap<u64, (TraceWorkIdentity, Option<TraceSequence>)>,
        trace_transaction: ApplicationTraceTransaction,
        root_quota: Option<crate::queue::RootQuotaId>,
    ) -> Self {
        Self {
            subscription_duplicates,
            subscription_cancelled,
            transaction_parent,
            cancellation_lineage,
            trace_transaction,
            root_quota,
        }
    }
}

impl PlannedWorkTrace {
    pub(in crate::runtime) fn new(
        transaction_parent: Option<TraceSequence>,
        logical_time: MonotonicInstant,
    ) -> Self {
        Self {
            transaction_parent,
            logical_time,
            pre_recorded_lineage: HashMap::new(),
        }
    }

    #[must_use]
    pub(in crate::runtime) fn with_pre_recorded_lineage(
        mut self,
        pre_recorded_lineage: HashMap<u64, (TraceWorkIdentity, Option<TraceSequence>)>,
    ) -> Self {
        self.pre_recorded_lineage = pre_recorded_lineage;
        self
    }
}

impl<State, Action, Protocol: HostProtocol> Runtime<State, Action, Protocol> {
    pub(in crate::runtime) fn commit_framework_service_effects(
        &mut self,
        effects: Vec<(
            crate::MountedNodeId,
            runenui_core::__runtime::FrameworkServiceEffect,
        )>,
        transaction_parent: Option<TraceSequence>,
        instant: MonotonicInstant,
        root_quota: Option<crate::queue::RootQuotaId>,
    ) -> Result<(), CommitError> {
        if effects.is_empty() {
            return Ok(());
        }
        let mut mounted = Vec::with_capacity(effects.len());
        for (owner, effect) in effects {
            let ledger = TransactionLedger::from_outputs(
                vec![Effect::FrameworkService(effect)],
                self.limits.transaction_outputs(),
            )
            .map_err(|_| CommitError::Registry)?;
            mounted.push(super::OwnedTransactionLedger {
                owner: WorkOwner::Mounted(owner),
                ledger,
            });
        }
        let input = ApplicationTransactionInput {
            lifecycle_invalidated: Vec::new(),
            mounted_subscription_dirty: Vec::new(),
            application: TransactionLedger::from_outputs(
                Vec::new(),
                self.limits.transaction_outputs(),
            )
            .map_err(|_| CommitError::Registry)?,
            application_subscription_invalidated: Vec::new(),
            application_subscription_starts: Vec::new(),
            mounted,
        };
        self.plan_and_commit_application_transaction(
            input,
            ApplicationTransactionCommitContext::new(
                HashSet::new(),
                0,
                transaction_parent,
                HashMap::new(),
                ApplicationTraceTransaction::new(instant),
                root_quota,
            ),
            root_quota.and_then(|root| self.queue.root_profile(root)),
            runenui_core::ProducerPhase::ApplicationUpdate,
        )
    }

    pub(in crate::runtime::application) fn plan_and_commit_application_transaction(
        &mut self,
        input: ApplicationTransactionInput<Action, Protocol>,
        context: ApplicationTransactionCommitContext,
        producer_budget: Option<runenui_core::ProducerBudget>,
        phase: runenui_core::ProducerPhase,
    ) -> Result<(), CommitError> {
        let root_quota = context.root_quota;
        let state = self.state();
        let plan = PlannedApplicationTransaction::plan(
            input,
            &self.work,
            &self.queue,
            state,
            self.action_producer_budget,
            crate::transaction::TransactionProducerContext {
                root_quota,
                producer_budget,
                phase,
            },
        )
        .map_err(CommitError::from)?;
        self.commit_planned_application_transaction(plan, context)
    }

    pub(in crate::runtime::application) fn commit_planned_application_transaction(
        &mut self,
        plan: PlannedApplicationTransaction<Action, Protocol>,
        context: ApplicationTransactionCommitContext,
    ) -> Result<(), CommitError> {
        let ApplicationTransactionCommitContext {
            subscription_duplicates: application_subscription_duplicates,
            subscription_cancelled: application_subscription_cancelled,
            transaction_parent,
            cancellation_lineage: pre_recorded_cancellation_lineage,
            trace_transaction,
            root_quota,
        } = context;
        let PlannedApplicationTransaction {
            invalidated,
            starts,
            application_outputs,
            application_subscription_starts,
            mounted_outputs,
            mounted_subscription_dirty,
            next_generation,
            semantic_events,
        } = plan;
        let required_trace_records = required_application_transaction_trace_records_from_parts(
            &invalidated,
            &starts,
            &application_outputs,
            &mounted_outputs,
        )
        .ok_or(CommitError::Registry)?;
        if !self
            .trace
            .can_admit(MandatoryTracePlan::planned_scheduler_transaction(
                required_trace_records,
            ))
        {
            return Err(CommitError::Registry);
        }
        let work_trace =
            PlannedWorkTrace::new(transaction_parent, trace_transaction.logical_time())
                .with_pre_recorded_lineage(pre_recorded_cancellation_lineage);
        let cancellation_lineage = self.commit_application_starts(
            &invalidated,
            starts,
            next_generation,
            semantic_events,
            work_trace,
            root_quota,
        )?;
        if let Some(root_quota) = root_quota {
            self.append_cancellation_envelopes_rooted(
                &invalidated,
                &cancellation_lineage,
                root_quota,
            );
        } else {
            self.append_cancellation_envelopes(&invalidated, &cancellation_lineage);
        }
        self.commit_mounted_subscription_reconciles(
            mounted_subscription_dirty,
            transaction_parent,
            root_quota,
        );
        self.append_planned_outputs(
            application_outputs,
            transaction_parent,
            trace_transaction.logical_time(),
            root_quota,
        )?;
        let application_subscription_started = application_subscription_starts.len();
        self.commit_application_subscription_starts(application_subscription_starts, root_quota);
        self.append_planned_outputs(
            mounted_outputs,
            transaction_parent,
            trace_transaction.logical_time(),
            root_quota,
        )?;
        self.record_subscription_duplicates(
            &WorkOwner::Application,
            &application_subscription_duplicates,
        );
        self.trace.record_draft(
            trace_transaction
                .fact(TraceRecordKind::SubscriptionDiffCommitted {
                    started: application_subscription_started,
                    cancelled: application_subscription_cancelled,
                    duplicate_keys: application_subscription_duplicates.len(),
                })
                .with_causal_parent(transaction_parent),
        );
        Ok(())
    }

    fn commit_mounted_subscription_reconciles(
        &mut self,
        owners: Vec<crate::MountedNodeId>,
        transaction_parent: Option<TraceSequence>,
        root_quota: Option<crate::queue::RootQuotaId>,
    ) {
        for owner in owners {
            if let Some(root_quota) = root_quota {
                self.queue
                    .push_mounted_subscription_reconcile_rooted(
                        owner.clone(),
                        transaction_parent,
                        root_quota,
                    )
                    .unwrap_or_else(|_| unreachable!("application root quota was preflighted"));
            } else {
                self.queue
                    .push_mounted_subscription_reconcile(owner.clone(), transaction_parent)
                    .unwrap_or_else(|_| unreachable!("application transaction was preflighted"));
            }
            self.mounted_subscription_reconcile_pending.push(owner);
        }
    }

    fn commit_application_subscription_starts(
        &mut self,
        generations: Vec<crate::work::WorkGeneration>,
        root_quota: Option<crate::queue::RootQuotaId>,
    ) {
        for generation in generations {
            if let Some(root_quota) = root_quota {
                self.queue
                    .push_effect_start_rooted(generation, root_quota)
                    .unwrap_or_else(|_| unreachable!("application root quota was preflighted"));
            } else {
                self.queue
                    .push_effect_start(generation)
                    .unwrap_or_else(|_| unreachable!("application transaction was preflighted"));
            }
        }
    }

    pub(in crate::runtime) fn commit_application_starts(
        &mut self,
        invalidated: &[crate::work::WorkGeneration],
        starts: Vec<crate::transaction::PlannedOwnedStart<Action, Protocol>>,
        next_generation: Option<core::num::NonZeroU64>,
        semantic_events: Vec<PlannedWorkSemanticEvent>,
        work_trace: PlannedWorkTrace,
        root_quota: Option<crate::queue::RootQuotaId>,
    ) -> Result<HashMap<u64, (TraceWorkIdentity, Option<TraceSequence>)>, CommitError> {
        self.work
            .commit_application_turn_starts(
                starts.iter().map(crate::transaction::planned_start_family),
            )
            .map_err(CommitError::ProducerViolation)?;
        let PlannedWorkTrace {
            transaction_parent,
            logical_time,
            pre_recorded_lineage,
        } = work_trace;
        let invalidated_set: HashSet<_> = invalidated.iter().copied().collect();
        let identities = self.application_start_identities(invalidated, &starts);
        self.work.commit_generation_reservation(next_generation);
        for start in starts {
            if !invalidated_set.contains(&start.generation) {
                self.commit_application_start(start, root_quota);
            }
        }
        let mut semantic_parents: HashMap<_, _> = invalidated
            .iter()
            .map(|generation| (generation.get(), self.work.trace_parent(*generation)))
            .collect();
        let mut lineage = pre_recorded_lineage;
        for event in semantic_events {
            let generation = match event {
                PlannedWorkSemanticEvent::Requested(generation)
                | PlannedWorkSemanticEvent::Invalidated(generation) => generation,
            };
            let identity = identities
                .get(&generation.get())
                .cloned()
                .unwrap_or_else(|| unreachable!("planned semantic event has trace identity"));
            match event {
                PlannedWorkSemanticEvent::Requested(_) => {
                    let requested = self.record_work_fact_with_parent_at(
                        TraceRecordKind::WorkRequested,
                        transaction_parent,
                        identity.clone(),
                        logical_time,
                    );
                    let committed = if identity.family() == crate::TraceWorkFamily::Subscription {
                        let declared = self.record_work_fact_with_parent_at(
                            TraceRecordKind::SubscriptionDeclared,
                            requested,
                            identity.clone(),
                            logical_time,
                        );
                        self.record_work_fact_with_parent_at(
                            TraceRecordKind::WorkGenerationCommitted,
                            declared,
                            identity,
                            logical_time,
                        )
                    } else {
                        self.record_work_fact_with_parent_at(
                            TraceRecordKind::WorkGenerationCommitted,
                            requested,
                            identity,
                            logical_time,
                        )
                    };
                    semantic_parents.insert(generation.get(), committed);
                }
                PlannedWorkSemanticEvent::Invalidated(_) => {
                    let parent = semantic_parents
                        .get(&generation.get())
                        .copied()
                        .flatten()
                        .or(transaction_parent);
                    let bound = self.record_work_fact_with_parent_at(
                        TraceRecordKind::WorkCancellationBound,
                        parent,
                        identity.clone(),
                        logical_time,
                    );
                    let invalidated = self.record_work_fact_with_parent_at(
                        TraceRecordKind::WorkLogicallyInvalidated,
                        bound,
                        identity.clone(),
                        logical_time,
                    );
                    semantic_parents.insert(generation.get(), invalidated);
                    lineage.insert(generation.get(), (identity, invalidated));
                    self.invalidate_generation_for_transaction(generation);
                }
            }
        }
        Ok(lineage)
    }

    /// Snapshot trace identities before invalidation removes their live records.
    fn application_start_identities(
        &self,
        invalidated: &[crate::work::WorkGeneration],
        starts: &[crate::transaction::PlannedOwnedStart<Action, Protocol>],
    ) -> HashMap<u64, TraceWorkIdentity> {
        invalidated
            .iter()
            .filter_map(|generation| {
                self.trace_work_identity(*generation)
                    .map(|identity| (generation.get(), identity))
            })
            .chain(starts.iter().map(|start| {
                (
                    start.generation.get(),
                    TraceWorkIdentity::new(
                        trace_work_owner(&start.owner),
                        trace_work_family(start.family),
                        start.generation.get(),
                        start.key.clone(),
                    ),
                )
            }))
            .collect()
    }

    pub(in crate::runtime) fn commit_application_start(
        &mut self,
        start: crate::transaction::PlannedOwnedStart<Action, Protocol>,
        root_quota: Option<crate::queue::RootQuotaId>,
    ) -> TraceWorkIdentity {
        let generation = start.generation;
        if let Some(root_quota) = root_quota {
            let repeating_timer = match &start.payload {
                PlannedStartPayload::Effect(Effect::Timer(timer)) => {
                    Some(timer.__runtime_is_repeating())
                }
                _ => None,
            };
            let retained = match start.family {
                crate::work::WorkFamily::HostRequest => self
                    .queue
                    .retain_root_host_request_lease(root_quota, generation),
                crate::work::WorkFamily::LocalTask => self
                    .queue
                    .retain_root_local_task_lease(root_quota, generation),
                crate::work::WorkFamily::SendTask => self
                    .queue
                    .retain_root_send_task_lease(root_quota, generation),
                crate::work::WorkFamily::Timer => self.queue.retain_root_timer_lease(
                    root_quota,
                    generation,
                    repeating_timer.unwrap_or_else(|| {
                        unreachable!("timer start retains timer effect payload")
                    }),
                ),
                crate::work::WorkFamily::FrameworkService => self
                    .queue
                    .retain_root_framework_service_lease(root_quota, generation),
                _ => self.queue.retain_root_work_lease(root_quota),
            };
            retained.unwrap_or_else(|_| unreachable!("work-start root lease was preflighted"));
        }
        match start.payload {
            PlannedStartPayload::Effect(effect) => {
                self.work.commit_record(
                    generation,
                    start.owner,
                    start.family,
                    start.key,
                    effect,
                    root_quota,
                );
            }
            PlannedStartPayload::Subscription(declaration) => {
                let key = declaration.key.clone();
                self.work.commit_subscription_record(
                    generation,
                    start.owner.clone(),
                    key,
                    root_quota,
                );
                self.subscriptions.push(LiveSubscription::new(
                    generation,
                    start.owner,
                    declaration,
                    self.wake.handle(),
                ));
            }
        }
        self.trace_work_identity(generation)
            .unwrap_or_else(|| unreachable!("committed work has trace identity"))
    }

    pub(in crate::runtime) fn append_planned_outputs(
        &mut self,
        outputs: Vec<PlannedOutput<Action>>,
        transaction_parent: Option<TraceSequence>,
        logical_time: MonotonicInstant,
        root_quota: Option<crate::queue::RootQuotaId>,
    ) -> Result<(), CommitError> {
        for output in outputs {
            match output {
                PlannedOutput::Action {
                    action,
                    producer_budget,
                } => {
                    if let Some(root_quota) = root_quota {
                        self.commit_preflighted_action_from_root(
                            action,
                            producer_budget,
                            root_quota,
                            transaction_parent,
                            None,
                            TraceActionCategory::ApplicationEffect,
                            logical_time,
                        )
                        .map_err(|_| CommitError::Registry)?;
                    } else {
                        self.admit_and_commit_action(
                            action,
                            transaction_parent,
                            None,
                            TraceActionCategory::ApplicationEffect,
                            logical_time,
                        )
                        .map_err(|_| CommitError::Registry)?;
                    }
                }
                PlannedOutput::Start(generation) => {
                    if let Some(root_quota) = root_quota {
                        self.queue
                            .push_effect_start_rooted(generation, root_quota)
                            .unwrap_or_else(|_| {
                                unreachable!("application root quota was preflighted")
                            });
                    } else {
                        self.queue
                            .push_effect_start(generation)
                            .unwrap_or_else(|_| {
                                unreachable!("application transaction was preflighted")
                            });
                    }
                }
                PlannedOutput::Redraw => self.request_redraw(transaction_parent, logical_time),
            }
        }
        Ok(())
    }
}
