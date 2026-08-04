use runenui_core::MonotonicInstant;

use super::{
    ApplicationActionOrigin, ApplicationTraceTransaction, ApplicationTransactionInput, CommitError,
    HashMap, HashSet, HostProtocol, LiveSubscription, MandatoryTracePlan,
    PlannedApplicationTransaction, PlannedOutput, PlannedStartPayload, PlannedWorkSemanticEvent,
    Runtime, TraceRecordKind, TraceSequence, TraceWorkIdentity, WorkOwner,
    required_application_transaction_trace_records_from_parts, trace_work_family, trace_work_owner,
};

/// Neutral trace ownership shared by application and routed work-plan commits.
#[derive(Debug)]
pub(in crate::runtime) struct PlannedWorkTrace {
    transaction_parent: Option<TraceSequence>,
    logical_time: MonotonicInstant,
    pre_recorded_lineage: HashMap<u64, (TraceWorkIdentity, Option<TraceSequence>)>,
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
    pub(in crate::runtime) fn plan_and_commit_application_transaction(
        &mut self,
        input: ApplicationTransactionInput<Action, Protocol>,
        application_subscription_duplicates: &HashSet<runenui_core::WorkKey>,
        application_subscription_cancelled: usize,
        transaction_parent: Option<TraceSequence>,
        pre_recorded_cancellation_lineage: HashMap<u64, (TraceWorkIdentity, Option<TraceSequence>)>,
        trace_transaction: ApplicationTraceTransaction,
    ) -> Result<(), CommitError> {
        let plan = PlannedApplicationTransaction::plan(input, &self.work, &self.queue)
            .map_err(|_| CommitError::Registry)?;
        self.commit_planned_application_transaction(
            plan,
            application_subscription_duplicates,
            application_subscription_cancelled,
            transaction_parent,
            pre_recorded_cancellation_lineage,
            trace_transaction,
        )
    }

    pub(in crate::runtime) fn commit_planned_application_transaction(
        &mut self,
        plan: PlannedApplicationTransaction<Action, Protocol>,
        application_subscription_duplicates: &HashSet<runenui_core::WorkKey>,
        application_subscription_cancelled: usize,
        transaction_parent: Option<TraceSequence>,
        pre_recorded_cancellation_lineage: HashMap<u64, (TraceWorkIdentity, Option<TraceSequence>)>,
        trace_transaction: ApplicationTraceTransaction,
    ) -> Result<(), CommitError> {
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
        let work_trace = PlannedWorkTrace::new(transaction_parent, trace_transaction.logical_time())
            .with_pre_recorded_lineage(pre_recorded_cancellation_lineage);
        let cancellation_lineage = self.commit_application_starts(
            &invalidated,
            starts,
            next_generation,
            semantic_events,
            work_trace,
        );
        self.append_cancellation_envelopes(&invalidated, &cancellation_lineage);
        for owner in mounted_subscription_dirty {
            self.queue
                .push_mounted_subscription_reconcile(owner.clone(), transaction_parent)
                .unwrap_or_else(|_| unreachable!("application transaction was preflighted"));
            self.mounted_subscription_reconcile_pending.push(owner);
        }
        self.append_planned_outputs(application_outputs, transaction_parent)?;
        let application_subscription_started = application_subscription_starts.len();
        for generation in application_subscription_starts {
            self.queue
                .push_effect_start(generation)
                .unwrap_or_else(|_| unreachable!("application transaction was preflighted"));
        }
        self.append_planned_outputs(mounted_outputs, transaction_parent)?;
        self.record_subscription_duplicates(
            &WorkOwner::Application,
            application_subscription_duplicates,
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

    pub(in crate::runtime) fn commit_application_starts(
        &mut self,
        invalidated: &[crate::work::WorkGeneration],
        starts: Vec<crate::transaction::PlannedOwnedStart<Action, Protocol>>,
        next_generation: Option<core::num::NonZeroU64>,
        semantic_events: Vec<PlannedWorkSemanticEvent>,
        work_trace: PlannedWorkTrace,
    ) -> HashMap<u64, (TraceWorkIdentity, Option<TraceSequence>)> {
        let PlannedWorkTrace {
            transaction_parent,
            logical_time,
            pre_recorded_lineage,
        } = work_trace;
        let invalidated_set: HashSet<_> = invalidated.iter().copied().collect();
        let mut identities: HashMap<_, _> = invalidated
            .iter()
            .filter_map(|generation| {
                self.trace_work_identity(*generation)
                    .map(|identity| (generation.get(), identity))
            })
            .collect();
        identities.extend(starts.iter().map(|start| {
            (
                start.generation.get(),
                TraceWorkIdentity::new(
                    trace_work_owner(&start.owner),
                    trace_work_family(start.family),
                    start.generation.get(),
                    start.key.clone(),
                ),
            )
        }));
        self.work.commit_generation_reservation(next_generation);
        for start in starts {
            if !invalidated_set.contains(&start.generation) {
                self.commit_application_start(start);
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
                    self.invalidate_generation_now(generation);
                }
            }
        }
        lineage
    }

    pub(in crate::runtime) fn commit_application_start(
        &mut self,
        start: crate::transaction::PlannedOwnedStart<Action, Protocol>,
    ) -> TraceWorkIdentity {
        let generation = start.generation;
        match start.payload {
            PlannedStartPayload::Effect(effect) => {
                self.work
                    .commit_record(generation, start.owner, start.family, start.key, effect);
            }
            PlannedStartPayload::Subscription(declaration) => {
                let key = declaration.key.clone();
                self.work
                    .commit_subscription_record(generation, start.owner.clone(), key);
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
    ) -> Result<(), CommitError> {
        for output in outputs {
            match output {
                PlannedOutput::Action(action) => {
                    self.commit_preflighted_action(
                        action,
                        transaction_parent,
                        None,
                        ApplicationActionOrigin::ApplicationEffect,
                    )
                    .map_err(|_| CommitError::Registry)?;
                }
                PlannedOutput::Start(generation) => {
                    self.queue
                        .push_effect_start(generation)
                        .unwrap_or_else(|_| {
                            unreachable!("application transaction was preflighted")
                        });
                }
                PlannedOutput::Redraw => self.request_redraw(),
            }
        }
        Ok(())
    }
}
