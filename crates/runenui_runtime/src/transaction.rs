//! Provisional ordered output planning and atomic queue/registry commit inputs.

#![allow(clippy::redundant_pub_crate)]

use std::collections::{HashMap, HashSet};

use runenui_core::{
    __runtime::{Effect, Subscription},
    Effects, HostProtocol, ProducerBudget, WorkKey,
};

use crate::{
    MountedNodeId,
    queue::{
        RootProducerCancellations, RootProducerCommit, RootProducerResponseReleases,
        RootProducerWorkStarts, RootQuotaId, WorkQueue,
    },
    work::{WorkFamily, WorkGeneration, WorkOwner, WorkRegistry},
};

pub(crate) struct OwnedTransactionLedger<Action, Protocol: HostProtocol> {
    pub(crate) owner: WorkOwner,
    pub(crate) ledger: TransactionLedger<Action, Protocol>,
}

pub(crate) struct ApplicationTransactionInput<Action, Protocol: HostProtocol> {
    pub(crate) lifecycle_invalidated: Vec<WorkGeneration>,
    pub(crate) mounted_subscription_dirty: Vec<MountedNodeId>,
    pub(crate) application: TransactionLedger<Action, Protocol>,
    pub(crate) application_subscription_invalidated: Vec<WorkGeneration>,
    pub(crate) application_subscription_starts: Vec<Subscription<Action>>,
    pub(crate) mounted: Vec<OwnedTransactionLedger<Action, Protocol>>,
}

pub(crate) struct PlannedApplicationTransaction<Action, Protocol: HostProtocol> {
    pub(crate) invalidated: Vec<WorkGeneration>,
    pub(crate) starts: Vec<PlannedOwnedStart<Action, Protocol>>,
    pub(crate) application_outputs: Vec<PlannedOutput<Action>>,
    pub(crate) application_subscription_starts: Vec<WorkGeneration>,
    pub(crate) mounted_outputs: Vec<PlannedOutput<Action>>,
    pub(crate) mounted_subscription_dirty: Vec<MountedNodeId>,
    pub(crate) next_generation: Option<core::num::NonZeroU64>,
    pub(crate) semantic_events: Vec<PlannedWorkSemanticEvent>,
}

pub(crate) struct PlannedOwnedStart<Action, Protocol: HostProtocol> {
    pub(crate) generation: WorkGeneration,
    pub(crate) owner: WorkOwner,
    pub(crate) family: WorkFamily,
    pub(crate) key: Option<WorkKey>,
    pub(crate) payload: PlannedStartPayload<Action, Protocol>,
}

pub(crate) enum PlannedStartPayload<Action, Protocol: HostProtocol> {
    Effect(Effect<Action, Protocol>),
    Subscription(Subscription<Action>),
}

pub(crate) struct TransactionLedger<Action, Protocol: HostProtocol> {
    outputs: Vec<Effect<Action, Protocol>>,
}

impl<Action, Protocol: HostProtocol> TransactionLedger<Action, Protocol> {
    pub(crate) fn collect(
        effects: Effects<Action, Protocol>,
        limit: usize,
    ) -> Result<Self, TransactionOutputError> {
        let outputs = effects.__runtime_into_items();
        if outputs.len() > limit {
            return Err(TransactionOutputError::Full {
                limit,
                attempted: outputs.len(),
            });
        }
        Ok(Self { outputs })
    }

    pub(crate) fn into_outputs(self) -> Vec<Effect<Action, Protocol>> {
        self.outputs
    }

    pub(crate) fn from_outputs(
        outputs: Vec<Effect<Action, Protocol>>,
        limit: usize,
    ) -> Result<Self, TransactionOutputError> {
        if outputs.len() > limit {
            return Err(TransactionOutputError::Full {
                limit,
                attempted: outputs.len(),
            });
        }
        Ok(Self { outputs })
    }

    pub(crate) const fn len(&self) -> usize {
        self.outputs.len()
    }

    pub(crate) fn work_start_budget(&self) -> Option<runenui_core::WorkStartBudget> {
        let mut counts = [0usize; 5];
        for effect in &self.outputs {
            let index = match effect_family(effect) {
                Some(WorkFamily::LocalTask) => Some(0),
                Some(WorkFamily::SendTask) => Some(1),
                Some(WorkFamily::Timer) => Some(2),
                Some(WorkFamily::HostRequest) => Some(3),
                Some(WorkFamily::FrameworkService) => Some(4),
                Some(WorkFamily::Subscription) | None => None,
            };
            if let Some(index) = index {
                counts[index] = counts[index].checked_add(1)?;
            }
        }
        Some(runenui_core::WorkStartBudget::new(
            counts[0], counts[1], counts[2], counts[3], counts[4],
        ))
    }

    pub(crate) fn repeating_timer_start_count(&self) -> Option<usize> {
        self.outputs.iter().try_fold(0usize, |count, effect| {
            if matches!(effect, Effect::Timer(timer) if timer.__runtime_is_repeating()) {
                count.checked_add(1)
            } else {
                Some(count)
            }
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum PlannedWorkSemanticEvent {
    Requested(WorkGeneration),
    Invalidated(WorkGeneration),
}

pub(crate) enum PlannedOutput<Action> {
    Action {
        action: Action,
        producer_budget: ProducerBudget,
    },
    Start(WorkGeneration),
    Redraw,
}

#[derive(Clone, Eq, Hash, PartialEq)]
struct OwnedProvisionalKey {
    owner: WorkOwner,
    family: WorkFamily,
    key: WorkKey,
}

struct PreparedApplicationTransaction<Action, Protocol: HostProtocol> {
    lifecycle_invalidated: Vec<WorkGeneration>,
    mounted_subscription_dirty: Vec<MountedNodeId>,
    application_effects: Vec<Effect<Action, Protocol>>,
    application_subscription_invalidated: Vec<WorkGeneration>,
    application_subscription_starts: Vec<Subscription<Action>>,
    mounted_effects: Vec<(WorkOwner, Vec<Effect<Action, Protocol>>)>,
}

impl<Action, Protocol: HostProtocol> PreparedApplicationTransaction<Action, Protocol> {
    fn new(input: ApplicationTransactionInput<Action, Protocol>) -> Self {
        Self {
            lifecycle_invalidated: input.lifecycle_invalidated,
            mounted_subscription_dirty: input.mounted_subscription_dirty,
            application_effects: input.application.into_outputs(),
            application_subscription_invalidated: input.application_subscription_invalidated,
            application_subscription_starts: input.application_subscription_starts,
            mounted_effects: input
                .mounted
                .into_iter()
                .map(|batch| (batch.owner, batch.ledger.into_outputs()))
                .collect(),
        }
    }

    fn start_count(&self) -> Result<usize, TransactionPlanError> {
        let application = self
            .application_effects
            .iter()
            .filter(|effect| effect_family(effect).is_some())
            .count()
            .checked_add(self.application_subscription_starts.len())
            .ok_or(TransactionPlanError::WorkGenerationExhausted)?;
        self.mounted_effects
            .iter()
            .try_fold(application, |count, (_, effects)| {
                count.checked_add(
                    effects
                        .iter()
                        .filter(|effect| effect_family(effect).is_some())
                        .count(),
                )
            })
            .ok_or(TransactionPlanError::WorkGenerationExhausted)
    }

    fn output_count(&self) -> Result<usize, TransactionPlanError> {
        self.mounted_effects
            .iter()
            .try_fold(self.application_effects.len(), |count, (_, effects)| {
                count.checked_add(effects.len())
            })
            .ok_or(TransactionPlanError::ProducerQuotaExceeded)
    }
}

struct PlannedOutputBatches<Action, Protocol: HostProtocol> {
    invalidated: Vec<WorkGeneration>,
    invalidated_set: HashSet<WorkGeneration>,
    starts: Vec<PlannedOwnedStart<Action, Protocol>>,
    application_outputs: Vec<PlannedOutput<Action>>,
    application_subscription_starts: Vec<WorkGeneration>,
    mounted_outputs: Vec<PlannedOutput<Action>>,
    mounted_subscription_dirty: Vec<MountedNodeId>,
    next_generation: Option<core::num::NonZeroU64>,
    semantic_events: Vec<PlannedWorkSemanticEvent>,
}

fn plan_output_batches<State, Action, Protocol: HostProtocol>(
    prepared: PreparedApplicationTransaction<Action, Protocol>,
    start_count: usize,
    work: &WorkRegistry<Action, Protocol>,
    state: &State,
    action_budgeter: Option<fn(&State, &Action) -> ProducerBudget>,
) -> Result<PlannedOutputBatches<Action, Protocol>, TransactionPlanError> {
    let PreparedApplicationTransaction {
        lifecycle_invalidated,
        mounted_subscription_dirty,
        application_effects,
        application_subscription_invalidated,
        application_subscription_starts,
        mounted_effects,
    } = prepared;
    let (generations, next_generation) = work.preview_generations(start_count)?;
    let mut generations = generations.into_iter();
    let mut provisional = HashMap::<OwnedProvisionalKey, WorkGeneration>::new();
    let mut invalidated_set = HashSet::new();
    let mut invalidated = Vec::new();
    for generation in lifecycle_invalidated {
        insert_invalidation(generation, &mut invalidated_set, &mut invalidated);
    }
    let mut semantic_events = Vec::with_capacity(start_count.saturating_mul(2));

    let mut starts = Vec::with_capacity(start_count);
    let mut application_outputs = Vec::with_capacity(application_effects.len());
    plan_owned_effects(
        &WorkOwner::Application,
        application_effects,
        &mut generations,
        work,
        &mut provisional,
        &mut invalidated_set,
        &mut invalidated,
        &mut starts,
        &mut application_outputs,
        &mut semantic_events,
        state,
        action_budgeter,
    );

    for generation in application_subscription_invalidated {
        if insert_invalidation(generation, &mut invalidated_set, &mut invalidated) {
            semantic_events.push(PlannedWorkSemanticEvent::Invalidated(generation));
        }
    }

    let application_subscription_starts = plan_application_subscription_starts(
        application_subscription_starts,
        &mut generations,
        &mut provisional,
        &mut starts,
        &mut semantic_events,
    );

    let mounted_output_capacity = mounted_effects
        .iter()
        .map(|(_, effects)| effects.len())
        .sum();
    let mut mounted_outputs = Vec::with_capacity(mounted_output_capacity);
    for (owner, effects) in mounted_effects {
        plan_owned_effects(
            &owner,
            effects,
            &mut generations,
            work,
            &mut provisional,
            &mut invalidated_set,
            &mut invalidated,
            &mut starts,
            &mut mounted_outputs,
            &mut semantic_events,
            state,
            action_budgeter,
        );
    }

    Ok(PlannedOutputBatches {
        invalidated,
        invalidated_set,
        starts,
        application_outputs,
        application_subscription_starts,
        mounted_outputs,
        mounted_subscription_dirty,
        next_generation,
        semantic_events,
    })
}

struct RootProducerPreflight<'a, Action, Protocol: HostProtocol> {
    queue: &'a WorkQueue<Action>,
    root_quota: RootQuotaId,
    starts: &'a [PlannedOwnedStart<Action, Protocol>],
    invalidated: &'a [WorkGeneration],
    invalidated_set: &'a HashSet<WorkGeneration>,
    sequenced_outputs: usize,
    application_outputs: &'a [PlannedOutput<Action>],
    mounted_outputs: &'a [PlannedOutput<Action>],
}

fn root_work_start_counts(
    starts: impl Iterator<Item = (WorkGeneration, WorkFamily, bool)> + Clone,
    invalidated: &HashSet<WorkGeneration>,
) -> (usize, RootProducerWorkStarts) {
    let live = starts.filter(|(generation, _, _)| !invalidated.contains(generation));
    let count_family = |family| live.clone().filter(|(_, kind, _)| *kind == family).count();
    (
        live.clone().count(),
        RootProducerWorkStarts {
            host_requests: count_family(WorkFamily::HostRequest),
            local_tasks: count_family(WorkFamily::LocalTask),
            send_tasks: count_family(WorkFamily::SendTask),
            timers: count_family(WorkFamily::Timer),
            repeating_timers: live
                .clone()
                .filter(|(_, kind, repeating)| *kind == WorkFamily::Timer && *repeating)
                .count(),
            framework_services: count_family(WorkFamily::FrameworkService),
        },
    )
}

fn root_work_cancellations<Action>(
    queue: &WorkQueue<Action>,
    root_quota: RootQuotaId,
    invalidated: &[WorkGeneration],
) -> Result<(RootProducerCancellations, RootProducerResponseReleases), TransactionPlanError> {
    let (host_requests, host_response_release) = queue
        .preflight_host_request_cancellations(invalidated, root_quota)
        .map_err(|_| TransactionPlanError::ProducerQuotaExceeded)?;
    let (framework_services, framework_service_response_release) = queue
        .preflight_framework_service_cancellations(invalidated, root_quota)
        .map_err(|_| TransactionPlanError::ProducerQuotaExceeded)?;
    let local_tasks = queue
        .preflight_local_task_cancellations(invalidated)
        .map_err(|_| TransactionPlanError::ProducerQuotaExceeded)?;
    let send_tasks = queue
        .preflight_send_task_cancellations(invalidated)
        .map_err(|_| TransactionPlanError::ProducerQuotaExceeded)?;
    let timers = queue
        .preflight_timer_cancellations(invalidated)
        .map_err(|_| TransactionPlanError::ProducerQuotaExceeded)?;
    Ok((
        RootProducerCancellations {
            host_requests,
            framework_services,
            local_tasks,
            send_tasks,
            timers,
        },
        RootProducerResponseReleases {
            host_requests: host_response_release,
            framework_services: framework_service_response_release,
        },
    ))
}

fn preflight_root_producer_commit<Action, Protocol: HostProtocol>(
    preflight: &RootProducerPreflight<'_, Action, Protocol>,
) -> Result<(), TransactionPlanError> {
    let queue = preflight.queue;
    let root_quota = preflight.root_quota;
    let starts = preflight.starts;
    let invalidated = preflight.invalidated;
    let invalidated_set = preflight.invalidated_set;
    let sequenced_outputs = preflight.sequenced_outputs;
    let application_outputs = preflight.application_outputs;
    let mounted_outputs = preflight.mounted_outputs;
    let (live_start_count, starts) = root_work_start_counts(starts.iter().map(|start| (start.generation,start.family,matches!(&start.payload,PlannedStartPayload::Effect(Effect::Timer(timer)) if timer.__runtime_is_repeating()))), invalidated_set);
    let (cancellations, response_releases) =
        root_work_cancellations(queue, root_quota, invalidated)?;
    let root_commit = RootProducerCommit {
        root: root_quota,
        queued_envelopes: sequenced_outputs,
        starts,
        cancellations,
        response_releases,
    };
    if queue.preflight_root_producer_commit(root_commit).is_err() {
        return Err(TransactionPlanError::ProducerQuotaExceeded);
    }
    queue
        .preflight_root_work_leases(root_quota, live_start_count)
        .map_err(|_| TransactionPlanError::ProducerQuotaExceeded)?;
    for producer_budget in application_outputs
        .iter()
        .chain(mounted_outputs)
        .filter_map(|output| match output {
            PlannedOutput::Action {
                producer_budget, ..
            } => Some(*producer_budget),
            PlannedOutput::Start(_) | PlannedOutput::Redraw => None,
        })
    {
        if !queue.root_covers_action_budget(root_quota, producer_budget) {
            return Err(TransactionPlanError::ProducerQuotaExceeded);
        }
    }
    Ok(())
}

impl<Action, Protocol: HostProtocol> PlannedApplicationTransaction<Action, Protocol> {
    pub(crate) fn plan<State>(
        input: ApplicationTransactionInput<Action, Protocol>,
        work: &WorkRegistry<Action, Protocol>,
        queue: &WorkQueue<Action>,
        state: &State,
        action_budgeter: Option<fn(&State, &Action) -> ProducerBudget>,
        root_quota: Option<RootQuotaId>,
        producer_budget: Option<ProducerBudget>,
    ) -> Result<Self, TransactionPlanError> {
        let prepared = PreparedApplicationTransaction::new(input);
        let start_count = prepared.start_count()?;
        if let Some(producer_budget) = producer_budget {
            let output_count = prepared.output_count()?;
            if output_count > producer_budget.outputs()
                || prepared.application_subscription_starts.len()
                    > producer_budget.application_subscriptions()
                || prepared.mounted_subscription_dirty.len()
                    > producer_budget.dirty_subscription_owners()
            {
                return Err(TransactionPlanError::ProducerQuotaExceeded);
            }
        }
        let PlannedOutputBatches {
            invalidated,
            invalidated_set,
            starts,
            application_outputs,
            application_subscription_starts,
            mounted_outputs,
            mounted_subscription_dirty,
            next_generation,
            semantic_events,
        } = plan_output_batches(prepared, start_count, work, state, action_budgeter)?;

        let sequenced_outputs = application_outputs
            .iter()
            .chain(&mounted_outputs)
            .filter(|output| !matches!(output, PlannedOutput::Redraw))
            .count()
            .checked_add(application_subscription_starts.len())
            .and_then(|count| count.checked_add(invalidated.len()))
            .and_then(|count| count.checked_add(mounted_subscription_dirty.len()))
            .ok_or(TransactionPlanError::QueueFull)?;
        if let Some(producer_budget) = producer_budget {
            let start_budget = producer_budget.work_starts();
            if sequenced_outputs > producer_budget.queue_credits() {
                return Err(TransactionPlanError::ProducerQuotaExceeded);
            }
            let start_exceeds = [
                (WorkFamily::LocalTask, runenui_core::WorkFamily::LocalTask),
                (WorkFamily::SendTask, runenui_core::WorkFamily::SendTask),
                (WorkFamily::Timer, runenui_core::WorkFamily::Timer),
                (
                    WorkFamily::HostRequest,
                    runenui_core::WorkFamily::HostRequest,
                ),
                (
                    WorkFamily::FrameworkService,
                    runenui_core::WorkFamily::FrameworkService,
                ),
            ]
            .into_iter()
            .any(|(runtime_family, authored_family)| {
                starts
                    .iter()
                    .filter(|start| start.family == runtime_family)
                    .count()
                    > start_budget.for_family(authored_family)
            });
            if start_exceeds {
                return Err(TransactionPlanError::ProducerQuotaExceeded);
            }
        }
        if let Some(root_quota) = root_quota {
            preflight_root_producer_commit(&RootProducerPreflight {
                queue,
                root_quota,
                starts: &starts,
                invalidated: &invalidated,
                invalidated_set: &invalidated_set,
                sequenced_outputs,
                application_outputs: &application_outputs,
                mounted_outputs: &mounted_outputs,
            })?;
        } else {
            queue.preflight_commit(sequenced_outputs)?;
        }
        work.preflight_planned_families(
            &invalidated_set,
            &starts.iter().map(|start| (start.generation, start.family)),
            queue,
            root_quota,
        )?;
        Ok(Self {
            invalidated,
            starts,
            application_outputs,
            application_subscription_starts,
            mounted_outputs,
            mounted_subscription_dirty,
            next_generation,
            semantic_events,
        })
    }
}

fn plan_application_subscription_starts<Action, Protocol: HostProtocol>(
    declarations: Vec<Subscription<Action>>,
    generations: &mut impl Iterator<Item = WorkGeneration>,
    provisional: &mut HashMap<OwnedProvisionalKey, WorkGeneration>,
    starts: &mut Vec<PlannedOwnedStart<Action, Protocol>>,
    semantic_events: &mut Vec<PlannedWorkSemanticEvent>,
) -> Vec<WorkGeneration> {
    let mut planned = Vec::with_capacity(declarations.len());
    for declaration in declarations {
        let generation = generations
            .next()
            .unwrap_or_else(|| unreachable!("start count and generations agree"));
        let key = declaration.key.clone();
        provisional.insert(
            OwnedProvisionalKey {
                owner: WorkOwner::Application,
                family: WorkFamily::Subscription,
                key: key.clone(),
            },
            generation,
        );
        semantic_events.push(PlannedWorkSemanticEvent::Requested(generation));
        starts.push(PlannedOwnedStart {
            generation,
            owner: WorkOwner::Application,
            family: WorkFamily::Subscription,
            key: Some(key),
            payload: PlannedStartPayload::Subscription(declaration),
        });
        planned.push(generation);
    }
    planned
}

#[allow(clippy::too_many_arguments)]
fn plan_owned_effects<State, Action, Protocol: HostProtocol>(
    owner: &WorkOwner,
    effects: Vec<Effect<Action, Protocol>>,
    generations: &mut impl Iterator<Item = WorkGeneration>,
    work: &WorkRegistry<Action, Protocol>,
    provisional: &mut HashMap<OwnedProvisionalKey, WorkGeneration>,
    invalidated_set: &mut HashSet<WorkGeneration>,
    invalidated: &mut Vec<WorkGeneration>,
    starts: &mut Vec<PlannedOwnedStart<Action, Protocol>>,
    outputs: &mut Vec<PlannedOutput<Action>>,
    semantic_events: &mut Vec<PlannedWorkSemanticEvent>,
    state: &State,
    action_budgeter: Option<fn(&State, &Action) -> ProducerBudget>,
) {
    for effect in effects {
        if let Effect::Cancel { family, key } = effect {
            let family = family.into();
            let identity = OwnedProvisionalKey {
                owner: owner.clone(),
                family,
                key,
            };
            let target = provisional
                .get(&identity)
                .copied()
                .or_else(|| work.current(&identity.owner, family, &identity.key));
            if let Some(target) = target
                && insert_invalidation(target, invalidated_set, invalidated)
            {
                semantic_events.push(PlannedWorkSemanticEvent::Invalidated(target));
            }
            continue;
        }
        if let Effect::Action(action) = effect {
            let producer_budget = action_budgeter.map_or_else(
                || ProducerBudget::new(1, 0),
                |profile| profile(state, &action),
            );
            outputs.push(PlannedOutput::Action {
                action,
                producer_budget,
            });
            continue;
        }
        if matches!(effect, Effect::Redraw) {
            outputs.push(PlannedOutput::Redraw);
            continue;
        }
        let generation = generations
            .next()
            .unwrap_or_else(|| unreachable!("start count and generations agree"));
        let family =
            effect_family(&effect).unwrap_or_else(|| unreachable!("work effect has a family"));
        let key = effect_key(&effect).cloned();
        if let Some(key) = key.as_ref() {
            let identity = OwnedProvisionalKey {
                owner: owner.clone(),
                family,
                key: key.clone(),
            };
            let replaced = provisional
                .get(&identity)
                .copied()
                .or_else(|| work.current(owner, family, key));
            if let Some(replaced) = replaced
                && insert_invalidation(replaced, invalidated_set, invalidated)
            {
                semantic_events.push(PlannedWorkSemanticEvent::Invalidated(replaced));
            }
            provisional.insert(identity, generation);
        }
        semantic_events.push(PlannedWorkSemanticEvent::Requested(generation));
        starts.push(PlannedOwnedStart {
            generation,
            owner: owner.clone(),
            family,
            key,
            payload: PlannedStartPayload::Effect(effect),
        });
        outputs.push(PlannedOutput::Start(generation));
    }
}

fn insert_invalidation(
    generation: WorkGeneration,
    invalidated_set: &mut HashSet<WorkGeneration>,
    invalidated: &mut Vec<WorkGeneration>,
) -> bool {
    if invalidated_set.insert(generation) {
        invalidated.push(generation);
        true
    } else {
        false
    }
}

const fn effect_family<Action, Protocol: HostProtocol>(
    effect: &Effect<Action, Protocol>,
) -> Option<WorkFamily> {
    match effect {
        Effect::LocalTask(_) => Some(WorkFamily::LocalTask),
        Effect::SendTask(_) => Some(WorkFamily::SendTask),
        Effect::Timer(_) => Some(WorkFamily::Timer),
        Effect::HostRequest(_) => Some(WorkFamily::HostRequest),
        Effect::FrameworkService(_) => Some(WorkFamily::FrameworkService),
        Effect::Action(_) | Effect::Cancel { .. } | Effect::Redraw => None,
    }
}

const fn effect_key<Action, Protocol: HostProtocol>(
    effect: &Effect<Action, Protocol>,
) -> Option<&WorkKey> {
    match effect {
        Effect::LocalTask(task) => task.key.as_ref(),
        Effect::SendTask(task) => task.key.as_ref(),
        Effect::Timer(timer) => timer.__runtime_key(),
        Effect::HostRequest(request) => request.key.as_ref(),
        Effect::FrameworkService(_)
        | Effect::Action(_)
        | Effect::Cancel { .. }
        | Effect::Redraw => None,
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum TransactionOutputError {
    Full { limit: usize, attempted: usize },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum TransactionPlanError {
    QueueFull,
    ProducerQuotaExceeded,
    WorkSequenceExhausted,
    RegistryFull,
    WorkGenerationExhausted,
}

impl From<crate::queue::QueueCommitError> for TransactionPlanError {
    fn from(error: crate::queue::QueueCommitError) -> Self {
        match error {
            crate::queue::QueueCommitError::Full => Self::QueueFull,
            crate::queue::QueueCommitError::SequenceExhausted => Self::WorkSequenceExhausted,
        }
    }
}

impl From<crate::work::RegistryInsertError> for TransactionPlanError {
    fn from(error: crate::work::RegistryInsertError) -> Self {
        match error {
            crate::work::RegistryInsertError::Full => Self::RegistryFull,
            crate::work::RegistryInsertError::GenerationExhausted => Self::WorkGenerationExhausted,
        }
    }
}

/// Immutable authority borrowed by the pre-mutation returned-effects pass.
pub(crate) struct ReturnedEffectsPreflight<'a, State, Action, Protocol: HostProtocol> {
    pub(crate) budget: ProducerBudget,
    pub(crate) phase: runenui_core::ProducerPhase,
    pub(crate) work: &'a WorkRegistry<Action, Protocol>,
    pub(crate) queue: &'a WorkQueue<Action>,
    pub(crate) root: Option<RootQuotaId>,
    pub(crate) state: &'a State,
    pub(crate) action_budgeter: Option<fn(&State, &Action) -> ProducerBudget>,
}

struct ReturnedWorkDemand {
    invalidated: HashSet<WorkGeneration>,
    starts: Vec<(WorkGeneration, WorkFamily, bool)>,
}

impl<Action, Protocol: HostProtocol> TransactionLedger<Action, Protocol> {
    /// Pure demand pass over the entire owned return value. Neither registries,
    /// canonical sequences nor external work are mutated here.
    pub(crate) fn validate_returned<State>(
        &self,
        context: &ReturnedEffectsPreflight<'_, State, Action, Protocol>,
    ) -> Result<(), crate::RuntimeTerminalReason> {
        use runenui_core::{BudgetComponent, ProducerContractViolation, ProducerViolationKind};
        let budget = context.budget;
        let phase = context.phase;
        let work = context.work;
        let queue = context.queue;
        let root = context.root;
        let exceeded = |component, declared, attempted| {
            crate::RuntimeTerminalReason::ProducerContractViolation(ProducerContractViolation {
                kind: ProducerViolationKind::DeclarationExceeded,
                phase,
                component,
                declared,
                attempted: Some(attempted),
            })
        };
        if self.len() > budget.application_outputs() {
            return Err(exceeded(
                BudgetComponent::ApplicationOutputs,
                budget.application_outputs(),
                self.len(),
            ));
        }
        let counts = self
            .work_start_budget()
            .ok_or(crate::RuntimeTerminalReason::Poisoned)?;
        for family in [
            runenui_core::WorkFamily::LocalTask,
            runenui_core::WorkFamily::SendTask,
            runenui_core::WorkFamily::Timer,
            runenui_core::WorkFamily::HostRequest,
            runenui_core::WorkFamily::FrameworkService,
        ] {
            if counts.for_family(family) > budget.work_starts().for_family(family) {
                return Err(exceeded(
                    BudgetComponent::WorkStarts(family),
                    budget.work_starts().for_family(family),
                    counts.for_family(family),
                ));
            }
        }
        let ReturnedWorkDemand {
            invalidated,
            starts,
        } = self.preview_returned_work(context)?;
        for (family, authored) in [
            (WorkFamily::LocalTask, runenui_core::WorkFamily::LocalTask),
            (WorkFamily::SendTask, runenui_core::WorkFamily::SendTask),
            (WorkFamily::Timer, runenui_core::WorkFamily::Timer),
            (
                WorkFamily::HostRequest,
                runenui_core::WorkFamily::HostRequest,
            ),
            (
                WorkFamily::FrameworkService,
                runenui_core::WorkFamily::FrameworkService,
            ),
        ] {
            let added = starts
                .iter()
                .filter(|(generation, kind, _)| {
                    *kind == family && !invalidated.contains(generation)
                })
                .count();
            let required = work
                .resident_family_requirement(family, &invalidated, added, root)
                .ok_or(crate::RuntimeTerminalReason::Poisoned)?;
            let declared = root
                .and_then(|id| queue.root_work_residency(id, family))
                .unwrap_or_else(|| budget.work_residency().for_family(authored));
            if required > declared {
                return Err(exceeded(
                    BudgetComponent::WorkResidency(authored),
                    declared,
                    required,
                ));
            }
        }
        work.preflight_planned_families(
            &invalidated,
            &starts
                .iter()
                .map(|(generation, family, _)| (*generation, *family)),
            queue,
            root,
        )
        .map_err(|_| crate::RuntimeTerminalReason::Poisoned)?;
        self.preflight_returned_queue(context, &invalidated, &starts)
    }

    fn preflight_returned_queue<State>(
        &self,
        context: &ReturnedEffectsPreflight<'_, State, Action, Protocol>,
        invalidated: &HashSet<WorkGeneration>,
        starts: &[(WorkGeneration, WorkFamily, bool)],
    ) -> Result<(), crate::RuntimeTerminalReason> {
        let Some(root) = context.root else {
            return Ok(());
        };
        let (live_start_count, starts) =
            root_work_start_counts(starts.iter().copied(), invalidated);
        let mut invalidated = invalidated.iter().copied().collect::<Vec<_>>();
        invalidated.sort_unstable();
        let sequenced = self
            .outputs
            .iter()
            .filter(|effect| !matches!(effect, Effect::Cancel { .. } | Effect::Redraw))
            .count()
            .checked_add(invalidated.len())
            .ok_or(crate::RuntimeTerminalReason::Poisoned)?;
        if sequenced > context.budget.queue_credits() {
            return Err(crate::RuntimeTerminalReason::ProducerContractViolation(
                runenui_core::ProducerContractViolation {
                    kind: runenui_core::ProducerViolationKind::DeclarationExceeded,
                    phase: context.phase,
                    component: runenui_core::BudgetComponent::QueueFrontier,
                    declared: context.budget.queue_credits(),
                    attempted: Some(sequenced),
                },
            ));
        }
        let (cancellations, response_releases) =
            root_work_cancellations(context.queue, root, &invalidated)
                .map_err(|_| crate::RuntimeTerminalReason::Poisoned)?;
        context
            .queue
            .preflight_root_producer_commit(RootProducerCommit {
                root,
                queued_envelopes: sequenced,
                starts,
                cancellations,
                response_releases,
            })
            .map_err(|error| match error {
                crate::queue::QueueReservationError::SequenceExhausted => {
                    crate::RuntimeTerminalReason::WorkSequenceExhausted
                }
                _ => crate::RuntimeTerminalReason::Poisoned,
            })?;
        context
            .queue
            .preflight_root_work_leases(root, live_start_count)
            .map_err(|_| crate::RuntimeTerminalReason::Poisoned)
    }

    fn preview_returned_work<State>(
        &self,
        context: &ReturnedEffectsPreflight<'_, State, Action, Protocol>,
    ) -> Result<ReturnedWorkDemand, crate::RuntimeTerminalReason> {
        use runenui_core::{ProducerContractViolation, ProducerViolationKind};
        let ReturnedEffectsPreflight {
            budget,
            phase,
            work,
            queue,
            root,
            state,
            action_budgeter,
        } = *context;
        // Use real previewed generation identities and the same key/family rules
        // as final planning. Later provisional replacements never resurrect an
        // invalidated predecessor, and cancelled starts consume no residency.
        let count = self
            .work_start_budget()
            .ok_or(crate::RuntimeTerminalReason::Poisoned)?
            .total()
            .ok_or(crate::RuntimeTerminalReason::Poisoned)?;
        let (generations, _) = work
            .preview_generations(count)
            .map_err(|_| crate::RuntimeTerminalReason::WorkGenerationExhausted)?;
        let mut generations = generations.into_iter();
        let mut provisional = HashMap::new();
        let mut invalidated = HashSet::new();
        let mut starts = Vec::with_capacity(count);
        for effect in &self.outputs {
            if let Effect::Action(action) = effect
                && let Some(budgeter) = action_budgeter
            {
                let child = budgeter(state, action);
                let parent = match root {
                    Some(root) => queue
                        .root_profile(root)
                        .ok_or(crate::RuntimeTerminalReason::Poisoned)?,
                    None => budget,
                };
                if let Some((component, declared, attempted)) = uncovered_component(parent, child) {
                    return Err(crate::RuntimeTerminalReason::ProducerContractViolation(
                        ProducerContractViolation {
                            kind: ProducerViolationKind::DescendantProfileNotCovered,
                            phase,
                            component,
                            declared,
                            attempted: Some(attempted),
                        },
                    ));
                }
            }
            if let Effect::Cancel { family, key } = effect {
                let identity = OwnedProvisionalKey {
                    owner: WorkOwner::Application,
                    family: (*family).into(),
                    key: key.clone(),
                };
                if let Some(target) = provisional
                    .get(&identity)
                    .copied()
                    .or_else(|| work.current(&identity.owner, identity.family, &identity.key))
                {
                    invalidated.insert(target);
                }
                continue;
            }
            let Some(family) = effect_family(effect) else {
                continue;
            };
            let generation = generations
                .next()
                .unwrap_or_else(|| unreachable!("counts and effects agree"));
            if let Some(key) = effect_key(effect) {
                let identity = OwnedProvisionalKey {
                    owner: WorkOwner::Application,
                    family,
                    key: key.clone(),
                };
                if let Some(old) = provisional
                    .insert(identity.clone(), generation)
                    .or_else(|| work.current(&identity.owner, family, key))
                {
                    invalidated.insert(old);
                }
            }
            starts.push((
                generation,
                family,
                matches!(effect,Effect::Timer(timer) if timer.__runtime_is_repeating()),
            ));
        }
        Ok(ReturnedWorkDemand {
            invalidated,
            starts,
        })
    }
}

/// Returns uncharged per-family credits after a complete provisional group.
/// No partial family charge is made on error.
pub(crate) fn remaining_work_starts(
    promised: runenui_core::WorkStartBudget,
    used: runenui_core::WorkStartBudget,
    phase: runenui_core::ProducerPhase,
) -> Result<runenui_core::WorkStartBudget, runenui_core::ProducerContractViolation> {
    let mut remaining = [0; 5];
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
        remaining[index] = promised
            .for_family(family)
            .checked_sub(used.for_family(family))
            .ok_or_else(|| runenui_core::ProducerContractViolation {
                kind: runenui_core::ProducerViolationKind::DeclarationExceeded,
                phase,
                component: runenui_core::BudgetComponent::WorkStarts(family),
                declared: promised.for_family(family),
                attempted: Some(used.for_family(family)),
            })?;
    }
    Ok(runenui_core::WorkStartBudget::new(
        remaining[0],
        remaining[1],
        remaining[2],
        remaining[3],
        remaining[4],
    ))
}

fn uncovered_component(
    parent: ProducerBudget,
    child: ProducerBudget,
) -> Option<(runenui_core::BudgetComponent, usize, usize)> {
    use runenui_core::BudgetComponent as B;
    for (component, declared, attempted) in [
        (
            B::QueueFrontier,
            parent.queue_credits(),
            child.queue_credits(),
        ),
        (B::Outputs, parent.outputs(), child.outputs()),
        (
            B::ApplicationOutputs,
            parent.application_outputs(),
            child.application_outputs(),
        ),
        (
            B::MountedNodes,
            parent.mounted_nodes(),
            child.mounted_nodes(),
        ),
        (
            B::RetainedUpdates,
            parent.retained_updates(),
            child.retained_updates(),
        ),
        (
            B::DirtySubscriptionOwners,
            parent.dirty_subscription_owners(),
            child.dirty_subscription_owners(),
        ),
        (
            B::ApplicationSubscriptions,
            parent.application_subscriptions(),
            child.application_subscriptions(),
        ),
        (
            B::MountedSubscriptionsPerOwner,
            parent.mounted_subscriptions_per_owner(),
            child.mounted_subscriptions_per_owner(),
        ),
        (
            B::SubscriptionResidency,
            parent.work_residency().subscriptions(),
            child.work_residency().subscriptions(),
        ),
    ] {
        if attempted > declared {
            return Some((component, declared, attempted));
        }
    }
    for family in [
        runenui_core::WorkFamily::LocalTask,
        runenui_core::WorkFamily::SendTask,
        runenui_core::WorkFamily::Timer,
        runenui_core::WorkFamily::HostRequest,
        runenui_core::WorkFamily::FrameworkService,
    ] {
        for (component, declared, attempted) in [
            (
                B::WorkStarts(family),
                parent.work_starts().for_family(family),
                child.work_starts().for_family(family),
            ),
            (
                B::WorkResidency(family),
                parent.work_residency().for_family(family),
                child.work_residency().for_family(family),
            ),
        ] {
            if attempted > declared {
                return Some((component, declared, attempted));
            }
        }
    }
    None
}

pub(crate) fn validate_application_family_starts<'a, Action: 'a, Protocol: HostProtocol + 'a>(
    promised: runenui_core::WorkStartBudget,
    ledgers: impl Iterator<Item = &'a TransactionLedger<Action, Protocol>>,
    subscriptions: &[Subscription<Action>],
) -> Result<(), crate::RuntimeTerminalReason> {
    let families = [
        runenui_core::WorkFamily::LocalTask,
        runenui_core::WorkFamily::SendTask,
        runenui_core::WorkFamily::Timer,
        runenui_core::WorkFamily::HostRequest,
        runenui_core::WorkFamily::FrameworkService,
    ];
    let mut counts = [0usize; 5];
    for ledger in ledgers {
        let used = ledger
            .work_start_budget()
            .ok_or(crate::RuntimeTerminalReason::Poisoned)?;
        for (index, family) in families.into_iter().enumerate() {
            counts[index] = counts[index]
                .checked_add(used.for_family(family))
                .ok_or(crate::RuntimeTerminalReason::Poisoned)?;
        }
    }
    let check = |counts: &[usize; 5], phase| {
        for (index, family) in families.into_iter().enumerate() {
            if counts[index] > promised.for_family(family) {
                return Err(crate::RuntimeTerminalReason::ProducerContractViolation(
                    runenui_core::ProducerContractViolation {
                        kind: runenui_core::ProducerViolationKind::DeclarationExceeded,
                        phase,
                        component: runenui_core::BudgetComponent::WorkStarts(family),
                        declared: promised.for_family(family),
                        attempted: Some(counts[index]),
                    },
                ));
            }
        }
        Ok(())
    };
    check(&counts, runenui_core::ProducerPhase::ApplicationUpdate)?;
    for source in subscriptions {
        let index = usize::from(matches!(
            source.source,
            runenui_core::__runtime::SubscriptionSource::Send { .. }
        ));
        counts[index] = counts[index]
            .checked_add(1)
            .ok_or(crate::RuntimeTerminalReason::Poisoned)?;
    }
    check(
        &counts,
        runenui_core::ProducerPhase::ApplicationSubscriptions,
    )
}
